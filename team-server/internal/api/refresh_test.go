package api

import (
	"strings"
	"testing"
	"time"

	"github.com/alicebob/miniredis/v2"
	"github.com/google/uuid"

	"github.com/ondemandworld/recap-team-server/internal/auth"
	"github.com/ondemandworld/recap-team-server/internal/db"
)

func newRedisTestServer(t *testing.T) (*Server, *miniredis.Miniredis) {
	t.Helper()
	mr := miniredis.RunT(t)
	return NewServer(nil, db.NewRedis(mr.Addr()), "secret"), mr
}

func TestRefreshTokenLifecycle(t *testing.T) {
	server, mr := newRedisTestServer(t)
	userID := uuid.New()

	token, err := server.storeRefresh(userID)
	if err != nil {
		t.Fatalf("storeRefresh failed: %v", err)
	}
	if token == "" {
		t.Fatal("expected non-empty refresh token")
	}

	// The raw token must never appear in Redis — keys are hashes, values are
	// user IDs.
	for _, key := range mr.Keys() {
		if strings.Contains(key, token) {
			t.Fatalf("raw refresh token leaked into redis key %q", key)
		}
		if value, err := mr.Get(key); err == nil && strings.Contains(value, token) {
			t.Fatalf("raw refresh token leaked into redis value for key %q", key)
		}
	}

	got, err := server.lookupRefresh(token)
	if err != nil {
		t.Fatalf("lookupRefresh failed: %v", err)
	}
	if got != userID {
		t.Fatalf("expected user %s, got %s", userID, got)
	}

	// Rotation: revoking the token makes it unusable.
	if err := server.revokeRefresh(token); err != nil {
		t.Fatalf("revokeRefresh failed: %v", err)
	}
	if _, err := server.lookupRefresh(token); err == nil {
		t.Fatal("revoked token must not validate")
	}

	// Unknown tokens fail too.
	if _, err := server.lookupRefresh("bogus-token"); err == nil {
		t.Fatal("unknown token must not validate")
	}
}

func TestRefreshTokenExpiry(t *testing.T) {
	server, mr := newRedisTestServer(t)
	userID := uuid.New()

	token, err := server.storeRefresh(userID)
	if err != nil {
		t.Fatalf("storeRefresh failed: %v", err)
	}

	// Just before expiry the token still works.
	mr.FastForward(RefreshTokenTTL - time.Minute)
	if _, err := server.lookupRefresh(token); err != nil {
		t.Fatalf("token should still be valid before expiry: %v", err)
	}

	// Past expiry it is gone.
	mr.FastForward(2 * time.Minute)
	if _, err := server.lookupRefresh(token); err == nil {
		t.Fatal("expired token must not validate")
	}
}

func TestAccessTokenTTLIsShortLived(t *testing.T) {
	jm := auth.NewJWTManager("secret")
	token, err := jm.GenerateToken(uuid.New(), "user@example.com", "member")
	if err != nil {
		t.Fatalf("GenerateToken failed: %v", err)
	}

	claims, err := jm.ValidateToken(token)
	if err != nil {
		t.Fatalf("ValidateToken failed: %v", err)
	}

	lifetime := claims.ExpiresAt.Time.Sub(claims.IssuedAt.Time)
	if lifetime != auth.AccessTokenTTL {
		t.Fatalf("expected access token TTL %s, got %s", auth.AccessTokenTTL, lifetime)
	}
}
