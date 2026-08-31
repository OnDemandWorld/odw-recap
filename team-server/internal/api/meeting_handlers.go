package api

import (
	"database/sql"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
	"github.com/ondemandworld/recap-team-server/internal/auth"
	"github.com/ondemandworld/recap-team-server/internal/models"
)

// parseMeetingIDParam extracts and validates the {meetingID} URL parameter.
func parseMeetingIDParam(r *http.Request) (uuid.UUID, bool) {
	id, err := uuid.Parse(chi.URLParam(r, "meetingID"))
	if err != nil {
		return uuid.Nil, false
	}
	return id, true
}

func (s *Server) listMeetingsHandler(w http.ResponseWriter, r *http.Request) {
	user := auth.GetUserFromContext(r.Context())
	if user == nil {
		respondError(w, http.StatusUnauthorized, "Unauthorized")
		return
	}

	// Only return meetings owned by the caller. Nullable columns are scanned
	// into sql.Null* types so rows containing NULLs are not silently dropped.
	rows, err := s.db.DB().Query(
		`SELECT id, title, meeting_type, language, status, created_at, updated_at
		 FROM meetings WHERE created_by = $1 ORDER BY created_at DESC LIMIT 100`,
		user.ID,
	)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to list meetings")
		return
	}
	defer rows.Close()

	meetings := []map[string]interface{}{}
	for rows.Next() {
		var id uuid.UUID
		var title, meetingType, language sql.NullString
		var status string
		var createdAt, updatedAt time.Time

		if err := rows.Scan(&id, &title, &meetingType, &language, &status, &createdAt, &updatedAt); err != nil {
			respondError(w, http.StatusInternalServerError, "Failed to read meetings")
			return
		}

		meetings = append(meetings, map[string]interface{}{
			"id":           id,
			"title":        title.String,
			"meeting_type": meetingType.String,
			"language":     language.String,
			"status":       status,
			"created_at":   createdAt,
			"updated_at":   updatedAt,
		})
	}
	if err := rows.Err(); err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to list meetings")
		return
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
	if user == nil {
		respondError(w, http.StatusUnauthorized, "Unauthorized")
		return
	}

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

// getOwnedMeeting fetches a meeting that belongs to the requesting user.
// Returns (nil, false) for a malformed ID or a meeting that does not exist
// for this user; callers respond with 400 or 404 respectively.
func (s *Server) getOwnedMeeting(r *http.Request, user *models.User) (*models.Meeting, bool, bool) {
	meetingID, ok := parseMeetingIDParam(r)
	if !ok {
		return nil, false, false
	}

	var meeting models.Meeting
	// createdBy is a pointer so a NULL column (creator deleted) scans as nil.
	var createdBy *uuid.UUID
	var createdAt, updatedAt time.Time

	err := s.db.DB().QueryRow(
		`SELECT id, created_by, title, meeting_type, language, status, created_at, updated_at
		 FROM meetings WHERE id = $1`,
		meetingID,
	).Scan(&meeting.ID, &createdBy, &meeting.Title, &meeting.MeetingType, &meeting.Language, &meeting.Status, &createdAt, &updatedAt)

	if err != nil {
		return nil, true, false
	}

	// Ownership check: only the creator (or an admin) may access the meeting.
	if user.Role != "admin" && (createdBy == nil || *createdBy != user.ID) {
		return nil, true, false
	}

	meeting.CreatedAt = createdAt
	meeting.UpdatedAt = updatedAt
	meeting.CreatedBy = createdBy

	return &meeting, true, true
}

func (s *Server) getMeetingHandler(w http.ResponseWriter, r *http.Request) {
	user := auth.GetUserFromContext(r.Context())
	if user == nil {
		respondError(w, http.StatusUnauthorized, "Unauthorized")
		return
	}

	meeting, validID, found := s.getOwnedMeeting(r, user)
	if !validID {
		respondError(w, http.StatusBadRequest, "Invalid meeting ID")
		return
	}
	if !found {
		respondError(w, http.StatusNotFound, "Meeting not found")
		return
	}

	respondJSON(w, http.StatusOK, meeting)
}

func (s *Server) updateMeetingHandler(w http.ResponseWriter, r *http.Request) {
	user := auth.GetUserFromContext(r.Context())
	if user == nil {
		respondError(w, http.StatusUnauthorized, "Unauthorized")
		return
	}

	meeting, validID, found := s.getOwnedMeeting(r, user)
	if !validID {
		respondError(w, http.StatusBadRequest, "Invalid meeting ID")
		return
	}
	if !found {
		respondError(w, http.StatusNotFound, "Meeting not found")
		return
	}

	var req models.Meeting
	if err := parseJSON(r, &req); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	// Partial update: only overwrite fields that were provided. Using COALESCE
	// prevents an omitted field from wiping the stored value.
	result, err := s.db.DB().Exec(
		`UPDATE meetings SET
			title = COALESCE($1, title),
			meeting_type = COALESCE($2, meeting_type),
			language = COALESCE($3, language),
			updated_at = $4
		 WHERE id = $5`,
		req.Title, req.MeetingType, req.Language, time.Now(), meeting.ID,
	)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to update meeting")
		return
	}

	if n, err := result.RowsAffected(); err == nil && n == 0 {
		respondError(w, http.StatusNotFound, "Meeting not found")
		return
	}

	respondJSON(w, http.StatusOK, map[string]string{"status": "updated"})
}

func (s *Server) deleteMeetingHandler(w http.ResponseWriter, r *http.Request) {
	user := auth.GetUserFromContext(r.Context())
	if user == nil {
		respondError(w, http.StatusUnauthorized, "Unauthorized")
		return
	}

	meeting, validID, found := s.getOwnedMeeting(r, user)
	if !validID {
		respondError(w, http.StatusBadRequest, "Invalid meeting ID")
		return
	}
	if !found {
		respondError(w, http.StatusNotFound, "Meeting not found")
		return
	}

	result, err := s.db.DB().Exec("DELETE FROM meetings WHERE id = $1", meeting.ID)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to delete meeting")
		return
	}

	if n, err := result.RowsAffected(); err == nil && n == 0 {
		respondError(w, http.StatusNotFound, "Meeting not found")
		return
	}

	respondJSON(w, http.StatusOK, map[string]string{"status": "deleted"})
}
