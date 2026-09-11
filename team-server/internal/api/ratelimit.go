package api

import (
	"sort"
	"sync"
	"time"
)

// loginRateLimit and loginRateWindow bound failed login attempts per client:
// at most loginRateLimit attempts inside loginRateWindow. Successful logins
// do not consume budget; the handler only records failures.
const (
	loginRateLimit  = 10
	loginRateWindow = time.Minute
)

// rateLimiter is a fixed-window in-memory rate limiter keyed by an arbitrary
// string (typically client IP + account identifier). It is deliberately
// dependency-free and per-process: sufficient to blunt credential stuffing
// against a single team-server instance.
type rateLimiter struct {
	limit  int
	window time.Duration

	mu      sync.Mutex
	buckets map[string]*rateBucket
}

type rateBucket struct {
	count     int
	windowEnd time.Time
}

func newRateLimiter(limit int, window time.Duration) *rateLimiter {
	return &rateLimiter{
		limit:   limit,
		window:  window,
		buckets: make(map[string]*rateBucket),
	}
}

// Allow records an attempt for key and reports whether it is within budget.
// Expired windows reset; when the map hits its cap, expired buckets are
// dropped and, if still over, the earliest-expiring buckets are evicted so
// memory stays bounded even under key-cycling attacks.
func (l *rateLimiter) Allow(key string) bool {
	now := time.Now()

	l.mu.Lock()
	defer l.mu.Unlock()

	if len(l.buckets) >= maxRateBuckets {
		l.evictLocked(now)
	}

	b, ok := l.buckets[key]
	if !ok || now.After(b.windowEnd) {
		l.buckets[key] = &rateBucket{count: 1, windowEnd: now.Add(l.window)}
		return true
	}
	if b.count >= l.limit {
		return false
	}
	b.count++
	return true
}

// maxRateBuckets caps memory use; when exceeded, stale buckets are dropped.
const maxRateBuckets = 100_000

func (l *rateLimiter) evictLocked(now time.Time) {
	for k, b := range l.buckets {
		if now.After(b.windowEnd) {
			delete(l.buckets, k)
		}
	}
	if len(l.buckets) < maxRateBuckets {
		return
	}
	// All windows still live (key-cycling attack): drop the oldest quarter of
	// buckets, which only relaxes limiting for the least recently seen keys.
	type entry struct {
		key string
		end time.Time
	}
	entries := make([]entry, 0, len(l.buckets))
	for k, b := range l.buckets {
		entries = append(entries, entry{k, b.windowEnd})
	}
	sort.Slice(entries, func(i, j int) bool { return entries[i].end.Before(entries[j].end) })
	for _, e := range entries[:len(entries)/4] {
		delete(l.buckets, e.key)
	}
}
