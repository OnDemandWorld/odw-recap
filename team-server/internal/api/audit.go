package api

import (
	"log"
	"net/http"
	"time"

	"github.com/google/uuid"
)

// writeAudit records a security-relevant event. Audit writes must never
// break the request path, so failures are logged and swallowed.
func (s *Server) writeAudit(r *http.Request, userID *uuid.UUID, action string, resourceType *string, details string) {
	if s.db == nil {
		return
	}

	var ipAddress, userAgent *string
	if r != nil {
		ip := r.RemoteAddr
		ipAddress = &ip
		ua := r.UserAgent()
		userAgent = &ua
	}

	_, err := s.db.DB().Exec(
		`INSERT INTO audit_log (user_id, action, resource_type, details, ip_address, user_agent, created_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7)`,
		userID, action, resourceType, details, ipAddress, userAgent, time.Now(),
	)
	if err != nil {
		log.Printf("audit: failed to write %q event: %v", action, err)
	}
}
