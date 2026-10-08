package api

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/google/uuid"
	"github.com/ondemandworld/recap-team-server/internal/auth"
	"github.com/ondemandworld/recap-team-server/internal/models"
	odwsync "github.com/ondemandworld/recap-team-server/internal/sync"
)

func strPtr(s string) *string { return &s }

// TestSyncHandlerTenantIsolation verifies /sync honours meeting ownership:
// a member may sync their OWN meeting (or a not-yet-created id) but gets 403
// when targeting another user's meeting — previously any authenticated user
// could sync against any meeting id, polluting audit trails and Vault.
func TestSyncHandlerTenantIsolation(t *testing.T) {
	vault := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte(`{"uploaded":1,"failed":[]}`))
	}))
	defer vault.Close()

	loop := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	}))
	defer loop.Close()

	attackerID := uuid.New()
	victimID := uuid.New()

	// The server has a meeting store holding ONE meeting owned by the victim.
	s := NewServer(nil, nil, "test-secret")
	store := newFakeMeetingStore()
	victimMeeting := &models.Meeting{ID: uuid.New(), Title: strPtr("victim"), CreatedBy: &victimID}
	if err := store.Create(context.Background(), victimMeeting); err != nil {
		t.Fatalf("seed meeting: %v", err)
	}
	s.SetMeetingStore(store)
	s.SetSyncForwarder(odwsync.NewForwarder(http.DefaultClient, odwsync.Config{
		VaultAPIURL: vault.URL,
		LoopAPIURL:  loop.URL,
	}))

	jwtManager := auth.NewJWTManager("test-secret")
	attackerToken, err := jwtManager.GenerateToken(attackerID, "attacker@example.com", "member")
	if err != nil {
		t.Fatalf("token: %v", err)
	}

	// 1. Someone else's meeting → 403.
	rr := postSync(t, s.Router(), attackerToken, SyncRequest{
		MeetingID:   victimMeeting.ID.String(),
		Action:      "full_sync",
		SummaryData: map[string]interface{}{"content": "hijack attempt"},
	})
	if rr.Code != http.StatusForbidden {
		t.Errorf("syncing another user's meeting: expected 403, got %d: %s", rr.Code, rr.Body.String())
	}

	// 2. A fresh (nonexistent) meeting id → allowed (create-then-sync flow).
	rr = postSync(t, s.Router(), attackerToken, SyncRequest{
		MeetingID:   uuid.New().String(),
		Action:      "full_sync",
		SummaryData: map[string]interface{}{"content": "own content"},
	})
	if rr.Code != http.StatusOK {
		t.Errorf("syncing a new meeting id: expected 200, got %d: %s", rr.Code, rr.Body.String())
	}

	// 3. The victim syncing their own meeting → allowed.
	victimToken, err := jwtManager.GenerateToken(victimID, "victim@example.com", "member")
	if err != nil {
		t.Fatalf("token: %v", err)
	}
	rr = postSync(t, s.Router(), victimToken, SyncRequest{
		MeetingID:   victimMeeting.ID.String(),
		Action:      "full_sync",
		SummaryData: map[string]interface{}{"content": "legit"},
	})
	if rr.Code != http.StatusOK {
		t.Errorf("syncing own meeting: expected 200, got %d: %s", rr.Code, rr.Body.String())
	}
}
