package auth

import (
	"context"
	"net/http"
	"strconv"
	"strings"

	"github.com/ondemandworld/recap-team-server/internal/models"
)

type contextKey string

const userContextKey contextKey = "user"

// Middleware is JWT authentication middleware
func Middleware(jwtManager *JWTManager) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			respondJSONError := func(message string, status int) {
				w.Header().Set("Content-Type", "application/json")
				w.WriteHeader(status)
				_, _ = w.Write([]byte(`{"error":` + strconv.Quote(message) + `}`))
			}

			authHeader := strings.TrimSpace(r.Header.Get("Authorization"))
			if authHeader == "" {
				respondJSONError("Authorization header required", http.StatusUnauthorized)
				return
			}

			// Extract token from "Bearer <token>" (any whitespace between
			// scheme and token, and the token itself, may not contain spaces).
			scheme, tokenString, found := strings.Cut(authHeader, " ")
			if !found || scheme != "Bearer" || strings.TrimSpace(tokenString) == "" {
				respondJSONError("Invalid authorization header format", http.StatusUnauthorized)
				return
			}

			claims, err := jwtManager.ValidateToken(strings.TrimSpace(tokenString))
			if err != nil {
				respondJSONError("Invalid token", http.StatusUnauthorized)
				return
			}

			// Add user to context
			user := &models.User{
				ID:    claims.UserID,
				Email: claims.Email,
				Role:  claims.Role,
			}

			ctx := context.WithValue(r.Context(), userContextKey, user)
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

// GetUserFromContext retrieves user from context
func GetUserFromContext(ctx context.Context) *models.User {
	if user, ok := ctx.Value(userContextKey).(*models.User); ok {
		return user
	}
	return nil
}

// RequireRole middleware checks if user has required role
func RequireRole(roles ...string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			respondJSONError := func(message string, status int) {
				w.Header().Set("Content-Type", "application/json")
				w.WriteHeader(status)
				_, _ = w.Write([]byte(`{"error":` + strconv.Quote(message) + `}`))
			}

			user := GetUserFromContext(r.Context())
			if user == nil {
				respondJSONError("Unauthorized", http.StatusUnauthorized)
				return
			}

			hasRole := false
			for _, role := range roles {
				if user.Role == role {
					hasRole = true
					break
				}
			}

			if !hasRole {
				respondJSONError("Forbidden", http.StatusForbidden)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}
