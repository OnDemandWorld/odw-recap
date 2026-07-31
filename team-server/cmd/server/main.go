package main

import (
	"context"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/ondemandworld/recap-team-server/internal/api"
	"github.com/ondemandworld/recap-team-server/internal/db"
	odwsync "github.com/ondemandworld/recap-team-server/internal/sync"
)

func main() {
	// Load configuration
	port := os.Getenv("PORT")
	if port == "" {
		port = "8080"
	}

	dsn := os.Getenv("DATABASE_URL")
	if dsn == "" {
		dsn = "postgres://recap:recap@localhost:5432/recap?sslmode=disable"
	}

	redisURL := os.Getenv("REDIS_URL")
	if redisURL == "" {
		redisURL = "localhost:6379"
	}

	jwtSecret := os.Getenv("JWT_SECRET")
	if jwtSecret == "" {
		jwtSecret = "change-this-secret-in-production"
		log.Println("WARNING: Using default JWT secret. Set JWT_SECRET environment variable!")
	}

	// Cross-product sync configuration (Recap -> Vault / Loop)
	vaultAPIURL := os.Getenv("VAULT_API_URL")
	if vaultAPIURL == "" {
		vaultAPIURL = "http://localhost:8765"
	}

	loopAPIURL := os.Getenv("LOOP_API_URL")
	if loopAPIURL == "" {
		loopAPIURL = "http://localhost:3000"
	}

	// Optional: when LOOP_WEBHOOK_TRIGGER_ID is unset, Loop triggering is skipped.
	loopTriggerID := os.Getenv("LOOP_WEBHOOK_TRIGGER_ID")
	loopWebhookSecret := os.Getenv("LOOP_WEBHOOK_SECRET")

	// Initialize database
	database, err := db.NewDatabase(dsn)
	if err != nil {
		log.Fatalf("Failed to initialize database: %v", err)
	}
	defer database.Close()

	// Initialize Redis
	redis := db.NewRedis(redisURL)
	defer redis.Close()

	// Initialize API server
	server := api.NewServer(database, redis, jwtSecret)

	// Wire the cross-product sync forwarder (Recap -> Vault / Loop)
	syncForwarder := odwsync.NewForwarder(&http.Client{Timeout: 30 * time.Second}, odwsync.Config{
		VaultAPIURL:        vaultAPIURL,
		LoopAPIURL:         loopAPIURL,
		LoopWebhookTrigger: loopTriggerID,
		LoopWebhookSecret:  loopWebhookSecret,
	})
	server.SetSyncForwarder(syncForwarder)

	// Create HTTP server
	httpServer := &http.Server{
		Addr:         ":" + port,
		Handler:      server.Router(),
		ReadTimeout:  15 * time.Second,
		WriteTimeout: 15 * time.Second,
		IdleTimeout:  60 * time.Second,
	}

	// Start server in goroutine
	go func() {
		log.Printf("Team server starting on port %s", port)
		if err := httpServer.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("Server failed: %v", err)
		}
	}()

	// Wait for interrupt signal
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	log.Println("Shutting down server...")

	// Graceful shutdown
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	if err := httpServer.Shutdown(ctx); err != nil {
		log.Fatalf("Server forced to shutdown: %v", err)
	}

	log.Println("Server stopped")
}
