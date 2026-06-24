package api

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/google/uuid"
	"github.com/ondemandworld/recap-team-server/internal/auth"
)

func setupTestServer() *Server {
	// Note: In production, use a real database. For unit tests without DB,
	// we pass nil and only test routes that don't require database access.
	return NewServer(nil, nil, "test-secret")
}

func TestHealthHandler(t *testing.T) {
	s := setupTestServer()
	router := s.Router()

	req := httptest.NewRequest("GET", "/health", nil)
	rr := httptest.NewRecorder()

	router.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status %d, got %d", http.StatusOK, rr.Code)
	}
}

func TestJWTManagerInvalidToken(t *testing.T) {
	jwtManager := auth.NewJWTManager("test-secret")

	token := "invalid-token"
	_, err := jwtManager.ValidateToken(token)
	if err == nil {
		t.Error("Expected error for invalid token")
	}
}

func TestJWTManagerValidToken(t *testing.T) {
	jwtManager := auth.NewJWTManager("test-secret")

	// Generate a valid token
	token, err := jwtManager.GenerateToken(userID(), "test@example.com", "member")
	if err != nil {
		t.Fatalf("Failed to generate token: %v", err)
	}

	claims, err := jwtManager.ValidateToken(token)
	if err != nil {
		t.Fatalf("Failed to validate valid token: %v", err)
	}

	if claims.Email != "test@example.com" {
		t.Errorf("Expected email 'test@example.com', got '%s'", claims.Email)
	}
}

func userID() uuid.UUID {
	id, err := uuid.Parse("12345678-1234-1234-1234-123456789abc")
	if err != nil {
		panic(err)
	}
	return id
}
