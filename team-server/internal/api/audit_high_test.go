package api

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/google/uuid"
)

// Regression for audit #2: createMeetingHandler must honor a client-supplied
// meeting id with owner-scoped upsert semantics so the desktop sync client's
// POST /meetings/ + /sync flow links data to a real row instead of creating a
// duplicate empty meeting on every sync.
func TestCreateMeeting_HonorsClientID(t *testing.T) {
	s, store, _ := newOwnershipTestServer()
	router := s.Router()
	tokenA := tokenFor(t, userA, "a@example.com", "member")
	tokenB := tokenFor(t, userB, "b@example.com", "member")

	clientID := uuid.MustParse("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")
	body := map[string]interface{}{
		"id":           clientID.String(),
		"title":        "Synced meeting",
		"meeting_type": "standup",
		"language":     "en",
	}

	// First create with the client id succeeds and preserves that id.
	rr := doRequest(t, router, "POST", "/meetings/", tokenA, body)
	if rr.Code != http.StatusCreated {
		t.Fatalf("first create: expected 201, got %d: %s", rr.Code, rr.Body.String())
	}
	var resp struct {
		ID uuid.UUID `json:"id"`
	}
	if err := json.Unmarshal(rr.Body.Bytes(), &resp); err != nil {
		t.Fatalf("decode: %v", err)
	}
	if resp.ID != clientID {
		t.Fatalf("expected server to honor client id %s, got %s", clientID, resp.ID)
	}
	if _, err := store.Get(context.Background(), clientID); err != nil {
		t.Fatalf("meeting %s not persisted: %v", clientID, err)
	}

	// Re-syncing the same id by the owner is an idempotent 409 (no duplicate).
	if rr := doRequest(t, router, "POST", "/meetings/", tokenA, body); rr.Code != http.StatusConflict {
		t.Errorf("re-create by owner: expected 409, got %d", rr.Code)
	}
	if got := len(store.meetings); got != 1 {
		t.Errorf("expected exactly 1 meeting after re-sync, got %d", got)
	}

	// Another tenant must not be able to claim or overwrite that id.
	if rr := doRequest(t, router, "POST", "/meetings/", tokenB, body); rr.Code != http.StatusForbidden {
		t.Errorf("create existing id by non-owner: expected 403, got %d", rr.Code)
	}
	if got := len(store.meetings); got != 1 {
		t.Errorf("non-owner attempt must not add a meeting, got %d", got)
	}
}

// A create with no client id still generates a fresh UUID (unchanged behavior).
func TestCreateMeeting_NoIDGeneratesUUID(t *testing.T) {
	s, _, _ := newOwnershipTestServer()
	router := s.Router()
	tokenA := tokenFor(t, userA, "a@example.com", "member")

	id := createMeeting(t, router, tokenA, "No id supplied")
	if id == uuid.Nil {
		t.Fatal("expected a generated UUID when no id is supplied")
	}
}

// Regression for audit #4: a spoofed X-Forwarded-For / X-Real-IP /
// True-Client-IP header must NOT change the client address used for audit
// attribution (and, by the same r.RemoteAddr, the login rate-limit key).
// middleware.RealIP was removed; the connection peer is authoritative.
func TestRealIP_SpoofedHeadersIgnored(t *testing.T) {
	s, _, audit := newOwnershipTestServer()
	router := s.Router()
	tokenA := tokenFor(t, userA, "a@example.com", "member")

	raw, _ := json.Marshal(map[string]interface{}{"title": "audit ip"})
	req := httptest.NewRequest("POST", "/meetings/", bytes.NewReader(raw))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+tokenA)
	req.Header.Set("X-Forwarded-For", "9.9.9.9")
	req.Header.Set("X-Real-IP", "8.8.8.8")
	req.Header.Set("True-Client-IP", "7.7.7.7")
	// httptest.NewRequest sets RemoteAddr to the connection peer 192.0.2.1:1234.
	rr := httptest.NewRecorder()
	router.ServeHTTP(rr, req)
	if rr.Code != http.StatusCreated {
		t.Fatalf("create: expected 201, got %d: %s", rr.Code, rr.Body.String())
	}

	entries := audit.snapshot()
	if len(entries) == 0 {
		t.Fatal("expected an audit entry")
	}
	ip := entries[0].IPAddress
	if ip == nil {
		t.Fatal("expected audit ip_address to be set")
	}
	if !strings.HasPrefix(*ip, "192.0.2.1") {
		t.Errorf("audit ip should be the connection peer 192.0.2.1, got %q", *ip)
	}
	for _, spoofed := range []string{"9.9.9.9", "8.8.8.8", "7.7.7.7"} {
		if strings.Contains(*ip, spoofed) {
			t.Errorf("audit ip honored spoofed header %q: %q", spoofed, *ip)
		}
	}
}
