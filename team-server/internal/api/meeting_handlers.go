package api

import (
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
	"github.com/ondemandworld/recap-team-server/internal/auth"
	"github.com/ondemandworld/recap-team-server/internal/models"
)

func (s *Server) listMeetingsHandler(w http.ResponseWriter, r *http.Request) {
	rows, err := s.db.DB().Query("SELECT id, title, meeting_type, language, status, created_at, updated_at FROM meetings ORDER BY created_at DESC LIMIT 100")
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to list meetings")
		return
	}
	defer rows.Close()

	var meetings []map[string]interface{}
	for rows.Next() {
		var id uuid.UUID
		var title, meetingType, language, status string
		var createdAt, updatedAt time.Time

		if err := rows.Scan(&id, &title, &meetingType, &language, &status, &createdAt, &updatedAt); err != nil {
			continue
		}

		meetings = append(meetings, map[string]interface{}{
			"id":           id,
			"title":        title,
			"meeting_type": meetingType,
			"language":     language,
			"status":       status,
			"created_at":   createdAt,
			"updated_at":   updatedAt,
		})
	}

	respondJSON(w, http.StatusOK, map[string]interface{}{"meetings": meetings})
}

func (s *Server) createMeetingHandler(w http.ResponseWriter, r *http.Request) {
	var req models.Meeting
	if err := parseJSON(r, &req); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	user := auth.GetUserFromContext(r.Context())
	meetingID := uuid.New()
	status := "pending"

	_, err := s.db.DB().Exec(
		"INSERT INTO meetings (id, created_by, title, meeting_type, language, status, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
		meetingID, user.ID, req.Title, req.MeetingType, req.Language, status, time.Now(), time.Now(),
	)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to create meeting")
		return
	}

	respondJSON(w, http.StatusCreated, map[string]interface{}{"id": meetingID, "status": status})
}

func (s *Server) getMeetingHandler(w http.ResponseWriter, r *http.Request) {
	meetingID := chi.URLParam(r, "meetingID")

	var meeting models.Meeting
	var createdBy uuid.UUID
	var createdAt, updatedAt time.Time

	err := s.db.DB().QueryRow(
		"SELECT id, created_by, title, meeting_type, language, status, created_at, updated_at FROM meetings WHERE id = $1",
		meetingID,
	).Scan(&meeting.ID, &createdBy, &meeting.Title, &meeting.MeetingType, &meeting.Language, &meeting.Status, &createdAt, &updatedAt)

	if err != nil {
		respondError(w, http.StatusNotFound, "Meeting not found")
		return
	}

	meeting.CreatedBy = &createdBy

	respondJSON(w, http.StatusOK, meeting)
}

func (s *Server) updateMeetingHandler(w http.ResponseWriter, r *http.Request) {
	meetingID := chi.URLParam(r, "meetingID")

	var req models.Meeting
	if err := parseJSON(r, &req); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	_, err := s.db.DB().Exec(
		"UPDATE meetings SET title = $1, meeting_type = $2, language = $3, updated_at = $4 WHERE id = $5",
		req.Title, req.MeetingType, req.Language, time.Now(), meetingID,
	)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to update meeting")
		return
	}

	respondJSON(w, http.StatusOK, map[string]string{"status": "updated"})
}

func (s *Server) deleteMeetingHandler(w http.ResponseWriter, r *http.Request) {
	meetingID := chi.URLParam(r, "meetingID")

	_, err := s.db.DB().Exec("DELETE FROM meetings WHERE id = $1", meetingID)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to delete meeting")
		return
	}

	respondJSON(w, http.StatusOK, map[string]string{"status": "deleted"})
}
