package api

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/ondemandworld/recap-team-server/internal/auth"
	odwsync "github.com/ondemandworld/recap-team-server/internal/sync"
)

// SyncRequest represents a sync request from the desktop app
type SyncRequest struct {
	MeetingID      string                 `json:"meeting_id"`
	Action         string                 `json:"action"`
	MeetingData    map[string]interface{} `json:"meeting_data,omitempty"`
	TranscriptData map[string]interface{} `json:"transcript_data,omitempty"`
	SummaryData    map[string]interface{} `json:"summary_data,omitempty"`
}

// SyncResponse represents a sync response
type SyncResponse struct {
	Status    string `json:"status"`
	MeetingID string `json:"meeting_id,omitempty"`
	Message   string `json:"message,omitempty"`
}

func (s *Server) syncHandler(w http.ResponseWriter, r *http.Request) {
	user := auth.GetUserFromContext(r.Context())
	if user == nil {
		respondError(w, http.StatusUnauthorized, "Unauthorized")
		return
	}

	var req SyncRequest
	if err := parseJSON(w, r, &req); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	meetingID := req.MeetingID
	if meetingID == "" {
		meetingID = uuid.New().String()
	}

	// Tenant isolation: when the id references an EXISTING meeting, the caller
	// must own it (same rule as /meetings/*). Without this check any
	// authenticated user could sync against someone else's meeting — writing
	// audit rows and Vault documents attributed to the victim's meeting id.
	if s.meetings != nil {
		if parsed, err := uuid.Parse(meetingID); err == nil {
			if meeting, err := s.meetings.Get(r.Context(), parsed); err == nil {
				if !s.canAccess(user, meeting) {
					respondError(w, http.StatusForbidden, "You do not have access to this meeting")
					return
				}
			}
		}
	}

	// Best-effort audit trail for the sync action. The meeting id is only a
	// UUID when the desktop app supplied one; otherwise resource_id stays nil.
	var auditResourceID *uuid.UUID
	if parsed, err := uuid.Parse(meetingID); err == nil {
		auditResourceID = &parsed
	}
	s.writeAudit(r, user, "meeting.sync", auditResourceID, map[string]string{"action": req.Action})

	// Persist to sync_queue for auditability + retry. The row id is captured
	// so the inline forward below settles it to processed/failed — rows used
	// to stay 'pending' forever. The FK on meeting_id means a client-supplied
	// id that is not an existing meeting row can never be inserted directly,
	// so fall back to a NULL meeting_id (the column is nullable) to still
	// record the sync attempt.
	var queueRowID int64
	if s.db != nil {
		if payload, err := json.Marshal(req); err == nil {
			queueMeetingID := interface{}(meetingID)
			if auditResourceID == nil {
				// Not a UUID at all — insert NULL rather than a guaranteed FK error.
				queueMeetingID = nil
			}
			if err := s.db.DB().QueryRow(
				"INSERT INTO sync_queue (meeting_id, action, payload, status, created_at) VALUES ($1, $2, $3, $4, $5) RETURNING id",
				queueMeetingID, req.Action, string(payload), "pending", time.Now(),
			).Scan(&queueRowID); err != nil {
				// Likely a FK violation (valid UUID but no such meeting row); retry
				// without the reference so the sync is still captured.
				if retryErr := s.db.DB().QueryRow(
					"INSERT INTO sync_queue (meeting_id, action, payload, status, created_at) VALUES (NULL, $1, $2, $3, $4) RETURNING id",
					req.Action, string(payload), "pending", time.Now(),
				).Scan(&queueRowID); retryErr != nil {
					log.Printf("sync: failed to persist sync_queue entry for meeting %s: %v", meetingID, retryErr)
				}
			}
		}
	}

	// settleQueue records the outcome of the inline forward on the queue row
	// (the retry worker picks up 'failed' rows with backoff).
	settleQueue := func(status string, retryCount int, errMsg string) {
		if s.db == nil || queueRowID == 0 {
			return
		}
		if status == "processed" {
			if _, err := s.db.DB().Exec(
				"UPDATE sync_queue SET status = $1, retry_count = $2, processed_at = NOW(), next_attempt_at = NULL WHERE id = $3",
				status, retryCount, queueRowID,
			); err != nil {
				log.Printf("sync: failed to settle sync_queue row %d: %v", queueRowID, err)
			}
			return
		}
		if _, err := s.db.DB().Exec(
			"UPDATE sync_queue SET status = $1, retry_count = $2, last_error = $3 WHERE id = $4",
			status, retryCount, errMsg, queueRowID,
		); err != nil {
			log.Printf("sync: failed to settle sync_queue row %d: %v", queueRowID, err)
		}
	}

	// No forwarder configured — fall back to legacy queue-only behaviour.
	if s.syncForwarder == nil {
		respondJSON(w, http.StatusOK, SyncResponse{
			Status:    "queued",
			MeetingID: meetingID,
			Message:   "Meeting data queued for sync",
		})
		return
	}

	syncReq := odwsync.SyncRequest{
		MeetingID:      meetingID,
		Action:         req.Action,
		MeetingData:    req.MeetingData,
		TranscriptData: req.TranscriptData,
		SummaryData:    req.SummaryData,
	}

	if err := s.syncForwarder.ProcessSync(r.Context(), syncReq); err != nil {
		// One or more targets failed, but we do not 500 the whole request —
		// record the error, leave the queue row for the retry worker, and
		// report a partial sync.
		log.Printf("sync: partial sync for meeting %s: %v", meetingID, err)
		settleQueue("failed", 1, err.Error())
		respondJSON(w, http.StatusOK, SyncResponse{
			Status:    "partial",
			MeetingID: meetingID,
			Message:   fmt.Sprintf("Sync completed with errors: %v", err),
		})
		return
	}

	settleQueue("processed", 0, "")
	respondJSON(w, http.StatusOK, SyncResponse{
		Status:    "synced",
		MeetingID: meetingID,
		Message:   "Meeting data synced to Vault and Loop",
	})
}
