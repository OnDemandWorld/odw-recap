package sync

import (
	"context"
	"database/sql"
	"encoding/json"
	"errors"
	"log"
	"time"
)

// Queue worker: the /sync handler forwards inline for immediate feedback and
// records the attempt in sync_queue. This worker is the durability path —
// rows that failed (or were stranded mid-request by a crash) are retried with
// bounded exponential backoff instead of silently losing the cross-product
// sync the first time Vault or Loop blips.

const (
	// maxQueueRetries bounds how often a failed row is re-attempted.
	maxQueueRetries = 5
	// baseRetryDelay is the backoff base: attempt N waits base * 2^(N-1).
	baseRetryDelay = 30 * time.Second
	// stalePendingAfter: rows still 'pending' after this long were stranded
	// by a crashed/restarted process (the handler settles rows synchronously).
	stalePendingAfter = 10 * time.Minute
	// claimBatchSize caps rows claimed per poll.
	claimBatchSize = 10
)

// queueRow is one claimed sync_queue record.
type queueRow struct {
	id         int64
	payload    string
	retryCount int
}

// QueueWorker retries failed/stranded sync_queue rows through the Forwarder.
type QueueWorker struct {
	db       *sql.DB
	forward  *Forwarder
	interval time.Duration
}

// NewQueueWorker builds a worker. interval <= 0 defaults to 30s.
func NewQueueWorker(db *sql.DB, forward *Forwarder, interval time.Duration) *QueueWorker {
	if interval <= 0 {
		interval = 30 * time.Second
	}
	return &QueueWorker{db: db, forward: forward, interval: interval}
}

// Start runs the worker until ctx is cancelled. It returns immediately; the
// loop owns one goroutine and logs its exit.
func (w *QueueWorker) Start(ctx context.Context) {
	go func() {
		ticker := time.NewTicker(w.interval)
		defer ticker.Stop()
		for {
			select {
			case <-ctx.Done():
				log.Println("sync queue worker stopped")
				return
			case <-ticker.C:
				w.processOnce(ctx)
			}
		}
	}()
	log.Printf("sync queue worker started (interval=%s, max_retries=%d)", w.interval, maxQueueRetries)
}

// processOnce claims up to claimBatchSize due rows (FOR UPDATE SKIP LOCKED so
// multiple server replicas never double-process) and forwards each.
func (w *QueueWorker) processOnce(ctx context.Context) {
	rows, err := w.claim(ctx)
	if err != nil {
		log.Printf("sync queue worker: claim failed: %v", err)
		return
	}
	for _, row := range rows {
		w.forwardRow(ctx, row)
	}
}

func (w *QueueWorker) claim(ctx context.Context) ([]queueRow, error) {
	tx, err := w.db.BeginTx(ctx, nil)
	if err != nil {
		return nil, err
	}
	defer func() { _ = tx.Rollback() }()

	query := `
		SELECT id, payload::text, retry_count
		FROM sync_queue
		WHERE next_attempt_at <= NOW()
		  AND (
			(status = 'failed' AND retry_count < $1)
		 OR (status = 'pending' AND created_at < NOW() - $2::interval)
		  )
		ORDER BY created_at ASC
		LIMIT $3
		FOR UPDATE SKIP LOCKED`
	rows, err := tx.QueryContext(ctx, query, maxQueueRetries, stalePendingAfter.String(), claimBatchSize)
	if err != nil {
		return nil, err
	}
	var claimed []queueRow
	for rows.Next() {
		var r queueRow
		if err := rows.Scan(&r.id, &r.payload, &r.retryCount); err != nil {
			rows.Close()
			return nil, err
		}
		claimed = append(claimed, r)
	}
	rows.Close()
	if err := rows.Err(); err != nil {
		return nil, err
	}
	if err := tx.Commit(); err != nil {
		return nil, err
	}
	return claimed, nil
}

func (w *QueueWorker) forwardRow(ctx context.Context, row queueRow) {
	var payload struct {
		MeetingID      string         `json:"meeting_id"`
		Action         string         `json:"action"`
		MeetingData    map[string]any `json:"meeting_data"`
		TranscriptData map[string]any `json:"transcript_data"`
		SummaryData    map[string]any `json:"summary_data"`
	}
	if err := json.Unmarshal([]byte(row.payload), &payload); err != nil {
		// Undecodable payload can never succeed — settle as failed permanently.
		log.Printf("sync queue worker: row %d has undecodable payload: %v", row.id, err)
		w.settle(ctx, row.id, "failed", row.retryCount, "undecodable payload: "+err.Error())
		return
	}

	req := SyncRequest{
		MeetingID:      payload.MeetingID,
		Action:         payload.Action,
		MeetingData:    payload.MeetingData,
		TranscriptData: payload.TranscriptData,
		SummaryData:    payload.SummaryData,
	}
	if err := w.forward.ProcessSync(ctx, req); err != nil {
		retries := row.retryCount + 1
		if retries >= maxQueueRetries {
			log.Printf("sync queue worker: row %d exhausted %d retries: %v", row.id, retries, err)
			w.settle(ctx, row.id, "failed", retries, err.Error())
			return
		}
		w.settle(ctx, row.id, "failed", retries, err.Error())
		log.Printf("sync queue worker: row %d failed (retry %d/%d): %v", row.id, retries, maxQueueRetries, err)
		return
	}
	w.settle(ctx, row.id, "processed", row.retryCount, "")
	log.Printf("sync queue worker: row %d processed", row.id)
}

// settle writes the terminal/retry state for a row. Errors are logged only —
// the next poll re-claims the row if the write itself failed.
func (w *QueueWorker) settle(ctx context.Context, id int64, status string, retryCount int, errMsg string) {
	var err error
	if status == "processed" {
		_, err = w.db.ExecContext(ctx,
			"UPDATE sync_queue SET status = $1, retry_count = $2, processed_at = NOW(), next_attempt_at = NULL WHERE id = $3",
			status, retryCount, id)
	} else {
		// Exponential backoff: attempt N waits base * 2^(N-1).
		delay := baseRetryDelay << (retryCount - 1)
		_, err = w.db.ExecContext(ctx,
			"UPDATE sync_queue SET status = $1, retry_count = $2, last_error = $3, next_attempt_at = NOW() + $4::interval WHERE id = $5",
			status, retryCount, truncateErr(errMsg), delay.String(), id)
	}
	if err != nil {
		log.Printf("sync queue worker: failed to settle row %d: %v", id, err)
	}
}

func truncateErr(msg string) string {
	if len(msg) > 500 {
		return msg[:500]
	}
	return msg
}

// ErrNoForwarder is returned by helpers when the worker was built without a
// forwarder — kept as a sentinel for tests.
var ErrNoForwarder = errors.New("sync queue worker: no forwarder configured")
