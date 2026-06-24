package api

import (
	"net/http"
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

func (s *Server) adminListUsersHandler(w http.ResponseWriter, r *http.Request) {
	rows, err := s.db.DB().Query("SELECT id, email, name, role, created_at, updated_at FROM users ORDER BY created_at DESC")
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to list users")
		return
	}
	defer rows.Close()

	var users []models.User
	for rows.Next() {
		var user models.User
		if err := rows.Scan(&user.ID, &user.Email, &user.Name, &user.Role, &user.CreatedAt, &user.UpdatedAt); err != nil {
			continue
		}
		users = append(users, user)
	}

	respondJSON(w, http.StatusOK, map[string]interface{}{"users": users})
}

func (s *Server) adminAuditLogHandler(w http.ResponseWriter, r *http.Request) {
	limit := 100
	rows, err := s.db.DB().Query(
		"SELECT id, user_id, action, resource_type, resource_id, details, ip_address, user_agent, created_at FROM audit_log ORDER BY created_at DESC LIMIT $1",
		limit,
	)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to list audit log")
		return
	}
	defer rows.Close()

	var logs []models.AuditLog
	for rows.Next() {
		var log models.AuditLog
		if err := rows.Scan(&log.ID, &log.UserID, &log.Action, &log.ResourceType, &log.ResourceID, &log.Details, &log.IPAddress, &log.UserAgent, &log.CreatedAt); err != nil {
			continue
		}
		logs = append(logs, log)
	}

	respondJSON(w, http.StatusOK, map[string]interface{}{"audit_log": logs})
}

func (s *Server) adminStatsHandler(w http.ResponseWriter, r *http.Request) {
	stats := map[string]interface{}{
		"users":           s.count("users"),
		"meetings":        s.count("meetings"),
		"organizations":   s.count("organizations"),
		"sync_queue":      s.count("sync_queue"),
		"transcripts":     s.count("transcript_segments"),
		"action_items":    s.count("action_items"),
		"decisions":       s.count("decisions"),
		"audit_log":       s.count("audit_log"),
	}

	respondJSON(w, http.StatusOK, stats)
}

func (s *Server) count(table string) int {
	var count int
	// Note: In production, never use raw table names like this
	// This is a simplified example
	row := s.db.DB().QueryRow("SELECT COUNT(*) FROM " + table)
	if err := row.Scan(&count); err != nil {
		return 0
	}
	return count
}
