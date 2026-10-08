package sync

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"
	"time"

	"github.com/ondemandworld/recap-team-server/internal/db"
)

// TestQueueWorkerRetriesFailedRows is a live-PostgreSQL integration test,
// gated behind RECAP_TEST_DSN (the suite must stay green without a DB):
//
//	RECAP_TEST_DSN='postgres://recap:recap@localhost:5433/recap?sslmode=disable' go test ./internal/sync/
//
// It seeds a 'failed' sync_queue row, runs one worker pass against a healthy
// fake Vault/Loop, and asserts the row settles to processed.
func TestQueueWorkerRetriesFailedRows(t *testing.T) {
	dsn := os.Getenv("RECAP_TEST_DSN")
	if dsn == "" {
		t.Skip("RECAP_TEST_DSN not set — skipping live-DB queue worker test")
	}

	database, err := db.NewDatabase(dsn)
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	defer database.Close()

	vault := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, _ = w.Write([]byte(`{"uploaded":1,"failed":[]}`))
	}))
	defer vault.Close()
	loop := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	}))
	defer loop.Close()

	forward := NewForwarder(http.DefaultClient, Config{
		VaultAPIURL: vault.URL,
		LoopAPIURL:  loop.URL,
	})
	worker := NewQueueWorker(database.DB(), forward, time.Second)

	payload, _ := json.Marshal(map[string]any{
		"meeting_id":   "worker-test-meeting",
		"action":       "full_sync",
		"summary_data": map[string]any{"content": "worker retry", "action_items": []string{"do"}},
	})

	// Seed a failed row that is due for retry immediately.
	var rowID int64
	err = database.DB().QueryRow(
		`INSERT INTO sync_queue (meeting_id, action, payload, status, retry_count, next_attempt_at)
		 VALUES (NULL, 'full_sync', $1, 'failed', 1, NOW() - interval '1 minute') RETURNING id`,
		string(payload),
	).Scan(&rowID)
	if err != nil {
		t.Fatalf("seed row: %v", err)
	}
	t.Cleanup(func() {
		_, _ = database.DB().Exec("DELETE FROM sync_queue WHERE id = $1", rowID)
	})

	worker.processOnce(context.Background())

	var status string
	var processedAt *time.Time
	if err := database.DB().QueryRow(
		"SELECT status, processed_at FROM sync_queue WHERE id = $1", rowID,
	).Scan(&status, &processedAt); err != nil {
		t.Fatalf("read row: %v", err)
	}
	if status != "processed" {
		t.Fatalf("expected status 'processed', got %q", status)
	}
	if processedAt == nil {
		t.Fatal("expected processed_at to be set")
	}
}

// TestQueueWorkerBackoffSchedulesNextAttempt verifies a failing row is NOT
// claimed again before its backoff window elapses, and that retry_count and
// last_error are recorded.
func TestQueueWorkerBackoffSchedulesNextAttempt(t *testing.T) {
	dsn := os.Getenv("RECAP_TEST_DSN")
	if dsn == "" {
		t.Skip("RECAP_TEST_DSN not set — skipping live-DB queue worker test")
	}

	database, err := db.NewDatabase(dsn)
	if err != nil {
		t.Fatalf("connect: %v", err)
	}
	defer database.Close()

	// Vault always fails → the row must stay failed with backoff scheduled.
	vault := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusBadGateway)
	}))
	defer vault.Close()
	loop := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	}))
	defer loop.Close()

	forward := NewForwarder(http.DefaultClient, Config{
		VaultAPIURL: vault.URL,
		LoopAPIURL:  loop.URL,
	})
	worker := NewQueueWorker(database.DB(), forward, time.Second)

	payload, _ := json.Marshal(map[string]any{
		"action":        "full_sync",
		"summary_data":  map[string]any{"content": "backoff test"},
		"meeting_title": "backoff test",
	})

	var rowID int64
	err = database.DB().QueryRow(
		`INSERT INTO sync_queue (meeting_id, action, payload, status, retry_count, next_attempt_at)
		 VALUES (NULL, 'full_sync', $1, 'failed', 0, NOW()) RETURNING id`,
		string(payload),
	).Scan(&rowID)
	if err != nil {
		t.Fatalf("seed row: %v", err)
	}
	t.Cleanup(func() {
		_, _ = database.DB().Exec("DELETE FROM sync_queue WHERE id = $1", rowID)
	})

	worker.processOnce(context.Background())

	var status string
	var retryCount int
	var lastError *string
	var nextAttempt *time.Time
	if err := database.DB().QueryRow(
		"SELECT status, retry_count, last_error, next_attempt_at FROM sync_queue WHERE id = $1", rowID,
	).Scan(&status, &retryCount, &lastError, &nextAttempt); err != nil {
		t.Fatalf("read row: %v", err)
	}
	if status != "failed" {
		t.Fatalf("expected status 'failed', got %q", status)
	}
	if retryCount != 1 {
		t.Fatalf("expected retry_count 1, got %d", retryCount)
	}
	if lastError == nil || *lastError == "" {
		t.Fatal("expected last_error to be recorded")
	}
	if nextAttempt == nil || !nextAttempt.After(time.Now()) {
		t.Fatal("expected next_attempt_at in the future (backoff)")
	}
}
