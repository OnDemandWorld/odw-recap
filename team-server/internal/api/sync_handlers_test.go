package api

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/ondemandworld/recap-team-server/internal/auth"
	odwsync "github.com/ondemandworld/recap-team-server/internal/sync"
)

// newSyncTestServer builds a Server with a nil DB (avoiding DB deps, matching
// setupTestServer) and an injected forwarder pointed at the given fake Vault
// and Loop servers.
func newSyncTestServer(vaultURL, loopURL string) *Server {
	s := NewServer(nil, nil, "test-secret")
	s.SetSyncForwarder(odwsync.NewForwarder(http.DefaultClient, odwsync.Config{
		VaultAPIURL:        vaultURL,
		LoopAPIURL:         loopURL,
		LoopWebhookTrigger: "trigger-test",
	}))
	return s
}

func syncAuthToken(t *testing.T) string {
	t.Helper()
	jwtManager := auth.NewJWTManager("test-secret")
	token, err := jwtManager.GenerateToken(userID(), "test@example.com", "member")
	if err != nil {
		t.Fatalf("failed to generate token: %v", err)
	}
	return token
}

func postSync(t *testing.T, handler http.Handler, token string, req SyncRequest) *httptest.ResponseRecorder {
	t.Helper()
	body, err := json.Marshal(req)
	if err != nil {
		t.Fatalf("failed to marshal request: %v", err)
	}
	httpReq := httptest.NewRequest("POST", "/sync", bytes.NewReader(body))
	httpReq.Header.Set("Content-Type", "application/json")
	httpReq.Header.Set("Authorization", "Bearer "+token)
	rr := httptest.NewRecorder()
	handler.ServeHTTP(rr, httpReq)
	return rr
}

func TestSyncHandlerSynced(t *testing.T) {
	vault := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte(`{"uploaded":1,"failed":[]}`))
	}))
	defer vault.Close()

	loop := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	}))
	defer loop.Close()

	s := newSyncTestServer(vault.URL, loop.URL)
	rr := postSync(t, s.Router(), syncAuthToken(t), SyncRequest{
		MeetingID:   "m-1",
		Action:      "full_sync",
		SummaryData: map[string]interface{}{"content": "summary", "action_items": []interface{}{"task a"}},
	})

	if rr.Code != http.StatusOK {
		t.Fatalf("expected status 200, got %d: %s", rr.Code, rr.Body.String())
	}

	var resp SyncResponse
	if err := json.Unmarshal(rr.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if resp.Status != "synced" {
		t.Errorf("expected status 'synced', got %q (message: %s)", resp.Status, resp.Message)
	}
	if resp.MeetingID != "m-1" {
		t.Errorf("expected meeting_id 'm-1', got %q", resp.MeetingID)
	}
}

func TestSyncHandlerPartialOnVaultFailure(t *testing.T) {
	vault := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		http.Error(w, "vault down", http.StatusServiceUnavailable)
	}))
	defer vault.Close()

	loopCalled := false
	loop := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		loopCalled = true
		w.WriteHeader(http.StatusOK)
	}))
	defer loop.Close()

	s := newSyncTestServer(vault.URL, loop.URL)
	rr := postSync(t, s.Router(), syncAuthToken(t), SyncRequest{
		MeetingID:   "m-2",
		SummaryData: map[string]interface{}{"action_items": []interface{}{"task b"}},
	})

	// The request itself must not 500 even though Vault failed.
	if rr.Code != http.StatusOK {
		t.Fatalf("expected status 200 (no 500 on partial failure), got %d: %s", rr.Code, rr.Body.String())
	}

	var resp SyncResponse
	if err := json.Unmarshal(rr.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to decode response: %v", err)
	}
	if resp.Status != "partial" {
		t.Errorf("expected status 'partial', got %q", resp.Status)
	}
	if !loopCalled {
		t.Error("expected Loop to still be attempted when Vault fails")
	}
}

func TestSyncHandlerRequiresAuth(t *testing.T) {
	s := newSyncTestServer("http://localhost:0", "http://localhost:0")
	rr := postSync(t, s.Router(), "invalid-token", SyncRequest{MeetingID: "m-3"})
	if rr.Code != http.StatusUnauthorized {
		t.Errorf("expected status 401 for invalid token, got %d", rr.Code)
	}
}
