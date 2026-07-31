// Package sync forwards meeting knowledge from Recap into the rest of the
// ODW.ai suite: meeting summaries are uploaded to Vault (knowledge base) and
// action items trigger Loop workflows via webhook.
package sync

import (
	"bytes"
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log"
	"mime/multipart"
	"net/http"
	"sort"
	"strings"
)

// Config holds cross-product sync configuration sourced from the environment.
type Config struct {
	// VaultAPIURL is the Vault base URL (default http://localhost:8765).
	VaultAPIURL string
	// LoopAPIURL is the Loop base URL (default http://localhost:3000).
	LoopAPIURL string
	// LoopWebhookTrigger is the Loop webhook trigger id. When empty, Loop
	// triggering is skipped.
	LoopWebhookTrigger string
	// LoopWebhookSecret, when set, is used to HMAC-sign Loop webhook bodies.
	LoopWebhookSecret string
}

// SyncRequest carries the meeting data to be synced to Vault and Loop.
type SyncRequest struct {
	MeetingID      string
	Action         string
	MeetingData    map[string]any
	TranscriptData map[string]any
	SummaryData    map[string]any
}

// VaultUploadResponse mirrors Vault's POST /files/upload response body.
type VaultUploadResponse struct {
	Uploaded int      `json:"uploaded"`
	Failed   []string `json:"failed"`
}

// Forwarder forwards meeting data to Vault and Loop. The HTTP client is
// injectable so tests can point it at httptest servers.
type Forwarder struct {
	client *http.Client
	cfg    Config
}

// NewForwarder creates a Forwarder. A nil client falls back to http.DefaultClient.
func NewForwarder(client *http.Client, cfg Config) *Forwarder {
	if client == nil {
		client = http.DefaultClient
	}
	return &Forwarder{client: client, cfg: cfg}
}

// ForwardToVault builds a Markdown document from the meeting summary and
// uploads it to Vault via multipart POST {VAULT_API_URL}/files/upload (field
// name "files"). It returns an identifier for the uploaded document (the
// filename, since Vault's upload endpoint does not return a file id).
func (f *Forwarder) ForwardToVault(ctx context.Context, meetingTitle, markdownBody string) (string, error) {
	filename := sanitizeFilename(meetingTitle) + ".md"

	var buf bytes.Buffer
	mw := multipart.NewWriter(&buf)
	part, err := mw.CreateFormFile("files", filename)
	if err != nil {
		return "", fmt.Errorf("create form file: %w", err)
	}
	if _, err := io.WriteString(part, markdownBody); err != nil {
		return "", fmt.Errorf("write markdown body: %w", err)
	}
	if err := mw.Close(); err != nil {
		return "", fmt.Errorf("close multipart writer: %w", err)
	}

	url := strings.TrimRight(f.cfg.VaultAPIURL, "/") + "/files/upload"
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, &buf)
	if err != nil {
		return "", fmt.Errorf("build vault request: %w", err)
	}
	req.Header.Set("Content-Type", mw.FormDataContentType())

	resp, err := f.client.Do(req)
	if err != nil {
		return "", fmt.Errorf("vault upload request failed: %w", err)
	}
	defer resp.Body.Close()

	body, _ := io.ReadAll(resp.Body)
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return "", fmt.Errorf("vault upload returned status %d: %s", resp.StatusCode, strings.TrimSpace(string(body)))
	}

	var uploadResp VaultUploadResponse
	if err := json.Unmarshal(body, &uploadResp); err != nil {
		return "", fmt.Errorf("parse vault response: %w", err)
	}
	if uploadResp.Uploaded == 0 {
		return "", fmt.Errorf("vault reported no files uploaded (failed: %v)", uploadResp.Failed)
	}

	return filename, nil
}

// ForwardToLoop JSON-marshals payload and POSTs it to
// {LOOP_API_URL}/webhooks/{trigger_id}. When a shared secret is configured the
// raw body is HMAC-SHA256 signed and sent in the x-loop-signature header as
// "sha256=<hex>". If no trigger id is configured the call is skipped (nil).
func (f *Forwarder) ForwardToLoop(ctx context.Context, payload map[string]any) error {
	if f.cfg.LoopWebhookTrigger == "" {
		log.Println("sync: no LOOP_WEBHOOK_TRIGGER_ID configured, skipping Loop trigger")
		return nil
	}

	body, err := json.Marshal(payload)
	if err != nil {
		return fmt.Errorf("marshal loop payload: %w", err)
	}

	url := strings.TrimRight(f.cfg.LoopAPIURL, "/") + "/webhooks/" + f.cfg.LoopWebhookTrigger
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, url, bytes.NewReader(body))
	if err != nil {
		return fmt.Errorf("build loop request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")

	if f.cfg.LoopWebhookSecret != "" {
		mac := hmac.New(sha256.New, []byte(f.cfg.LoopWebhookSecret))
		mac.Write(body)
		req.Header.Set("x-loop-signature", "sha256="+hex.EncodeToString(mac.Sum(nil)))
	}

	resp, err := f.client.Do(req)
	if err != nil {
		return fmt.Errorf("loop webhook request failed: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		respBody, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("loop webhook returned status %d: %s", resp.StatusCode, strings.TrimSpace(string(respBody)))
	}

	return nil
}

// ProcessSync orchestrates a full sync: it builds a meeting summary Markdown
// document, uploads it to Vault, and — when action items are present —
// triggers a Loop workflow. A failure in one target does not abort the other;
// all encountered errors are joined and returned.
func (f *Forwarder) ProcessSync(ctx context.Context, req SyncRequest) error {
	title := meetingTitle(req)
	markdown := buildSummaryMarkdown(req)

	vaultID, vaultErr := f.ForwardToVault(ctx, title, markdown)
	if vaultErr != nil {
		log.Printf("sync: vault forward failed for meeting %s: %v", req.MeetingID, vaultErr)
	} else {
		log.Printf("sync: meeting %s synced to vault as %q", req.MeetingID, vaultID)
	}

	var loopErr error
	if actionItems := extractActionItems(req.SummaryData); len(actionItems) > 0 {
		loopErr = f.ForwardToLoop(ctx, map[string]any{
			"meeting_id":   req.MeetingID,
			"action_items": actionItems,
		})
		if loopErr != nil {
			log.Printf("sync: loop trigger failed for meeting %s: %v", req.MeetingID, loopErr)
		} else {
			log.Printf("sync: meeting %s action items forwarded to loop", req.MeetingID)
		}
	}

	return errors.Join(vaultErr, loopErr)
}

// buildSummaryMarkdown renders the sync request into a Markdown knowledge doc.
func buildSummaryMarkdown(req SyncRequest) string {
	var b strings.Builder

	b.WriteString("# " + meetingTitle(req) + "\n\n")
	if req.MeetingID != "" {
		fmt.Fprintf(&b, "> Meeting ID: %s\n\n", req.MeetingID)
	}

	if len(req.MeetingData) > 0 {
		b.WriteString("## Meeting Details\n\n")
		writeMapSection(&b, req.MeetingData)
		b.WriteString("\n")
	}

	if summary := stringField(req.SummaryData, "content", "summary", "text"); summary != "" {
		b.WriteString("## Summary\n\n" + summary + "\n\n")
	}

	if items := extractActionItems(req.SummaryData); len(items) > 0 {
		b.WriteString("## Action Items\n\n")
		for _, item := range items {
			b.WriteString("- " + item + "\n")
		}
		b.WriteString("\n")
	}

	if len(req.TranscriptData) > 0 {
		b.WriteString("## Transcript\n\n")
		writeMapSection(&b, req.TranscriptData)
		b.WriteString("\n")
	}

	return b.String()
}

// meetingTitle resolves a human-readable title from the sync request.
func meetingTitle(req SyncRequest) string {
	if t := stringField(req.MeetingData, "title"); t != "" {
		return t
	}
	if t := stringField(req.SummaryData, "title"); t != "" {
		return t
	}
	if req.MeetingID != "" {
		return "Meeting " + req.MeetingID
	}
	return "Untitled Meeting"
}

// extractActionItems pulls action item descriptions out of summary data,
// tolerating both []string and []any (of strings or maps) shapes.
func extractActionItems(m map[string]any) []string {
	if m == nil {
		return nil
	}
	raw, ok := m["action_items"]
	if !ok {
		return nil
	}

	var items []string
	switch v := raw.(type) {
	case []string:
		items = append(items, v...)
	case []any:
		for _, e := range v {
			switch it := e.(type) {
			case string:
				if it != "" {
					items = append(items, it)
				}
			case map[string]any:
				if desc := stringField(it, "description", "text", "title"); desc != "" {
					items = append(items, desc)
				}
			}
		}
	}
	return items
}

// stringField returns the first non-empty string value found under any of keys.
func stringField(m map[string]any, keys ...string) string {
	for _, k := range keys {
		if v, ok := m[k]; ok {
			if s, ok := v.(string); ok && s != "" {
				return s
			}
		}
	}
	return ""
}

// writeMapSection renders a map as a sorted Markdown bullet list. Scalar values
// are written verbatim; complex values are JSON-encoded inline.
func writeMapSection(b *strings.Builder, m map[string]any) {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	sort.Strings(keys)

	for _, k := range keys {
		switch val := m[k].(type) {
		case nil:
			// skip empty values
		case string:
			fmt.Fprintf(b, "- **%s**: %s\n", k, val)
		default:
			if encoded, err := json.Marshal(val); err == nil {
				fmt.Fprintf(b, "- **%s**: %s\n", k, string(encoded))
			}
		}
	}
}

// sanitizeFilename strips path separators and other characters that are unsafe
// in filenames across platforms.
func sanitizeFilename(name string) string {
	replacer := strings.NewReplacer(
		"/", "-", "\\", "-", ":", "-", "*", "-", "?", "-",
		"\"", "-", "<", "-", ">", "-", "|", "-",
	)
	name = strings.TrimSpace(replacer.Replace(name))
	if name == "" {
		name = "meeting"
	}
	return name
}
