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
	if err := parseJSON(r, &req); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	meetingID := req.MeetingID
	if meetingID == "" {
		meetingID = uuid.New().String()
	}

	// Best-effort audit trail for the sync action. The meeting id is only a
	// UUID when the desktop app supplied one; otherwise resource_id stays nil.
	var auditResourceID *uuid.UUID
	if parsed, err := uuid.Parse(meetingID); err == nil {
		auditResourceID = &parsed
	}
	s.writeAudit(r, user, "meeting.sync", auditResourceID)

	// Persist to sync_queue for auditability. Best-effort: a database failure
	// here must not block the actual cross-product sync below.
	if s.db != nil {
		if payload, err := json.Marshal(req); err == nil {
			if _, err := s.db.DB().Exec(
				"INSERT INTO sync_queue (meeting_id, action, payload, status, created_at) VALUES ($1, $2, $3, $4, $5)",
				meetingID, req.Action, string(payload), "pending", time.Now(),
			); err != nil {
				log.Printf("sync: failed to persist sync_queue entry for meeting %s: %v", meetingID, err)
			}
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
		// record the error and report a partial sync.
		log.Printf("sync: partial sync for meeting %s: %v", meetingID, err)
		respondJSON(w, http.StatusOK, SyncResponse{
			Status:    "partial",
			MeetingID: meetingID,
			Message:   fmt.Sprintf("Sync completed with errors: %v", err),
		})
		return
	}

	respondJSON(w, http.StatusOK, SyncResponse{
		Status:    "synced",
		MeetingID: meetingID,
		Message:   "Meeting data synced to Vault and Loop",
	})
}
