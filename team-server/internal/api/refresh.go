package api

import (
	"errors"
	"net/http"
	"time"

	"github.com/google/uuid"
	"github.com/redis/go-redis/v9"

	"github.com/ondemandworld/recap-team-server/internal/auth"
	"github.com/ondemandworld/recap-team-server/internal/models"
)

// RefreshTokenTTL bounds the lifetime of a refresh token. Tokens are rotated
// on every use, so this is the maximum idle window, not a session cap.
const RefreshTokenTTL = 30 * 24 * time.Hour

func refreshKey(token string) string {
	return "auth:refresh:" + auth.HashToken(token)
}

// storeRefresh creates a new refresh token for the user and persists its hash.
func (s *Server) storeRefresh(userID uuid.UUID) (string, error) {
	token := auth.GenerateRefreshToken()
	if err := s.redis.Set(refreshKey(token), userID.String(), RefreshTokenTTL); err != nil {
		return "", err
	}
	return token, nil
}

// lookupRefresh returns the user ID a refresh token belongs to.
func (s *Server) lookupRefresh(token string) (uuid.UUID, error) {
	value, err := s.redis.Get(refreshKey(token))
	if err != nil {
		if errors.Is(err, redis.Nil) {
			return uuid.Nil, errors.New("invalid or expired refresh token")
		}
		return uuid.Nil, err
	}
	return uuid.Parse(value)
}

// revokeRefresh invalidates a refresh token. Revoked tokens cannot be
// distinguished from unknown ones, which is intentional.
func (s *Server) revokeRefresh(token string) error {
	return s.redis.Delete(refreshKey(token))
}

type refreshRequest struct {
	RefreshToken string `json:"refresh_token"`
}

// refreshHandler rotates a refresh token: the presented token is revoked and
// a new access/refresh pair is issued. Replaying an old refresh token fails.
func (s *Server) refreshHandler(w http.ResponseWriter, r *http.Request) {
	var req refreshRequest
	if err := parseJSON(r, &req); err != nil || req.RefreshToken == "" {
		respondError(w, http.StatusBadRequest, "refresh_token is required")
		return
	}

	userID, err := s.lookupRefresh(req.RefreshToken)
	if err != nil {
		respondError(w, http.StatusUnauthorized, "Invalid or expired refresh token")
		return
	}

	// Re-check the user against the database: deleted or disabled accounts
	// must not be able to refresh even with a valid token.
	var email, role string
	err = s.db.DB().QueryRow("SELECT email, role FROM users WHERE id = $1", userID).Scan(&email, &role)
	if err != nil {
		_ = s.revokeRefresh(req.RefreshToken)
		respondError(w, http.StatusUnauthorized, "Invalid or expired refresh token")
		return
	}

	// Rotate: revoke the presented token, then issue a new pair.
	_ = s.revokeRefresh(req.RefreshToken)
	newRefresh, err := s.storeRefresh(userID)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to issue refresh token")
		return
	}

	accessToken, err := s.jwtManager.GenerateToken(userID, email, role)
	if err != nil {
		respondError(w, http.StatusInternalServerError, "Failed to generate token")
		return
	}

	s.writeAudit(r, &userID, "token_refresh", strPtr("auth"), "refresh token rotated")

	respondJSON(w, http.StatusOK, AuthResponse{
		Token:        accessToken,
		RefreshToken: newRefresh,
		ExpiresIn:    int64(auth.AccessTokenTTL.Seconds()),
		User:         models.User{ID: userID, Email: email, Role: role},
	})
}

// logoutHandler revokes the presented refresh token.
func (s *Server) logoutHandler(w http.ResponseWriter, r *http.Request) {
	var req refreshRequest
	if err := parseJSON(r, &req); err != nil || req.RefreshToken == "" {
		respondError(w, http.StatusBadRequest, "refresh_token is required")
		return
	}

	userID, err := s.lookupRefresh(req.RefreshToken)
	if err == nil {
		s.writeAudit(r, &userID, "logout", strPtr("auth"), "refresh token revoked")
	}

	// Revoking an unknown token is not an error: logout must be idempotent.
	_ = s.revokeRefresh(req.RefreshToken)
	respondJSON(w, http.StatusOK, map[string]string{"status": "logged_out"})
}

func strPtr(s string) *string { return &s }
