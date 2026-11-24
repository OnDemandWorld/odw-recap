package sync

import (
	"testing"
	"time"
)

// Regression for audit #23: retryDelay must never panic on a negative shift.
// The undecodable-payload branch settles stranded 'pending' rows whose
// retryCount is 0, which previously computed baseRetryDelay << -1 and crashed
// the whole server.
func TestRetryDelay_ClampsNonPositiveCounts(t *testing.T) {
	cases := []struct {
		retryCount int
		want       time.Duration
	}{
		{0, baseRetryDelay},      // stranded pending row: shift clamped to 0
		{-3, baseRetryDelay},     // defensive: unexpected negative count
		{1, baseRetryDelay},      // first retry: base * 2^0
		{2, 2 * baseRetryDelay},  // base * 2^1
		{3, 4 * baseRetryDelay},  // base * 2^2
		{5, 16 * baseRetryDelay}, // base * 2^4
	}
	for _, c := range cases {
		got := retryDelay(c.retryCount) // must not panic
		if got != c.want {
			t.Errorf("retryDelay(%d) = %s, want %s", c.retryCount, got, c.want)
		}
	}
}

// A large count is capped so the shift cannot overflow into a negative duration.
func TestRetryDelay_CapsLargeCounts(t *testing.T) {
	got := retryDelay(1000)
	if want := baseRetryDelay << 16; got != want {
		t.Errorf("retryDelay(1000) = %s, want capped %s", got, want)
	}
	if got <= 0 {
		t.Errorf("retryDelay must stay positive, got %s", got)
	}
}
