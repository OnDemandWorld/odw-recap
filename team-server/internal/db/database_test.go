package db

import (
	"errors"
	"strings"
	"testing"
)

func TestDSNHasSSLMode(t *testing.T) {
	tests := []struct {
		name string
		dsn  string
		want bool
	}{
		{"url form disable", "postgres://u:p@localhost:5432/recap?sslmode=disable", true},
		{"url form require", "postgres://u:p@localhost:5432/recap?sslmode=require", true},
		{"keyword form", "host=localhost user=recap sslmode=disable", true},
		{"case insensitive", "postgres://u:p@localhost/recap?SSLMODE=disable", true},
		{"no sslmode url", "postgres://u:p@localhost:5432/recap", false},
		{"no sslmode keyword", "host=localhost user=recap dbname=recap", false},
		{"empty", "", false},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := dsnHasSSLMode(tt.dsn); got != tt.want {
				t.Errorf("dsnHasSSLMode(%q) = %v, want %v", tt.dsn, got, tt.want)
			}
		})
	}
}

func TestAnnotateSSLError(t *testing.T) {
	sslErr := errors.New("pq: SSL is not enabled on the server")
	otherErr := errors.New("connection refused")

	tests := []struct {
		name      string
		err       error
		dsn       string
		wantNil   bool
		wantHint  bool
		wantUnwrap bool
	}{
		{"nil error", nil, "postgres://x", true, false, false},
		{"ssl error without sslmode gets hint", sslErr, "postgres://u:p@localhost/recap", false, true, true},
		{"ssl error with sslmode untouched", sslErr, "postgres://u:p@localhost/recap?sslmode=disable", false, false, true},
		{"protocol error without sslmode gets hint", errors.New("pq: unsupported frontend protocol"), "postgres://u:p@localhost/recap", false, true, true},
		{"unrelated error untouched", otherErr, "postgres://u:p@localhost/recap", false, false, true},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := annotateSSLError(tt.err, tt.dsn)
			if tt.wantNil {
				if got != nil {
					t.Fatalf("expected nil, got %v", got)
				}
				return
			}
			if got == nil {
				t.Fatal("expected non-nil error")
			}
			if tt.wantHint && !strings.Contains(got.Error(), "sslmode=disable") {
				t.Errorf("expected actionable hint in %q", got.Error())
			}
			if !tt.wantHint && strings.Contains(got.Error(), "append ?sslmode") {
				t.Errorf("unexpected hint in %q", got.Error())
			}
			if tt.wantUnwrap && !errors.Is(got, tt.err) {
				t.Error("annotated error must still unwrap to the original")
			}
		})
	}
}
