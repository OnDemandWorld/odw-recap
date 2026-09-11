package api

import (
	"context"
	"database/sql"
	"errors"
	"log"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/ondemandworld/recap-team-server/internal/db"
	"github.com/ondemandworld/recap-team-server/internal/models"
)

// ErrMeetingNotFound is returned by MeetingStore.Get when no meeting matches
// the requested id. Handlers map it to HTTP 404.
var ErrMeetingNotFound = errors.New("meeting not found")

// MeetingStore abstracts meeting persistence. It exists so the meeting
// handlers can be unit-tested without a live PostgreSQL database and so the
// tenant-ownership rules are enforced consistently in the handlers above a
// single storage seam.
type MeetingStore interface {
	// Create persists a new meeting. The meeting's CreatedBy field is its
	// owning user.
	Create(ctx context.Context, m *models.Meeting) error
	// Get returns a meeting by id regardless of owner; ownership checks are
	// performed by the caller. Returns ErrMeetingNotFound when absent.
	Get(ctx context.Context, id uuid.UUID) (*models.Meeting, error)
	// ListByOwner returns only the meetings owned by ownerID (tenant scope),
	// newest first, with limit/offset pagination.
	ListByOwner(ctx context.Context, ownerID uuid.UUID, limit, offset int) ([]*models.Meeting, error)
	// Update mutates an existing meeting's editable fields.
	Update(ctx context.Context, m *models.Meeting) error
	// Delete removes a meeting by id.
	Delete(ctx context.Context, id uuid.UUID) error
}

// AuditLogger records audit events on a best-effort basis. Implementations
// must never propagate failures to the caller — an audit write failure must
// not fail the originating request.
type AuditLogger interface {
	Log(ctx context.Context, entry *models.AuditLog)
}

// dbMeetingStore is the PostgreSQL-backed MeetingStore.
type dbMeetingStore struct {
	database *db.Database
}

func (s *dbMeetingStore) Create(ctx context.Context, m *models.Meeting) error {
	_, err := s.database.DB().ExecContext(ctx,
		"INSERT INTO meetings (id, created_by, title, meeting_type, language, status, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
		m.ID, m.CreatedBy, m.Title, m.MeetingType, m.Language, m.Status, m.CreatedAt, m.UpdatedAt,
	)
	return err
}

func (s *dbMeetingStore) Get(ctx context.Context, id uuid.UUID) (*models.Meeting, error) {
	var m models.Meeting
	var createdBy *uuid.UUID
	err := s.database.DB().QueryRowContext(ctx,
		"SELECT id, created_by, title, meeting_type, language, status, created_at, updated_at FROM meetings WHERE id = $1",
		id,
	).Scan(&m.ID, &createdBy, &m.Title, &m.MeetingType, &m.Language, &m.Status, &m.CreatedAt, &m.UpdatedAt)
	if errors.Is(err, sql.ErrNoRows) {
		return nil, ErrMeetingNotFound
	}
	if err != nil {
		return nil, err
	}
	m.CreatedBy = createdBy
	return &m, nil
}

func (s *dbMeetingStore) ListByOwner(ctx context.Context, ownerID uuid.UUID, limit, offset int) ([]*models.Meeting, error) {
	rows, err := s.database.DB().QueryContext(ctx,
		"SELECT id, created_by, title, meeting_type, language, status, created_at, updated_at FROM meetings WHERE created_by = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
		ownerID, limit, offset,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	meetings := make([]*models.Meeting, 0)
	for rows.Next() {
		var m models.Meeting
		var createdBy *uuid.UUID
		if err := rows.Scan(&m.ID, &createdBy, &m.Title, &m.MeetingType, &m.Language, &m.Status, &m.CreatedAt, &m.UpdatedAt); err != nil {
			return nil, err
		}
		m.CreatedBy = createdBy
		meetings = append(meetings, &m)
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}
	return meetings, nil
}

func (s *dbMeetingStore) Update(ctx context.Context, m *models.Meeting) error {
	res, err := s.database.DB().ExecContext(ctx,
		"UPDATE meetings SET title = $1, meeting_type = $2, language = $3, updated_at = $4 WHERE id = $5",
		m.Title, m.MeetingType, m.Language, m.UpdatedAt, m.ID,
	)
	if err != nil {
		return err
	}
	if n, err := res.RowsAffected(); err == nil && n == 0 {
		// The row vanished between the handler's Get and this Update
		// (TOCTOU); surface it instead of reporting success on a no-op.
		return ErrMeetingNotFound
	}
	return nil
}

func (s *dbMeetingStore) Delete(ctx context.Context, id uuid.UUID) error {
	res, err := s.database.DB().ExecContext(ctx, "DELETE FROM meetings WHERE id = $1", id)
	if err != nil {
		return err
	}
	if n, err := res.RowsAffected(); err == nil && n == 0 {
		return ErrMeetingNotFound
	}
	return nil
}

// dbAuditLogger is the PostgreSQL-backed AuditLogger. Writes are best-effort:
// any failure is logged and swallowed.
type dbAuditLogger struct {
	database *db.Database
}

func (l *dbAuditLogger) Log(ctx context.Context, e *models.AuditLog) {
	_, err := l.database.DB().ExecContext(ctx,
		"INSERT INTO audit_log (user_id, action, resource_type, resource_id, ip_address, user_agent, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
		e.UserID, e.Action, e.ResourceType, e.ResourceID, e.IPAddress, e.UserAgent, e.CreatedAt,
	)
	if err != nil {
		log.Printf("audit: failed to write audit log entry (action=%s): %v", e.Action, err)
	}
}

// writeAudit records a "meeting"-scoped audit event for the authenticated
// user. It is best-effort and safe to call when no audit logger is configured
// (e.g. DB-less test servers) or when no user is present.
func (s *Server) writeAudit(r *http.Request, user *models.User, action string, resourceID *uuid.UUID) {
	if s.audit == nil || user == nil {
		return
	}

	resourceType := "meeting"
	ip := r.RemoteAddr
	userAgent := r.UserAgent()

	s.audit.Log(r.Context(), &models.AuditLog{
		UserID:       &user.ID,
		Action:       action,
		ResourceType: &resourceType,
		ResourceID:   resourceID,
		IPAddress:    &ip,
		UserAgent:    &userAgent,
		CreatedAt:    time.Now(),
	})
}
