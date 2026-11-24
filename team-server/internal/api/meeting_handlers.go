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

// validateMeetingFields bounds the fields backed by fixed-width columns so an
// over-length value returns a 400 instead of a pq 22001 500 (audit #75).
// meeting_type VARCHAR(100), language VARCHAR(10); title is TEXT but bounded to
// keep absurd payloads out of the Vault upload.
func validateMeetingFields(req *models.Meeting) string {
	if req.MeetingType != nil && len(*req.MeetingType) > 100 {
		return "meeting_type is too long (max 100 characters)"
	}
	if req.Language != nil && len(*req.Language) > 10 {
		return "language is too long (max 10 characters)"
	}
	if req.Title != nil && len(*req.Title) > 1000 {
		return "title is too long (max 1000 characters)"
	}
	return ""
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

	// Audit #2: the desktop sync client POSTs its own meeting UUID as `id`
	// expecting upsert semantics (it treats 201 or 409 as success, then sends
	// /sync referencing that id). Previously req.ID was discarded and a fresh
	// uuid.New() minted on every call, so each sync created a duplicate empty
	// meeting and /sync referenced a nonexistent row. Honor a client-supplied
	// id: create it when new, return 409 when the caller already owns it, and
	// 403 when it belongs to another tenant (never overwrite another's row).
	if msg := validateMeetingFields(&req); msg != "" {
		respondError(w, http.StatusBadRequest, msg)
		return
	}
	meetingID := uuid.New()
	if req.ID != uuid.Nil {
		existing, err := s.meetings.Get(r.Context(), req.ID)
		switch {
		case err == nil:
			if !s.canAccess(user, existing) {
				respondError(w, http.StatusForbidden, "You do not have access to this meeting")
				return
			}
			// Already exists and is accessible: idempotent upsert no-op. The
			// desktop client accepts 409 and proceeds to /sync against this id.
			respondJSON(w, http.StatusConflict, map[string]interface{}{"id": existing.ID, "status": existing.Status})
			return
		case errors.Is(err, ErrMeetingNotFound):
			meetingID = req.ID
		default:
			respondError(w, http.StatusInternalServerError, "Failed to create meeting")
			return
		}
	}

	now := time.Now()
	meeting := &models.Meeting{
		ID:          meetingID,
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

	s.writeAudit(r, user, "meeting.create", &meeting.ID, map[string]*string{"title": meeting.Title})

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

	if msg := validateMeetingFields(&req); msg != "" {
		respondError(w, http.StatusBadRequest, msg)
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

	s.writeAudit(r, user, "meeting.update", &meeting.ID, map[string]*string{"title": meeting.Title})

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

	s.writeAudit(r, user, "meeting.delete", &meeting.ID, map[string]*string{"title": meeting.Title})

	respondJSON(w, http.StatusOK, map[string]string{"status": "deleted"})
}
