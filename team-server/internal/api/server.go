package api

import (
	"encoding/json"
	"log"
	"net/http"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	"github.com/go-chi/cors"
	"github.com/ondemandworld/recap-team-server/internal/auth"
	"github.com/ondemandworld/recap-team-server/internal/db"
	odwsync "github.com/ondemandworld/recap-team-server/internal/sync"
)

// Server represents the API server
type Server struct {
	db             *db.Database
	redis          *db.Redis
	jwtManager     *auth.JWTManager
	syncForwarder  *odwsync.Forwarder
	meetings       MeetingStore
	audit          AuditLogger
	allowedOrigins []string
	rateLimiter    *rateLimiter
}

// DefaultAllowedOrigins is the CORS origin allowlist used unless
// SetAllowedOrigins overrides it. Authentication is bearer-token based, so no
// ambient browser credentials are needed and AllowCredentials stays false.
var DefaultAllowedOrigins = []string{"*"}

// NewServer creates a new API server. When a non-nil database is supplied the
// PostgreSQL-backed meeting store and audit logger are wired automatically.
func NewServer(database *db.Database, redis *db.Redis, jwtSecret string) *Server {
	s := &Server{
		db:             database,
		redis:          redis,
		jwtManager:     auth.NewJWTManager(jwtSecret),
		allowedOrigins: DefaultAllowedOrigins,
		rateLimiter:    newRateLimiter(loginRateLimit, loginRateWindow),
	}
	if database != nil {
		s.meetings = &dbMeetingStore{database: database}
		s.audit = &dbAuditLogger{database: database}
	}
	return s
}

// SetAllowedOrigins overrides the CORS origin allowlist. Pass explicit origins
// (e.g. ["https://recap.example.com"]) when hosting a browser-based client;
// keep ["*"] only for development or native clients.
func (s *Server) SetAllowedOrigins(origins []string) {
	if len(origins) > 0 {
		s.allowedOrigins = origins
	}
}

// SetSyncForwarder attaches the cross-product sync forwarder used by POST /sync
// to push meeting knowledge into Vault and trigger Loop workflows.
func (s *Server) SetSyncForwarder(f *odwsync.Forwarder) {
	s.syncForwarder = f
}

// SetMeetingStore overrides the meeting store (used by tests to inject an
// in-memory fake).
func (s *Server) SetMeetingStore(m MeetingStore) {
	s.meetings = m
}

// SetAuditLogger overrides the audit logger (used by tests to inject an
// in-memory recorder).
func (s *Server) SetAuditLogger(a AuditLogger) {
	s.audit = a
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

	// CORS. Credentials are never allowed: the API is bearer-token based and
	// echoing arbitrary origins together with credentials would let any site
	// make credentialed cross-origin calls (go-chi/cors mirrors the request
	// origin when "*" is combined with AllowCredentials).
	r.Use(cors.Handler(cors.Options{
		AllowedOrigins:   s.allowedOrigins,
		AllowedMethods:   []string{"GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"},
		AllowedHeaders:   []string{"Accept", "Authorization", "Content-Type"},
		ExposedHeaders:   []string{"Link"},
		AllowCredentials: false,
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

// JSON response helper. Once the status and payload are being written the
// response can no longer be restarted, so an encoder failure can only be
// logged — calling http.Error here would append malformed output and trigger
// a "superfluous WriteHeader" warning.
func respondJSON(w http.ResponseWriter, status int, payload interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(payload); err != nil {
		log.Printf("respondJSON: failed to encode payload: %v", err)
	}
}

// Error response helper
func respondError(w http.ResponseWriter, status int, message string) {
	respondJSON(w, status, map[string]string{"error": message})
}
