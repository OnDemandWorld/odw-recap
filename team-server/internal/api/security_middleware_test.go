package api

import (
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/alicebob/miniredis/v2"

	"github.com/ondemandworld/recap-team-server/internal/db"
)

func passthrough() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	}
}

func doRequest(t *testing.T, handler http.Handler, path, remoteAddr string) int {
	t.Helper()
	req := httptest.NewRequest("POST", path, nil)
	req.RemoteAddr = remoteAddr
	rr := httptest.NewRecorder()
	handler.ServeHTTP(rr, req)
	return rr.Code
}

func TestRateLimitBlocksAfterLimit(t *testing.T) {
	mr := miniredis.RunT(t)
	server := NewServer(nil, db.NewRedis(mr.Addr()), "secret")
	handler := server.rateLimit(3, time.Minute)(passthrough())

	for i := 0; i < 3; i++ {
		if code := doRequest(t, handler, "/auth/login", "192.0.2.1:1234"); code != http.StatusOK {
			t.Fatalf("request %d: expected 200, got %d", i+1, code)
		}
	}
	if code := doRequest(t, handler, "/auth/login", "192.0.2.1:1234"); code != http.StatusTooManyRequests {
		t.Fatalf("expected 429 after limit, got %d", code)
	}

	// A different client IP is unaffected.
	if code := doRequest(t, handler, "/auth/login", "192.0.2.2:1234"); code != http.StatusOK {
		t.Fatalf("different IP should pass, got %d", code)
	}

	// A different path has its own budget.
	if code := doRequest(t, handler, "/auth/register", "192.0.2.1:1234"); code != http.StatusOK {
		t.Fatalf("different path should pass, got %d", code)
	}
}

func TestRateLimitWindowResets(t *testing.T) {
	mr := miniredis.RunT(t)
	server := NewServer(nil, db.NewRedis(mr.Addr()), "secret")
	handler := server.rateLimit(1, time.Minute)(passthrough())

	if code := doRequest(t, handler, "/auth/login", "192.0.2.1:1234"); code != http.StatusOK {
		t.Fatalf("first request should pass, got %d", code)
	}
	if code := doRequest(t, handler, "/auth/login", "192.0.2.1:1234"); code != http.StatusTooManyRequests {
		t.Fatalf("second request should be limited, got %d", code)
	}

	// Advance time past the window: the budget resets.
	mr.FastForward(61 * time.Second)
	if code := doRequest(t, handler, "/auth/login", "192.0.2.1:1234"); code != http.StatusOK {
		t.Fatalf("request after window should pass, got %d", code)
	}
}

func TestRateLimitFailsOpenWithoutRedis(t *testing.T) {
	server := NewServer(nil, nil, "secret")
	handler := server.rateLimit(1, time.Minute)(passthrough())

	for i := 0; i < 5; i++ {
		if code := doRequest(t, handler, "/auth/login", "192.0.2.1:1234"); code != http.StatusOK {
			t.Fatalf("nil redis should fail open, got %d", code)
		}
	}
}
