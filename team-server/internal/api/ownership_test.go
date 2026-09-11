package api

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"sync"
	"testing"

	"github.com/google/uuid"
	"github.com/ondemandworld/recap-team-server/internal/auth"
	"github.com/ondemandworld/recap-team-server/internal/models"
)

// fakeMeetingStore is an in-memory MeetingStore used to exercise the
// tenant-ownership rules without a database.
type fakeMeetingStore struct {
	mu       sync.Mutex
	meetings map[uuid.UUID]*models.Meeting
}

func newFakeMeetingStore() *fakeMeetingStore {
	return &fakeMeetingStore{meetings: make(map[uuid.UUID]*models.Meeting)}
}

func (f *fakeMeetingStore) Create(_ context.Context, m *models.Meeting) error {
	f.mu.Lock()
	defer f.mu.Unlock()
	cp := *m
	f.meetings[m.ID] = &cp
	return nil
}

func (f *fakeMeetingStore) Get(_ context.Context, id uuid.UUID) (*models.Meeting, error) {
	f.mu.Lock()
	defer f.mu.Unlock()
	m, ok := f.meetings[id]
	if !ok {
		return nil, ErrMeetingNotFound
	}
	cp := *m
	return &cp, nil
}

func (f *fakeMeetingStore) ListByOwner(_ context.Context, ownerID uuid.UUID, limit, offset int) ([]*models.Meeting, error) {
	f.mu.Lock()
	defer f.mu.Unlock()
	owned := make([]*models.Meeting, 0)
	for _, m := range f.meetings {
		if m.CreatedBy != nil && *m.CreatedBy == ownerID {
			cp := *m
			owned = append(owned, &cp)
		}
	}
	// Newest first, matching the PostgreSQL store's ORDER BY created_at DESC.
	for i := 1; i < len(owned); i++ {
		for j := i; j > 0 && owned[j].CreatedAt.After(owned[j-1].CreatedAt); j-- {
			owned[j], owned[j-1] = owned[j-1], owned[j]
		}
	}
	if offset > len(owned) {
		offset = len(owned)
	}
	owned = owned[offset:]
	if limit < len(owned) {
		owned = owned[:limit]
	}
	return owned, nil
}

func (f *fakeMeetingStore) Update(_ context.Context, m *models.Meeting) error {
	f.mu.Lock()
	defer f.mu.Unlock()
	if _, ok := f.meetings[m.ID]; !ok {
		return ErrMeetingNotFound
	}
	cp := *m
	f.meetings[m.ID] = &cp
	return nil
}

func (f *fakeMeetingStore) Delete(_ context.Context, id uuid.UUID) error {
	f.mu.Lock()
	defer f.mu.Unlock()
	delete(f.meetings, id)
	return nil
}

// fakeAuditLogger records audit entries in memory for assertions.
type fakeAuditLogger struct {
	mu      sync.Mutex
	entries []*models.AuditLog
}

func (f *fakeAuditLogger) Log(_ context.Context, e *models.AuditLog) {
	f.mu.Lock()
	defer f.mu.Unlock()
	f.entries = append(f.entries, e)
}

func (f *fakeAuditLogger) snapshot() []*models.AuditLog {
	f.mu.Lock()
	defer f.mu.Unlock()
	out := make([]*models.AuditLog, len(f.entries))
	copy(out, f.entries)
	return out
}

// newOwnershipTestServer builds a DB-less Server backed by in-memory fakes.
func newOwnershipTestServer() (*Server, *fakeMeetingStore, *fakeAuditLogger) {
	s := NewServer(nil, nil, "test-secret")
	store := newFakeMeetingStore()
	audit := &fakeAuditLogger{}
	s.SetMeetingStore(store)
	s.SetAuditLogger(audit)
	return s, store, audit
}

func tokenFor(t *testing.T, id uuid.UUID, email, role string) string {
	t.Helper()
	jwtManager := auth.NewJWTManager("test-secret")
	token, err := jwtManager.GenerateToken(id, email, role)
	if err != nil {
		t.Fatalf("failed to generate token: %v", err)
	}
	return token
}

func doRequest(t *testing.T, handler http.Handler, method, path, token string, body interface{}) *httptest.ResponseRecorder {
	t.Helper()
	var reader *bytes.Reader
	if body != nil {
		raw, err := json.Marshal(body)
		if err != nil {
			t.Fatalf("failed to marshal body: %v", err)
		}
		reader = bytes.NewReader(raw)
	} else {
		reader = bytes.NewReader(nil)
	}
	req := httptest.NewRequest(method, path, reader)
	req.Header.Set("Content-Type", "application/json")
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	rr := httptest.NewRecorder()
	handler.ServeHTTP(rr, req)
	return rr
}

// createMeeting creates a meeting via the HTTP API and returns its id.
func createMeeting(t *testing.T, handler http.Handler, token, title string) uuid.UUID {
	t.Helper()
	rr := doRequest(t, handler, "POST", "/meetings/", token, map[string]interface{}{
		"title":        title,
		"meeting_type": "standup",
		"language":     "en",
	})
	if rr.Code != http.StatusCreated {
		t.Fatalf("expected 201 creating meeting, got %d: %s", rr.Code, rr.Body.String())
	}
	var resp struct {
		ID uuid.UUID `json:"id"`
	}
	if err := json.Unmarshal(rr.Body.Bytes(), &resp); err != nil {
		t.Fatalf("failed to decode create response: %v", err)
	}
	return resp.ID
}

var (
	userA = uuid.MustParse("11111111-1111-1111-1111-111111111111")
	userB = uuid.MustParse("22222222-2222-2222-2222-222222222222")
)

func TestTenantIsolation_NonOwnerDenied(t *testing.T) {
	s, _, _ := newOwnershipTestServer()
	router := s.Router()

	tokenA := tokenFor(t, userA, "a@example.com", "member")
	tokenB := tokenFor(t, userB, "b@example.com", "member")

	meetingID := createMeeting(t, router, tokenA, "A private meeting")

	// User B must not read, update, or delete user A's meeting.
	if rr := doRequest(t, router, "GET", "/meetings/"+meetingID.String(), tokenB, nil); rr.Code != http.StatusForbidden {
		t.Errorf("GET by non-owner: expected 403, got %d", rr.Code)
	}
	if rr := doRequest(t, router, "PUT", "/meetings/"+meetingID.String(), tokenB, map[string]interface{}{"title": "hijacked"}); rr.Code != http.StatusForbidden {
		t.Errorf("PUT by non-owner: expected 403, got %d", rr.Code)
	}
	if rr := doRequest(t, router, "DELETE", "/meetings/"+meetingID.String(), tokenB, nil); rr.Code != http.StatusForbidden {
		t.Errorf("DELETE by non-owner: expected 403, got %d", rr.Code)
	}

	// The meeting must still exist and be readable by its owner.
	if rr := doRequest(t, router, "GET", "/meetings/"+meetingID.String(), tokenA, nil); rr.Code != http.StatusOK {
		t.Errorf("GET by owner: expected 200, got %d", rr.Code)
	}
}

func TestTenantIsolation_OwnerCanCRUD(t *testing.T) {
	s, _, _ := newOwnershipTestServer()
	router := s.Router()

	tokenA := tokenFor(t, userA, "a@example.com", "member")
	meetingID := createMeeting(t, router, tokenA, "My meeting")

	if rr := doRequest(t, router, "GET", "/meetings/"+meetingID.String(), tokenA, nil); rr.Code != http.StatusOK {
		t.Errorf("owner GET: expected 200, got %d", rr.Code)
	}
	if rr := doRequest(t, router, "PUT", "/meetings/"+meetingID.String(), tokenA, map[string]interface{}{"title": "renamed"}); rr.Code != http.StatusOK {
		t.Errorf("owner PUT: expected 200, got %d", rr.Code)
	}
	if rr := doRequest(t, router, "DELETE", "/meetings/"+meetingID.String(), tokenA, nil); rr.Code != http.StatusOK {
		t.Errorf("owner DELETE: expected 200, got %d", rr.Code)
	}
	// After deletion the meeting is gone.
	if rr := doRequest(t, router, "GET", "/meetings/"+meetingID.String(), tokenA, nil); rr.Code != http.StatusNotFound {
		t.Errorf("GET after delete: expected 404, got %d", rr.Code)
	}
}

func TestTenantIsolation_ListScopedToOwner(t *testing.T) {
	s, _, _ := newOwnershipTestServer()
	router := s.Router()

	tokenA := tokenFor(t, userA, "a@example.com", "member")
	tokenB := tokenFor(t, userB, "b@example.com", "member")

	idA := createMeeting(t, router, tokenA, "A meeting")
	createMeeting(t, router, tokenB, "B meeting")

	listIDs := func(token string) []uuid.UUID {
		rr := doRequest(t, router, "GET", "/meetings/", token, nil)
		if rr.Code != http.StatusOK {
			t.Fatalf("list: expected 200, got %d", rr.Code)
		}
		var resp struct {
			Meetings []struct {
				ID uuid.UUID `json:"id"`
			} `json:"meetings"`
		}
		if err := json.Unmarshal(rr.Body.Bytes(), &resp); err != nil {
			t.Fatalf("failed to decode list response: %v", err)
		}
		ids := make([]uuid.UUID, 0, len(resp.Meetings))
		for _, m := range resp.Meetings {
			ids = append(ids, m.ID)
		}
		return ids
	}

	aIDs := listIDs(tokenA)
	if len(aIDs) != 1 || aIDs[0] != idA {
		t.Errorf("user A list: expected exactly [%s], got %v", idA, aIDs)
	}

	bIDs := listIDs(tokenB)
	if len(bIDs) != 1 || bIDs[0] == idA {
		t.Errorf("user B list: must not contain user A's meeting, got %v", bIDs)
	}
}

func TestTenantIsolation_AdminCanAccessOthersMeeting(t *testing.T) {
	s, _, _ := newOwnershipTestServer()
	router := s.Router()

	tokenA := tokenFor(t, userA, "a@example.com", "member")
	adminID := uuid.MustParse("33333333-3333-3333-3333-333333333333")
	tokenAdmin := tokenFor(t, adminID, "admin@example.com", "admin")

	meetingID := createMeeting(t, router, tokenA, "A meeting")

	if rr := doRequest(t, router, "GET", "/meetings/"+meetingID.String(), tokenAdmin, nil); rr.Code != http.StatusOK {
		t.Errorf("admin GET on other's meeting: expected 200, got %d", rr.Code)
	}
}

func TestGetMeeting_NotFound(t *testing.T) {
	s, _, _ := newOwnershipTestServer()
	router := s.Router()
	tokenA := tokenFor(t, userA, "a@example.com", "member")

	missing := uuid.New()
	if rr := doRequest(t, router, "GET", "/meetings/"+missing.String(), tokenA, nil); rr.Code != http.StatusNotFound {
		t.Errorf("expected 404 for missing meeting, got %d", rr.Code)
	}
}

func TestAuditLog_WrittenOnCreateAndDelete(t *testing.T) {
	s, _, audit := newOwnershipTestServer()
	router := s.Router()
	tokenA := tokenFor(t, userA, "a@example.com", "member")

	meetingID := createMeeting(t, router, tokenA, "Audited meeting")
	if rr := doRequest(t, router, "DELETE", "/meetings/"+meetingID.String(), tokenA, nil); rr.Code != http.StatusOK {
		t.Fatalf("delete: expected 200, got %d", rr.Code)
	}

	entries := audit.snapshot()

	hasAction := func(action string) bool {
		for _, e := range entries {
			if e.Action != action {
				continue
			}
			if e.UserID == nil || *e.UserID != userA {
				t.Errorf("audit %s: expected actor %s, got %v", action, userA, e.UserID)
			}
			if e.ResourceType == nil || *e.ResourceType != "meeting" {
				t.Errorf("audit %s: expected resource_type 'meeting', got %v", action, e.ResourceType)
			}
			if e.ResourceID == nil || *e.ResourceID != meetingID {
				t.Errorf("audit %s: expected resource_id %s, got %v", action, meetingID, e.ResourceID)
			}
			return true
		}
		return false
	}

	if !hasAction("meeting.create") {
		t.Errorf("expected a 'meeting.create' audit entry, got %+v", entries)
	}
	if !hasAction("meeting.delete") {
		t.Errorf("expected a 'meeting.delete' audit entry, got %+v", entries)
	}
}

func TestAuditLog_WrittenOnSync(t *testing.T) {
	s, _, audit := newOwnershipTestServer()
	router := s.Router()
	tokenA := tokenFor(t, userA, "a@example.com", "member")

	meetingID := uuid.New()
	rr := doRequest(t, router, "POST", "/sync", tokenA, SyncRequest{
		MeetingID:   meetingID.String(),
		Action:      "full_sync",
		SummaryData: map[string]interface{}{"content": "summary"},
	})
	if rr.Code != http.StatusOK {
		t.Fatalf("sync: expected 200, got %d: %s", rr.Code, rr.Body.String())
	}

	found := false
	for _, e := range audit.snapshot() {
		if e.Action == "meeting.sync" {
			found = true
			if e.ResourceID == nil || *e.ResourceID != meetingID {
				t.Errorf("sync audit: expected resource_id %s, got %v", meetingID, e.ResourceID)
			}
		}
	}
	if !found {
		t.Error("expected a 'meeting.sync' audit entry")
	}
}
