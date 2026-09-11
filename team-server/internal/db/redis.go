package db

import (
	"context"
	"time"

	"github.com/redis/go-redis/v9"
)

// Redis wraps Redis client
type Redis struct {
	client *redis.Client
}

// NewRedis creates a new Redis client. addr accepts either a bare
// "host:port" or a redis:// URL (redis://[:password@]host:port[/db]) —
// redis.ParseURL handles the URL form, including passwords and DB numbers.
func NewRedis(addr string) *Redis {
	opts := &redis.Options{Addr: addr}
	if len(addr) > 8 && (addr[:8] == "redis://" || (len(addr) > 9 && addr[:9] == "rediss://")) {
		if parsed, err := redis.ParseURL(addr); err == nil {
			opts = parsed
		}
	}

	client := redis.NewClient(opts)

	return &Redis{client: client}
}

// Addr returns the effective address of the underlying client.
func (r *Redis) Addr() string {
	return r.client.Options().Addr
}

// Close closes the Redis connection
func (r *Redis) Close() error {
	return r.client.Close()
}

// Client returns the underlying Redis client
func (r *Redis) Client() *redis.Client {
	return r.client
}

// Set stores a value with expiration
func (r *Redis) Set(key string, value interface{}, expiration time.Duration) error {
	ctx := context.Background()
	return r.client.Set(ctx, key, value, expiration).Err()
}

// Get retrieves a value
func (r *Redis) Get(key string) (string, error) {
	ctx := context.Background()
	return r.client.Get(ctx, key).Result()
}

// Delete removes a value
func (r *Redis) Delete(key string) error {
	ctx := context.Background()
	return r.client.Del(ctx, key).Err()
}
