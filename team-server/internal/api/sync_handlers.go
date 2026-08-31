package api

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/ondemandworld/recap-team-server/internal/auth"
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

// validSyncActions enumerates the accepted sync actions.
var validSyncActions = map[string]bool{
	"create": true,
	"update": true,
	"delete": true,
	"upsert": true,
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

	if !validSyncActions[req.Action] {
		respondError(w, http.StatusBadRequest, "Invalid action: must be one of create, update, delete, upsert")
		return
	}

	meetingID := req.MeetingID
	if meetingID == "" {
		meetingID = uuid.New().String()
	}
	if _, err := uuid.Parse(meetingID); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid meeting_id")
		return
	}

	// sync_queue.meeting_id references meetings(id), so ensure the meeting
	// exists before queueing. This makes the endpoint idempotent for new
	// meetings created on the desktop.
	_, err := s.db.DB().Exec(
		`INSERT INTO meetings (id, created_by, status, created_at, updated_at)
		 VALUES ($1, $2, 'pending', $3, $3)
		 ON CONFLICT (id) DO NOTHING`,
		meetingID, user.ID, time.Now(),
	)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to prepare sync")
		return
	}

	// Queue sync item for processing
	payload, err := json.Marshal(req)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to marshal sync payload")
		return
	}

	_, err = s.db.DB().Exec(
		"INSERT INTO sync_queue (meeting_id, action, payload, status, created_at) VALUES ($1, $2, $3, $4, $5)",
		meetingID, req.Action, string(payload), "pending", time.Now(),
	)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to queue sync")
		return
	}

	respondJSON(w, http.StatusOK, SyncResponse{
		Status:    "queued",
		MeetingID: meetingID,
		Message:   "Meeting data queued for sync",
	})
}
