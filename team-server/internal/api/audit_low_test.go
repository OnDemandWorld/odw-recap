package api

import (
	"net/http"
	"strings"
	"testing"

	"github.com/ondemandworld/recap-team-server/internal/models"
)

// Audit #74: an over-length email must be a 400 validation error, not a 500.
func TestValidateRegister_EmailTooLong(t *testing.T) {
	req := &RegisterRequest{
		Email:    strings.Repeat("a", 245) + "@example.com", // 257 chars > VARCHAR(255)
		Password: "password123",
		Name:     "Long Email",
	}
	if msg := validateRegisterRequest(req); msg != "Email is too long" {
		t.Errorf("expected 'Email is too long', got %q", msg)
	}
}

// Audit #75: fixed-width meeting fields are bounded so bad input is a 400.
func TestValidateMeetingFields(t *testing.T) {
	longType := strings.Repeat("x", 101)
	longLang := strings.Repeat("x", 11)
	longTitle := strings.Repeat("x", 1001)
	okType := "standup"
	okLang := "en"
	okTitle := "Weekly sync"

	cases := []struct {
		name    string
		meeting models.Meeting
		want    string
	}{
		{"valid", models.Meeting{MeetingType: &okType, Language: &okLang, Title: &okTitle}, ""},
		{"type too long", models.Meeting{MeetingType: &longType}, "meeting_type is too long (max 100 characters)"},
		{"language too long", models.Meeting{Language: &longLang}, "language is too long (max 10 characters)"},
		{"title too long", models.Meeting{Title: &longTitle}, "title is too long (max 1000 characters)"},
	}
	for _, c := range cases {
		if got := validateMeetingFields(&c.meeting); got != c.want {
			t.Errorf("%s: got %q, want %q", c.name, got, c.want)
		}
	}
}

// Audit #75: the create endpoint rejects over-length fields with 400.
func TestCreateMeeting_RejectsOverLengthLanguage(t *testing.T) {
	s, _, _ := newOwnershipTestServer()
	router := s.Router()
	tokenA := tokenFor(t, userA, "a@example.com", "member")

	rr := doRequest(t, router, "POST", "/meetings/", tokenA, map[string]interface{}{
		"title":    "ok",
		"language": strings.Repeat("x", 11),
	})
	if rr.Code != http.StatusBadRequest {
		t.Errorf("expected 400 for over-length language, got %d: %s", rr.Code, rr.Body.String())
	}
}

// Audit #77: clientIP strips the port and keeps IPv6 hosts within VARCHAR(45).
func TestClientIP_StripsPortAndHandlesIPv6(t *testing.T) {
	cases := []struct{ in, want string }{
		{"192.0.2.1:1234", "192.0.2.1"},
		{"[2001:db8:85a3:8d3:1319:8a2e:370:7348]:443", "2001:db8:85a3:8d3:1319:8a2e:370:7348"},
		{"bare-host", "bare-host"},
	}
	for _, c := range cases {
		got := clientIP(c.in)
		if got != c.want {
			t.Errorf("clientIP(%q) = %q, want %q", c.in, got, c.want)
		}
		if len(got) > 45 {
			t.Errorf("clientIP(%q) = %q exceeds VARCHAR(45)", c.in, got)
		}
	}
}
