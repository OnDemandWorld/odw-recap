package api

import (
	"testing"
	"time"
)

func TestRateLimiterAllowsWithinBudget(t *testing.T) {
	l := newRateLimiter(3, time.Minute)
	for i := 0; i < 3; i++ {
		if !l.Allow("ip|user@example.com") {
			t.Fatalf("attempt %d within budget was rejected", i+1)
		}
	}
	if l.Allow("ip|user@example.com") {
		t.Fatal("attempt beyond budget was accepted")
	}
}

func TestRateLimiterKeysAreIndependent(t *testing.T) {
	l := newRateLimiter(1, time.Minute)
	if !l.Allow("ip1|a@example.com") {
		t.Fatal("first attempt for key1 rejected")
	}
	if !l.Allow("ip2|a@example.com") {
		t.Fatal("different key should have its own budget")
	}
	if l.Allow("ip1|a@example.com") {
		t.Fatal("key1 beyond budget accepted")
	}
}

func TestRateLimiterWindowReset(t *testing.T) {
	l := newRateLimiter(1, 5*time.Millisecond)
	if !l.Allow("k") {
		t.Fatal("first attempt rejected")
	}
	if l.Allow("k") {
		t.Fatal("second immediate attempt accepted")
	}
	time.Sleep(10 * time.Millisecond)
	if !l.Allow("k") {
		t.Fatal("attempt after window expiry rejected")
	}
}

func TestRateLimiterPruneBoundsMemory(t *testing.T) {
	l := newRateLimiter(1, time.Nanosecond)
	for i := 0; i < maxRateBuckets+1; i++ {
		l.Allow(string(rune('a'+i%26)) + time.Now().String())
	}
	l.mu.Lock()
	size := len(l.buckets)
	l.mu.Unlock()
	if size > maxRateBuckets {
		t.Fatalf("bucket map grew beyond cap: %d", size)
	}
}
