package sync

import (
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestForwardToVault(t *testing.T) {
	var gotPath, gotFilename, gotContentType, gotBody string

	vault := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.Path
		gotContentType = r.Header.Get("Content-Type")

		if err := r.ParseMultipartForm(1 << 20); err != nil {
			t.Errorf("failed to parse multipart form: %v", err)
		}
		file, header, err := r.FormFile("files")
		if err != nil {
			t.Errorf("missing 'files' part: %v", err)
			http.Error(w, "bad request", http.StatusBadRequest)
			return
		}
		defer file.Close()
		gotFilename = header.Filename
		body, _ := io.ReadAll(file)
		gotBody = string(body)

		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"uploaded":1,"failed":[]}`))
	}))
	defer vault.Close()

	f := NewForwarder(vault.Client(), Config{VaultAPIURL: vault.URL})
	id, err := f.ForwardToVault(context.Background(), "Sprint Planning", "# Sprint Planning\n\nSummary body")
	if err != nil {
		t.Fatalf("ForwardToVault returned error: %v", err)
	}

	if gotPath != "/files/upload" {
		t.Errorf("expected path /files/upload, got %q", gotPath)
	}
	if !strings.HasPrefix(gotContentType, "multipart/form-data") {
		t.Errorf("expected multipart content type, got %q", gotContentType)
	}
	if gotFilename != "Sprint Planning.md" {
		t.Errorf("expected filename 'Sprint Planning.md', got %q", gotFilename)
	}
	if !strings.Contains(gotBody, "Summary body") {
		t.Errorf("expected uploaded body to contain markdown, got %q", gotBody)
	}
	if id != "Sprint Planning.md" {
		t.Errorf("expected id 'Sprint Planning.md', got %q", id)
	}
}

func TestForwardToVaultNon2xx(t *testing.T) {
	vault := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "boom", http.StatusInternalServerError)
	}))
	defer vault.Close()

	f := NewForwarder(vault.Client(), Config{VaultAPIURL: vault.URL})
	if _, err := f.ForwardToVault(context.Background(), "t", "body"); err == nil {
		t.Fatal("expected error for non-2xx vault response, got nil")
	}
}

func TestForwardToLoopWithSignature(t *testing.T) {
	const secret = "s3cr3t"
	var gotPath, gotSignature, gotContentType string
	var gotBody []byte

	loop := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotPath = r.URL.Path
		gotSignature = r.Header.Get("x-loop-signature")
		gotContentType = r.Header.Get("Content-Type")
		gotBody, _ = io.ReadAll(r.Body)
		w.WriteHeader(http.StatusOK)
	}))
	defer loop.Close()

	f := NewForwarder(loop.Client(), Config{
		LoopAPIURL:         loop.URL,
		LoopWebhookTrigger: "trigger-123",
		LoopWebhookSecret:  secret,
	})

	payload := map[string]any{"meeting_id": "m1", "action_items": []string{"do x"}}
	if err := f.ForwardToLoop(context.Background(), payload); err != nil {
		t.Fatalf("ForwardToLoop returned error: %v", err)
	}

	if gotPath != "/webhooks/trigger-123" {
		t.Errorf("expected path /webhooks/trigger-123, got %q", gotPath)
	}
	if gotContentType != "application/json" {
		t.Errorf("expected application/json content type, got %q", gotContentType)
	}

	// Verify the HMAC-SHA256 signature matches the raw body.
	mac := hmac.New(sha256.New, []byte(secret))
	mac.Write(gotBody)
	wantSig := "sha256=" + hex.EncodeToString(mac.Sum(nil))
	if gotSignature != wantSig {
		t.Errorf("signature mismatch: got %q, want %q", gotSignature, wantSig)
	}

	var decoded map[string]any
	if err := json.Unmarshal(gotBody, &decoded); err != nil {
		t.Fatalf("loop body is not valid JSON: %v", err)
	}
	if decoded["meeting_id"] != "m1" {
		t.Errorf("expected meeting_id m1, got %v", decoded["meeting_id"])
	}
}

func TestForwardToLoopSkipsWithoutTrigger(t *testing.T) {
	called := false
	loop := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		called = true
	}))
	defer loop.Close()

	f := NewForwarder(loop.Client(), Config{LoopAPIURL: loop.URL}) // no trigger id
	if err := f.ForwardToLoop(context.Background(), map[string]any{"x": 1}); err != nil {
		t.Fatalf("expected nil error when trigger unset, got %v", err)
	}
	if called {
		t.Error("expected Loop endpoint NOT to be called when trigger id is unset")
	}
}

func TestProcessSyncCallsBothTargets(t *testing.T) {
	vaultCalled, loopCalled := false, false

	vault := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		vaultCalled = true
		_, _ = w.Write([]byte(`{"uploaded":1,"failed":[]}`))
	}))
	defer vault.Close()

	loop := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		loopCalled = true
		w.WriteHeader(http.StatusOK)
	}))
	defer loop.Close()

	f := NewForwarder(http.DefaultClient, Config{
		VaultAPIURL:        vault.URL,
		LoopAPIURL:         loop.URL,
		LoopWebhookTrigger: "trigger-1",
	})

	req := SyncRequest{
		MeetingID:   "m-42",
		MeetingData: map[string]any{"title": "Kickoff"},
		SummaryData: map[string]any{
			"content":      "We agreed to ship V1.0.",
			"action_items": []any{"Draft PRD", map[string]any{"description": "Wire sync"}},
		},
	}

	if err := f.ProcessSync(context.Background(), req); err != nil {
		t.Fatalf("ProcessSync returned error: %v", err)
	}
	if !vaultCalled {
		t.Error("expected Vault to be called")
	}
	if !loopCalled {
		t.Error("expected Loop to be called when action items present")
	}
}

func TestProcessSyncVaultFailureStillAttemptsLoop(t *testing.T) {
	loopCalled := false

	vault := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "vault down", http.StatusServiceUnavailable)
	}))
	defer vault.Close()

	loop := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		loopCalled = true
		w.WriteHeader(http.StatusOK)
	}))
	defer loop.Close()

	f := NewForwarder(http.DefaultClient, Config{
		VaultAPIURL:        vault.URL,
		LoopAPIURL:         loop.URL,
		LoopWebhookTrigger: "trigger-1",
	})

	req := SyncRequest{
		MeetingID:   "m-99",
		SummaryData: map[string]any{"action_items": []string{"follow up"}},
	}

	err := f.ProcessSync(context.Background(), req)
	if err == nil {
		t.Fatal("expected error when Vault fails, got nil")
	}
	if !strings.Contains(err.Error(), "vault") {
		t.Errorf("expected error to mention vault, got %q", err.Error())
	}
	if !loopCalled {
		t.Error("expected Loop to still be attempted after Vault failure")
	}
}
