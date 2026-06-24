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

// NewRedis creates a new Redis client
func NewRedis(addr string) *Redis {
	client := redis.NewClient(&redis.Options{
		Addr:     addr,
		Password: "", // no password set
		DB:       0,  // use default DB
	})

	return &Redis{client: client}
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
