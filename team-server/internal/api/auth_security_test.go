package api

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/google/uuid"
)

func TestValidateRegisterRequest(t *testing.T) {
	cases := []struct {
		name    string
		email   string
		pass    string
		person  string
		wantErr bool
	}{
		{"valid", "User@Example.com", "longenough1", " Alice ", false},
		{"missing email", "", "longenough1", "Alice", true},
		{"missing password", "a@b.co", "", "Alice", true},
		{"missing name", "a@b.co", "longenough1", "  ", true},
		{"bad email", "not-an-email", "longenough1", "Alice", true},
		{"short password", "a@b.co", "short", "Alice", true},
		{"oversize password", "a@b.co", strings.Repeat("x", 73), "Alice", true},
	}
	for _, tc := range cases {
		req := &RegisterRequest{Email: tc.email, Password: tc.pass, Name: tc.person}
		msg := validateRegisterRequest(req)
		if tc.wantErr && msg == "" {
			t.Errorf("%s: expected error, got none", tc.name)
		}
		if !tc.wantErr && msg != "" {
			t.Errorf("%s: unexpected error: %s", tc.name, msg)
		}
	}
	// Normalization: trimmed and lowercased.
	req := &RegisterRequest{Email: "  Bob@Example.COM ", Password: "longenough1", Name: " Bob "}
	validateRegisterRequest(req)
	if req.Email != "bob@example.com" || req.Name != "Bob" {
		t.Errorf("normalization failed: email=%q name=%q", req.Email, req.Name)
	}
}

func TestLoginRateLimited(t *testing.T) {
	s := NewServer(nil, nil, "0123456789abcdef0123456789abcdef")
	router := s.Router()

	// With no database wired, in-budget attempts fail with 500 (recovered
	// panic) — the point here is they are NOT throttled (429).
	body := `{"email":"nobody@example.com","password":"wrongpass1"}`
	for i := 0; i < loginRateLimit; i++ {
		req := httptest.NewRequest(http.MethodPost, "/auth/login", strings.NewReader(body))
		req.RemoteAddr = "203.0.113.9:1234"
		rec := httptest.NewRecorder()
		router.ServeHTTP(rec, req)
		if rec.Code == http.StatusTooManyRequests {
			t.Fatalf("attempt %d throttled before budget exhausted", i+1)
		}
	}

	req := httptest.NewRequest(http.MethodPost, "/auth/login", strings.NewReader(body))
	req.RemoteAddr = "203.0.113.9:1234"
	rec := httptest.NewRecorder()
	router.ServeHTTP(rec, req)
	if rec.Code != http.StatusTooManyRequests {
		t.Fatalf("over-budget attempt: want 429, got %d", rec.Code)
	}

	// A different client IP still gets through (not throttled).
	req = httptest.NewRequest(http.MethodPost, "/auth/login", strings.NewReader(body))
	req.RemoteAddr = "198.51.100.7:9999"
	rec = httptest.NewRecorder()
	router.ServeHTTP(rec, req)
	if rec.Code == http.StatusTooManyRequests {
		t.Fatal("other client wrongly throttled")
	}
}

func TestLoginBodyTooLargeRejected(t *testing.T) {
	s := NewServer(nil, nil, "0123456789abcdef0123456789abcdef")
	router := s.Router()

	huge := `{"email":"` + strings.Repeat("a", maxJSONBodySize) + `"}`
	req := httptest.NewRequest(http.MethodPost, "/auth/login", strings.NewReader(huge))
	rec := httptest.NewRecorder()
	router.ServeHTTP(rec, req)
	if rec.Code != http.StatusBadRequest {
		t.Fatalf("oversized body: want 400, got %d", rec.Code)
	}
}

func TestAuthErrorsAreJSON(t *testing.T) {
	s := NewServer(nil, nil, "0123456789abcdef0123456789abcdef")
	router := s.Router()

	cases := []struct {
		name   string
		req    *http.Request
		status int
	}{
		{"missing header", httptest.NewRequest(http.MethodGet, "/me", nil), http.StatusUnauthorized},
		{"bad scheme", withAuthHeader(httptest.NewRequest(http.MethodGet, "/me", nil), "Token abc"), http.StatusUnauthorized},
		{"bad token", withAuthHeader(httptest.NewRequest(http.MethodGet, "/me", nil), "Bearer not-a-jwt"), http.StatusUnauthorized},
	}
	for _, tc := range cases {
		rec := httptest.NewRecorder()
		router.ServeHTTP(rec, tc.req)
		if rec.Code != tc.status {
			t.Errorf("%s: want %d, got %d", tc.name, tc.status, rec.Code)
		}
		if ct := rec.Header().Get("Content-Type"); !strings.HasPrefix(ct, "application/json") {
			t.Errorf("%s: want JSON content type, got %q (body=%q)", tc.name, ct, rec.Body.String())
		}
		if !strings.Contains(rec.Body.String(), `"error"`) {
			t.Errorf("%s: body %q lacks error field", tc.name, rec.Body.String())
		}
	}
}

func TestRequireRoleReturnsJSONForbidden(t *testing.T) {
	s := NewServer(nil, nil, "0123456789abcdef0123456789abcdef")
	tok, err := s.jwtManager.GenerateToken(testUUID(), "member@example.com", "member")
	if err != nil {
		t.Fatal(err)
	}
	req := withAuthHeader(httptest.NewRequest(http.MethodGet, "/admin/users", nil), "Bearer "+tok)
	rec := httptest.NewRecorder()
	s.Router().ServeHTTP(rec, req)
	if rec.Code != http.StatusForbidden {
		t.Fatalf("member hitting admin route: want 403, got %d", rec.Code)
	}
	if ct := rec.Header().Get("Content-Type"); !strings.HasPrefix(ct, "application/json") {
		t.Errorf("want JSON content type, got %q", ct)
	}
}

func withAuthHeader(r *http.Request, v string) *http.Request {
	r.Header.Set("Authorization", v)
	return r
}

func TestCORSDoesNotAllowCredentials(t *testing.T) {
	s := NewServer(nil, nil, "0123456789abcdef0123456789abcdef")
	router := s.Router()

	req := httptest.NewRequest(http.MethodOptions, "/health", nil)
	req.Header.Set("Origin", "https://evil.example")
	req.Header.Set("Access-Control-Request-Method", "GET")
	rec := httptest.NewRecorder()
	router.ServeHTTP(rec, req)

	if got := rec.Header().Get("Access-Control-Allow-Credentials"); got == "true" {
		t.Error("Access-Control-Allow-Credentials must never be true")
	}
}

func TestSetAllowedOrigins(t *testing.T) {
	s := NewServer(nil, nil, "0123456789abcdef0123456789abcdef")
	s.SetAllowedOrigins([]string{"https://recap.example.com"})
	router := s.Router()

	req := httptest.NewRequest(http.MethodOptions, "/health", nil)
	req.Header.Set("Origin", "https://recap.example.com")
	req.Header.Set("Access-Control-Request-Method", "GET")
	rec := httptest.NewRecorder()
	router.ServeHTTP(rec, req)
	if got := rec.Header().Get("Access-Control-Allow-Origin"); got != "https://recap.example.com" {
		t.Errorf("configured origin not echoed, got %q", got)
	}

	req = httptest.NewRequest(http.MethodOptions, "/health", nil)
	req.Header.Set("Origin", "https://evil.example")
	req.Header.Set("Access-Control-Request-Method", "GET")
	rec = httptest.NewRecorder()
	router.ServeHTTP(rec, req)
	if got := rec.Header().Get("Access-Control-Allow-Origin"); got != "" {
		t.Errorf("unconfigured origin should not be echoed, got %q", got)
	}
}

// testUUID returns a fixed valid UUID for token generation in tests.
func testUUID() uuid.UUID {
	return uuid.MustParse("11111111-1111-1111-1111-111111111111")
}
