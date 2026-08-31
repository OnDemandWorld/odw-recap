package auth

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/google/uuid"
)

func okHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
	}
}

func TestMiddlewareMissingHeader(t *testing.T) {
	mw := Middleware(NewJWTManager("secret"))
	req := httptest.NewRequest("GET", "/me", nil)
	rr := httptest.NewRecorder()

	mw(okHandler()).ServeHTTP(rr, req)

	if rr.Code != http.StatusUnauthorized {
		t.Errorf("expected 401, got %d", rr.Code)
	}
}

func TestMiddlewareMalformedHeader(t *testing.T) {
	mw := Middleware(NewJWTManager("secret"))

	for _, header := range []string{"Bearer", "Bearer a b", "Basic abc123"} {
		req := httptest.NewRequest("GET", "/me", nil)
		req.Header.Set("Authorization", header)
		rr := httptest.NewRecorder()

		mw(okHandler()).ServeHTTP(rr, req)

		if rr.Code != http.StatusUnauthorized {
			t.Errorf("header %q: expected 401, got %d", header, rr.Code)
		}
	}
}

func TestMiddlewareValidToken(t *testing.T) {
	jm := NewJWTManager("secret")
	token, err := jm.GenerateToken(uuid.New(), "user@example.com", "member")
	if err != nil {
		t.Fatalf("failed to generate token: %v", err)
	}

	var capturedEmail, capturedRole string
	inner := http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		user := GetUserFromContext(r.Context())
		if user == nil {
			t.Fatal("expected user in context")
		}
		capturedEmail = user.Email
		capturedRole = user.Role
		w.WriteHeader(http.StatusOK)
	})

	req := httptest.NewRequest("GET", "/me", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	rr := httptest.NewRecorder()

	Middleware(jm)(inner).ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("expected 200, got %d", rr.Code)
	}
	if capturedEmail != "user@example.com" || capturedRole != "member" {
		t.Errorf("unexpected user in context: %s / %s", capturedEmail, capturedRole)
	}
}

func TestMiddlewareWrongSecret(t *testing.T) {
	issuer := NewJWTManager("secret-a")
	verifier := NewJWTManager("secret-b")

	token, err := issuer.GenerateToken(uuid.New(), "user@example.com", "member")
	if err != nil {
		t.Fatalf("failed to generate token: %v", err)
	}

	req := httptest.NewRequest("GET", "/me", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	rr := httptest.NewRecorder()

	Middleware(verifier)(okHandler()).ServeHTTP(rr, req)

	if rr.Code != http.StatusUnauthorized {
		t.Errorf("expected 401 for token signed with different secret, got %d", rr.Code)
	}
}

func TestRequireRole(t *testing.T) {
	jm := NewJWTManager("secret")

	adminToken, _ := jm.GenerateToken(uuid.New(), "admin@example.com", "admin")
	memberToken, _ := jm.GenerateToken(uuid.New(), "member@example.com", "member")

	stack := Middleware(jm)(RequireRole("admin")(okHandler()))

	tests := []struct {
		name     string
		token    string
		expected int
	}{
		{"admin allowed", adminToken, http.StatusOK},
		{"member forbidden", memberToken, http.StatusForbidden},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", "/admin/stats", nil)
			req.Header.Set("Authorization", "Bearer "+tc.token)
			rr := httptest.NewRecorder()

			stack.ServeHTTP(rr, req)

			if rr.Code != tc.expected {
				t.Errorf("expected %d, got %d", tc.expected, rr.Code)
			}
		})
	}
}

func TestRequireRoleNoUser(t *testing.T) {
	req := httptest.NewRequest("GET", "/admin/stats", nil)
	rr := httptest.NewRecorder()

	RequireRole("admin")(okHandler()).ServeHTTP(rr, req)

	if rr.Code != http.StatusUnauthorized {
		t.Errorf("expected 401 without user in context, got %d", rr.Code)
	}
}
