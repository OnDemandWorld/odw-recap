package api

import (
	"database/sql"
	"encoding/json"
	"net/http"
	"time"

	"github.com/google/uuid"
	"golang.org/x/crypto/bcrypt"
	"github.com/ondemandworld/recap-team-server/internal/auth"
	"github.com/ondemandworld/recap-team-server/internal/models"
)

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
	Token string       `json:"token"`
	User  models.User  `json:"user"`
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

func parseJSON(r *http.Request, v interface{}) error {
	return json.NewDecoder(r.Body).Decode(v)
}
