package api

import (
	"database/sql"
	"encoding/json"
	"errors"
	"net/http"
	"net/mail"
	"strings"
	"time"

	"github.com/google/uuid"
	"github.com/lib/pq"
	"github.com/ondemandworld/recap-team-server/internal/auth"
	"github.com/ondemandworld/recap-team-server/internal/models"
	"golang.org/x/crypto/bcrypt"
)

// bcrypt only uses the first 72 bytes of a password; reject longer input
// rather than silently truncating it.
const maxPasswordBytes = 72
const minPasswordLength = 8

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

// AuthResponse represents an authentication response. Token is a short-lived
// access token; RefreshToken rotates on every use (see refreshHandler).
type AuthResponse struct {
	Token        string      `json:"token"`
	RefreshToken string      `json:"refresh_token"`
	ExpiresIn    int64       `json:"expires_in"`
	User         models.User `json:"user"`
}

func (s *Server) registerHandler(w http.ResponseWriter, r *http.Request) {
	var req RegisterRequest
	if err := parseJSON(r, &req); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	if req.Email == "" || req.Password == "" || req.Name == "" {
		respondError(w, http.StatusBadRequest, "Email, password, and name are required")
		return
	}

	req.Email = strings.ToLower(strings.TrimSpace(req.Email))
	req.Name = strings.TrimSpace(req.Name)

	if _, err := mail.ParseAddress(req.Email); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid email address")
		return
	}
	if req.Name == "" {
		respondError(w, http.StatusBadRequest, "Name must not be empty")
		return
	}
	if len(req.Password) < minPasswordLength {
		respondError(w, http.StatusBadRequest, "Password must be at least 8 characters")
		return
	}
	if len(req.Password) > maxPasswordBytes {
		respondError(w, http.StatusBadRequest, "Password is too long")
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
		if errors.As(err, &pqErr) && pqErr.Code == "23505" { // unique_violation
			respondError(w, http.StatusConflict, "A user with this email already exists")
			return
		}
		respondError(w, http.StatusInternalServerError, "Failed to create user")
		return
	}

	// Generate tokens
	token, err := s.jwtManager.GenerateToken(user.ID, user.Email, user.Role)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to generate token")
		return
	}
	refreshToken, err := s.storeRefresh(user.ID)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to issue refresh token")
		return
	}

	s.writeAudit(r, &user.ID, "register", strPtr("auth"), "user registered")

	respondJSON(w, http.StatusCreated, AuthResponse{
		Token:        token,
		RefreshToken: refreshToken,
		ExpiresIn:    int64(auth.AccessTokenTTL.Seconds()),
		User:         *user,
	})
}

func (s *Server) loginHandler(w http.ResponseWriter, r *http.Request) {
	var req LoginRequest
	if err := parseJSON(r, &req); err != nil {
		respondError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	// Find user
	var user models.User
	var passwordHash string
	err := s.db.DB().QueryRow(
		"SELECT id, email, password_hash, name, role, created_at, updated_at FROM users WHERE email = $1",
		req.Email,
	).Scan(&user.ID, &user.Email, &passwordHash, &user.Name, &user.Role, &user.CreatedAt, &user.UpdatedAt)

	if err == sql.ErrNoRows {
		s.writeAudit(r, nil, "login_failed", strPtr("auth"), "unknown email: "+req.Email)
		respondError(w, http.StatusUnauthorized, "Invalid credentials")
		return
	} else if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to query user")
		return
	}

	// Verify password
	if err := bcrypt.CompareHashAndPassword([]byte(passwordHash), []byte(req.Password)); err != nil {
		s.writeAudit(r, &user.ID, "login_failed", strPtr("auth"), "bad password")
		respondError(w, http.StatusUnauthorized, "Invalid credentials")
		return
	}

	// Generate tokens
	token, err := s.jwtManager.GenerateToken(user.ID, user.Email, user.Role)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to generate token")
		return
	}
	refreshToken, err := s.storeRefresh(user.ID)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to issue refresh token")
		return
	}

	s.writeAudit(r, &user.ID, "login", strPtr("auth"), "login successful")

	respondJSON(w, http.StatusOK, AuthResponse{
		Token:        token,
		RefreshToken: refreshToken,
		ExpiresIn:    int64(auth.AccessTokenTTL.Seconds()),
		User:         user,
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

func parseJSON(r *http.Request, v interface{}) error {
	r.Body = http.MaxBytesReader(nil, r.Body, maxJSONBodyBytes)
	return json.NewDecoder(r.Body).Decode(v)
}
