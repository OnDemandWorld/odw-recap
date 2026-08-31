package api

import (
	"encoding/json"
	"log"
	"net/http"
	"os"
	"strings"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/go-chi/cors"
	"github.com/ondemandworld/recap-team-server/internal/auth"
	"github.com/ondemandworld/recap-team-server/internal/db"
)

// maxJSONBodyBytes caps the size of JSON request bodies to prevent abuse.
// Sync payloads embed transcript data, so this is generous but bounded.
const maxJSONBodyBytes = 10 << 20 // 10 MiB

// Server represents the API server
type Server struct {
	db         *db.Database
	redis      *db.Redis
	jwtManager *auth.JWTManager
}

// NewServer creates a new API server
func NewServer(database *db.Database, redis *db.Redis, jwtSecret string) *Server {
	s := &Server{
		db:         database,
		redis:      redis,
		jwtManager: auth.NewJWTManager(jwtSecret),
	}
	return s
}

// Router sets up and returns the HTTP router
func (s *Server) Router() http.Handler {
	r := chi.NewRouter()

	// Middleware
	r.Use(middleware.Logger)
	r.Use(middleware.Recoverer)
	r.Use(middleware.RequestID)
	r.Use(middleware.RealIP)
	r.Use(middleware.Timeout(60 * time.Second))

	// CORS: origins are configurable via CORS_ALLOWED_ORIGINS (comma-separated).
	// Never combine a wildcard origin with AllowCredentials — that would let any
	// website make credentialed requests to this API.
	allowedOrigins := []string{"*"}
	allowCredentials := false
	if origins := os.Getenv("CORS_ALLOWED_ORIGINS"); origins != "" {
		allowedOrigins = strings.Split(origins, ",")
		for i := range allowedOrigins {
			allowedOrigins[i] = strings.TrimSpace(allowedOrigins[i])
		}
		allowCredentials = true
	}
	r.Use(cors.Handler(cors.Options{
		AllowedOrigins:   allowedOrigins,
		AllowedMethods:   []string{"GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"},
		AllowedHeaders:   []string{"Accept", "Authorization", "Content-Type"},
		ExposedHeaders:   []string{"Link"},
		AllowCredentials: allowCredentials,
		MaxAge:           300,
	}))

	// Public routes
	r.Get("/health", s.healthHandler)
	r.Post("/auth/register", s.registerHandler)
	r.Post("/auth/login", s.loginHandler)

	// Protected routes
	r.Group(func(r chi.Router) {
		r.Use(auth.Middleware(s.jwtManager))

		// User routes
		r.Get("/me", s.meHandler)

		// Meeting routes
		r.Route("/meetings", func(r chi.Router) {
			r.Get("/", s.listMeetingsHandler)
			r.Post("/", s.createMeetingHandler)
			r.Get("/{meetingID}", s.getMeetingHandler)
			r.Put("/{meetingID}", s.updateMeetingHandler)
			r.Delete("/{meetingID}", s.deleteMeetingHandler)
		})

		// Sync routes
		r.Post("/sync", s.syncHandler)

		// Admin routes
		r.Route("/admin", func(r chi.Router) {
			r.Use(auth.RequireRole("admin"))
			r.Get("/users", s.adminListUsersHandler)
			r.Get("/audit-log", s.adminAuditLogHandler)
			r.Get("/stats", s.adminStatsHandler)
		})
	})

	return r
}

// JSON response helper
func respondJSON(w http.ResponseWriter, status int, payload interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(payload); err != nil {
		// Headers are already written; we can only log the failure.
		log.Printf("respondJSON: failed to encode payload: %v", err)
	}
}

// Error response helper
func respondError(w http.ResponseWriter, status int, message string) {
	respondJSON(w, status, map[string]string{"error": message})
}
