package api

import (
	"errors"
	"net/http"
	"strconv"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"
	"github.com/ondemandworld/recap-team-server/internal/auth"
	"github.com/ondemandworld/recap-team-server/internal/models"
)

// canAccess reports whether user may read/mutate meeting. The owning user
// always can; admins may access any meeting (they can also see all meetings
// via the /admin routes). Meetings with no recorded owner are only accessible
// to admins.
func (s *Server) canAccess(user *models.User, meeting *models.Meeting) bool {
	if user == nil || meeting == nil {
		return false
	}
	if user.Role == "admin" {
		return true
	}
	return meeting.CreatedBy != nil && *meeting.CreatedBy == user.ID
}

// listMeetingsPagination bounds and normalizes ?limit / ?offset.
func listMeetingsPagination(r *http.Request) (limit, offset int) {
	limit, offset = 100, 0
	if v, err := strconv.Atoi(r.URL.Query().Get("limit")); err == nil && v > 0 {
		limit = v
	}
	if v, err := strconv.Atoi(r.URL.Query().Get("offset")); err == nil && v >= 0 {
		offset = v
	}
	if limit > 500 {
		limit = 500
	}
	return limit, offset
}

func (s *Server) listMeetingsHandler(w http.ResponseWriter, r *http.Request) {
	user := auth.GetUserFromContext(r.Context())
	if user == nil {
		respondError(w, http.StatusUnauthorized, "Unauthorized")
		return
	}
	if s.meetings == nil {
		respondError(w, http.StatusInternalServerError, "Meeting store not configured")
		return
	}

	// Tenant isolation: a user only ever sees their own meetings here.
	limit, offset := listMeetingsPagination(r)
	owned, err := s.meetings.ListByOwner(r.Context(), user.ID, limit, offset)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to list meetings")
		return
	}

	meetings := make([]map[string]interface{}, 0, len(owned))
	for _, m := range owned {
		meetings = append(meetings, map[string]interface{}{
			"id":           m.ID,
			"title":        m.Title,
			"meeting_type": m.MeetingType,
			"language":     m.Language,
			"status":       m.Status,
			"created_at":   m.CreatedAt,
			"updated_at":   m.UpdatedAt,
		})
	}

	respondJSON(w, http.StatusOK, map[string]interface{}{"meetings": meetings})
}

func (s *Server) createMeetingHandler(w http.ResponseWriter, r *http.Request) {
	user := auth.GetUserFromContext(r.Context())
	if user == nil {
		respondError(w, http.StatusUnauthorized, "Unauthorized")
		return
	}
	if s.meetings == nil {
		respondError(w, http.StatusInternalServerError, "Meeting store not configured")
		return
	}

	var req models.Meeting
	if err := parseJSON(w, r, &req); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	now := time.Now()
	meeting := &models.Meeting{
		ID:          uuid.New(),
		CreatedBy:   &user.ID,
		Title:       req.Title,
		MeetingType: req.MeetingType,
		Language:    req.Language,
		Status:      "pending",
		CreatedAt:   now,
		UpdatedAt:   now,
	}

	if err := s.meetings.Create(r.Context(), meeting); err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to create meeting")
		return
	}

	s.writeAudit(r, user, "meeting.create", &meeting.ID)

	respondJSON(w, http.StatusCreated, map[string]interface{}{"id": meeting.ID, "status": meeting.Status})
}

func (s *Server) getMeetingHandler(w http.ResponseWriter, r *http.Request) {
	user := auth.GetUserFromContext(r.Context())
	if user == nil {
		respondError(w, http.StatusUnauthorized, "Unauthorized")
		return
	}
	if s.meetings == nil {
		respondError(w, http.StatusInternalServerError, "Meeting store not configured")
		return
	}

	meetingID, err := uuid.Parse(chi.URLParam(r, "meetingID"))
	if err != nil {
		respondError(w, http.StatusNotFound, "Meeting not found")
		return
	}

	meeting, err := s.meetings.Get(r.Context(), meetingID)
	if errors.Is(err, ErrMeetingNotFound) {
		respondError(w, http.StatusNotFound, "Meeting not found")
		return
	}
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to get meeting")
		return
	}

	if !s.canAccess(user, meeting) {
		respondError(w, http.StatusForbidden, "You do not have access to this meeting")
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
	if s.meetings == nil {
		respondError(w, http.StatusInternalServerError, "Meeting store not configured")
		return
	}

	meetingID, err := uuid.Parse(chi.URLParam(r, "meetingID"))
	if err != nil {
		respondError(w, http.StatusNotFound, "Meeting not found")
		return
	}

	var req models.Meeting
	if err := parseJSON(w, r, &req); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	meeting, err := s.meetings.Get(r.Context(), meetingID)
	if errors.Is(err, ErrMeetingNotFound) {
		respondError(w, http.StatusNotFound, "Meeting not found")
		return
	}
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to get meeting")
		return
	}

	if !s.canAccess(user, meeting) {
		respondError(w, http.StatusForbidden, "You do not have access to this meeting")
		return
	}

	meeting.Title = req.Title
	meeting.MeetingType = req.MeetingType
	meeting.Language = req.Language
	meeting.UpdatedAt = time.Now()

	if err := s.meetings.Update(r.Context(), meeting); err != nil {
		if errors.Is(err, ErrMeetingNotFound) {
			respondError(w, http.StatusNotFound, "Meeting not found")
			return
		}
		respondError(w, http.StatusInternalServerError, "Failed to update meeting")
		return
	}

	s.writeAudit(r, user, "meeting.update", &meeting.ID)

	respondJSON(w, http.StatusOK, map[string]string{"status": "updated"})
}

func (s *Server) deleteMeetingHandler(w http.ResponseWriter, r *http.Request) {
	user := auth.GetUserFromContext(r.Context())
	if user == nil {
		respondError(w, http.StatusUnauthorized, "Unauthorized")
		return
	}
	if s.meetings == nil {
		respondError(w, http.StatusInternalServerError, "Meeting store not configured")
		return
	}

	meetingID, err := uuid.Parse(chi.URLParam(r, "meetingID"))
	if err != nil {
		respondError(w, http.StatusNotFound, "Meeting not found")
		return
	}

	meeting, err := s.meetings.Get(r.Context(), meetingID)
	if errors.Is(err, ErrMeetingNotFound) {
		respondError(w, http.StatusNotFound, "Meeting not found")
		return
	}
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to get meeting")
		return
	}

	if !s.canAccess(user, meeting) {
		respondError(w, http.StatusForbidden, "You do not have access to this meeting")
		return
	}

	if err := s.meetings.Delete(r.Context(), meetingID); err != nil {
		if errors.Is(err, ErrMeetingNotFound) {
			respondError(w, http.StatusNotFound, "Meeting not found")
			return
		}
		respondError(w, http.StatusInternalServerError, "Failed to delete meeting")
		return
	}

	s.writeAudit(r, user, "meeting.delete", &meeting.ID)

	respondJSON(w, http.StatusOK, map[string]string{"status": "deleted"})
}
