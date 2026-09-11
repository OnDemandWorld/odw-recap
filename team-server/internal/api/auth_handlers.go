package api

import (
	"database/sql"
	"encoding/json"
	"errors"
	"net"
	"net/http"
	"regexp"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/lib/pq"
	"github.com/ondemandworld/recap-team-server/internal/auth"
	"github.com/ondemandworld/recap-team-server/internal/models"
	"golang.org/x/crypto/bcrypt"
)

// maxJSONBodySize bounds every JSON request body. Sync payloads carry
// transcript/summary metadata (not audio), so a few MiB is generous.
const maxJSONBodySize = 4 << 20 // 4 MiB

// RegisterRequest represents a registration request
type RegisterRequest struct {
	Email    string `json:"email"`
	Password string `json:"password"`
	Name     string `json:"name"`
}

// LoginRequest represents a login request
type LoginRequest struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

// AuthResponse represents an authentication response
type AuthResponse struct {
	Token string      `json:"token"`
	User  models.User `json:"user"`
}

// emailPattern is a pragmatic (not RFC-exhaustive) shape check: one "@", a
// dotted domain, no whitespace.
var emailPattern = regexp.MustCompile(`^[^\s@]+@[^\s@]+\.[^\s@]+$`)

// validateRegisterRequest normalizes the request and returns a
// user-facing error message when the input is unacceptable. bcrypt only
// hashes the first 72 bytes, so longer passwords are rejected instead of
// silently truncated.
func validateRegisterRequest(req *RegisterRequest) string {
	req.Email = strings.ToLower(strings.TrimSpace(req.Email))
	req.Name = strings.TrimSpace(req.Name)

	switch {
	case req.Email == "" || req.Password == "" || req.Name == "":
		return "Email, password, and name are required"
	case !emailPattern.MatchString(req.Email):
		return "Invalid email address"
	case len(req.Name) > 255:
		return "Name is too long"
	case len(req.Password) < 8:
		return "Password must be at least 8 characters"
	case len(req.Password) > 72:
		return "Password must be at most 72 characters"
	}
	return ""
}

func (s *Server) registerHandler(w http.ResponseWriter, r *http.Request) {
	var req RegisterRequest
	if err := parseJSON(w, r, &req); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if msg := validateRegisterRequest(&req); msg != "" {
		respondError(w, http.StatusBadRequest, msg)
		return
	}

	// Hash password
	hashedPassword, err := bcrypt.GenerateFromPassword([]byte(req.Password), bcrypt.DefaultCost)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to hash password")
		return
	}

	// Create user
	user := &models.User{
		ID:           uuid.New(),
		Email:        req.Email,
		PasswordHash: string(hashedPassword),
		Name:         req.Name,
		Role:         "member",
		CreatedAt:    time.Now(),
		UpdatedAt:    time.Now(),
	}

	// Insert into database
	_, err = s.db.DB().Exec(
		"INSERT INTO users (id, email, password_hash, name, role, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
		user.ID, user.Email, user.PasswordHash, user.Name, user.Role, user.CreatedAt, user.UpdatedAt,
	)
	if err != nil {
		var pqErr *pq.Error
		if errors.As(err, &pqErr) && pqErr.Code == "23505" {
			respondError(w, http.StatusConflict, "An account with this email already exists")
			return
		}
		respondError(w, http.StatusInternalServerError, "Failed to create user")
		return
	}

	// Generate token
	token, err := s.jwtManager.GenerateToken(user.ID, user.Email, user.Role)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to generate token")
		return
	}

	respondJSON(w, http.StatusCreated, AuthResponse{
		Token: token,
		User:  *user,
	})
}

func (s *Server) loginHandler(w http.ResponseWriter, r *http.Request) {
	var req LoginRequest
	if err := parseJSON(w, r, &req); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	req.Email = strings.ToLower(strings.TrimSpace(req.Email))

	// Throttle credential-stuffing per client + account. Note the check runs
	// before the DB lookup: we still answer 401 for over-budget attempts so
	// as not to reveal whether the account exists.
	limiterKey := clientKey(r) + "|" + req.Email
	if !s.rateLimiter.Allow(limiterKey) {
		respondError(w, http.StatusTooManyRequests, "Too many login attempts, try again later")
		return
	}

	// Find user
	var user models.User
	var passwordHash string
	err := s.db.DB().QueryRow(
		"SELECT id, email, password_hash, name, role, created_at, updated_at FROM users WHERE email = $1",
		req.Email,
	).Scan(&user.ID, &user.Email, &passwordHash, &user.Name, &user.Role, &user.CreatedAt, &user.UpdatedAt)

	if errors.Is(err, sql.ErrNoRows) {
		respondError(w, http.StatusUnauthorized, "Invalid credentials")
		return
	} else if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to query user")
		return
	}

	// Verify password
	if err := bcrypt.CompareHashAndPassword([]byte(passwordHash), []byte(req.Password)); err != nil {
		respondError(w, http.StatusUnauthorized, "Invalid credentials")
		return
	}

	// Generate token
	token, err := s.jwtManager.GenerateToken(user.ID, user.Email, user.Role)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to generate token")
		return
	}

	respondJSON(w, http.StatusOK, AuthResponse{
		Token: token,
		User:  user,
	})
}

func (s *Server) meHandler(w http.ResponseWriter, r *http.Request) {
	user := auth.GetUserFromContext(r.Context())
	if user == nil {
		respondError(w, http.StatusUnauthorized, "Unauthorized")
		return
	}

	respondJSON(w, http.StatusOK, user)
}

// parseJSON decodes a request body of at most maxJSONBodySize bytes. Larger
// bodies are rejected up front instead of being buffered in full (DoS guard).
func parseJSON(w http.ResponseWriter, r *http.Request, v interface{}) error {
	r.Body = http.MaxBytesReader(w, r.Body, maxJSONBodySize)
	dec := json.NewDecoder(r.Body)
	return dec.Decode(v)
}

// clientKey identifies the rate-limit subject for a request: the real client
// IP when behind a trusted proxy (X-Forwarded-For set by middleware.RealIP)
// or the connection peer otherwise.
func clientKey(r *http.Request) string {
	if host, _, err := net.SplitHostPort(r.RemoteAddr); err == nil {
		return host
	}
	return r.RemoteAddr
}
