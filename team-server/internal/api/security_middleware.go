package api

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"strings"
	"time"

	"github.com/ondemandworld/recap-team-server/internal/auth"
)

// clientIP extracts the host part of the remote address (RealIP middleware
// rewrites RemoteAddr when X-Forwarded-For is present).
func clientIP(r *http.Request) string {
	ip := r.RemoteAddr
	if idx := strings.LastIndex(ip, ":"); idx != -1 {
		ip = ip[:idx]
	}
	return ip
}

// rateLimit caps requests per client IP for the wrapped route. If Redis is
// unavailable it fails open (availability over hard throttling) and logs.
func (s *Server) rateLimit(maxRequests int64, window time.Duration) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			if s.redis == nil {
				next.ServeHTTP(w, r)
				return
			}

			key := fmt.Sprintf("ratelimit:%s:%s", r.URL.Path, clientIP(r))
			ctx := context.Background()

			count, err := s.redis.Client().Incr(ctx, key).Result()
			if err != nil {
				log.Printf("ratelimit: redis error, failing open: %v", err)
				next.ServeHTTP(w, r)
				return
			}
			if count == 1 {
				s.redis.Client().Expire(ctx, key, window)
			}

			if count > maxRequests {
				respondError(w, http.StatusTooManyRequests, "Too many requests, please slow down")
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// userSync re-validates the token's user against the database on every
// request. Claims alone are trusted for nothing beyond identifying the user:
// deleted accounts are rejected immediately and role changes take effect
// without waiting for the access token to expire.
func (s *Server) userSync(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		user := auth.GetUserFromContext(r.Context())
		if user == nil {
			http.Error(w, "Unauthorized", http.StatusUnauthorized)
			return
		}

		if s.db != nil {
			var email, role string
			err := s.db.DB().QueryRow(
				"SELECT email, role FROM users WHERE id = $1", user.ID,
			).Scan(&email, &role)
			if err != nil {
				// Unknown user: deleted or never existed (forged/old token).
				http.Error(w, "Unauthorized", http.StatusUnauthorized)
				return
			}
			user.Email = email
			user.Role = role
		}

		next.ServeHTTP(w, r)
	})
}
