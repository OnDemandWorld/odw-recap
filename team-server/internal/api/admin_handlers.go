package api

import (
	"net/http"
	"strconv"
	"time"

	"github.com/ondemandworld/recap-team-server/internal/models"
)

func (s *Server) healthHandler(w http.ResponseWriter, r *http.Request) {
	respondJSON(w, http.StatusOK, map[string]string{
		"status":    "ok",
		"service":   "recap-team-server",
		"version":   "1.0.0",
		"timestamp": time.Now().Format(time.RFC3339),
	})
}

// adminPagination bounds and normalizes ?limit / ?offset for admin list
// endpoints.
func adminPagination(r *http.Request) (limit, offset int) {
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

func (s *Server) adminListUsersHandler(w http.ResponseWriter, r *http.Request) {
	limit, offset := adminPagination(r)
	rows, err := s.db.DB().Query(
		"SELECT id, email, name, role, created_at, updated_at FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2",
		limit, offset,
	)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to list users")
		return
	}
	defer rows.Close()

	users := make([]models.User, 0)
	for rows.Next() {
		var user models.User
		if err := rows.Scan(&user.ID, &user.Email, &user.Name, &user.Role, &user.CreatedAt, &user.UpdatedAt); err != nil {
			respondError(w, http.StatusInternalServerError, "Failed to read users")
			return
		}
		users = append(users, user)
	}
	if err := rows.Err(); err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to read users")
		return
	}

	respondJSON(w, http.StatusOK, map[string]interface{}{"users": users})
}

func (s *Server) adminAuditLogHandler(w http.ResponseWriter, r *http.Request) {
	limit, offset := adminPagination(r)
	rows, err := s.db.DB().Query(
		"SELECT id, user_id, action, resource_type, resource_id, details, ip_address, user_agent, created_at FROM audit_log ORDER BY created_at DESC LIMIT $1 OFFSET $2",
		limit, offset,
	)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to list audit log")
		return
	}
	defer rows.Close()

	logs := make([]models.AuditLog, 0)
	for rows.Next() {
		var log models.AuditLog
		if err := rows.Scan(&log.ID, &log.UserID, &log.Action, &log.ResourceType, &log.ResourceID, &log.Details, &log.IPAddress, &log.UserAgent, &log.CreatedAt); err != nil {
			respondError(w, http.StatusInternalServerError, "Failed to read audit log")
			return
		}
		logs = append(logs, log)
	}
	if err := rows.Err(); err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to read audit log")
		return
	}

	respondJSON(w, http.StatusOK, map[string]interface{}{"audit_log": logs})
}

// adminStatsTables is the fixed set of entity counts reported by
// adminStatsHandler. Table names are compile-time constants — never accept
// them from request input.
var adminStatsTables = []struct {
	name  string
	table string
}{
	{"users", "users"},
	{"meetings", "meetings"},
	{"organizations", "organizations"},
	{"sync_queue", "sync_queue"},
	{"transcripts", "transcript_segments"},
	{"action_items", "action_items"},
	{"decisions", "decisions"},
	{"audit_log", "audit_log"},
}

func (s *Server) adminStatsHandler(w http.ResponseWriter, r *http.Request) {
	stats := make(map[string]interface{}, len(adminStatsTables))
	for _, stat := range adminStatsTables {
		stats[stat.name] = s.count(stat.table)
	}

	respondJSON(w, http.StatusOK, stats)
}

func (s *Server) count(table string) int {
	var count int
	// table always comes from adminStatsTables, never from request input.
	row := s.db.DB().QueryRow("SELECT COUNT(*) FROM " + table)
	if err := row.Scan(&count); err != nil {
		return 0
	}
	return count
}
