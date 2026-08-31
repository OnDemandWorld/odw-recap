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

// statsQueries maps each stat name to a fixed COUNT query. Queries are static
// literals — table names are never assembled from user input.
var statsQueries = map[string]string{
	"users":         "SELECT COUNT(*) FROM users",
	"meetings":      "SELECT COUNT(*) FROM meetings",
	"organizations": "SELECT COUNT(*) FROM organizations",
	"sync_queue":    "SELECT COUNT(*) FROM sync_queue",
	"transcripts":   "SELECT COUNT(*) FROM transcript_segments",
	"action_items":  "SELECT COUNT(*) FROM action_items",
	"decisions":     "SELECT COUNT(*) FROM decisions",
	"audit_log":     "SELECT COUNT(*) FROM audit_log",
}

func (s *Server) adminStatsHandler(w http.ResponseWriter, r *http.Request) {
	stats := make(map[string]interface{}, len(statsQueries))
	for name, query := range statsQueries {
		stats[name] = s.count(query)
	}

	respondJSON(w, http.StatusOK, stats)
}

func (s *Server) count(query string) int {
	var count int
	row := s.db.DB().QueryRow(query)
	if err := row.Scan(&count); err != nil {
		return 0
	}
	return count
}
