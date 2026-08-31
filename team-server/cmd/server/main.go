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

	// Fail closed: never boot with a guessable signing secret. Development
	// setups must opt in explicitly with RECAP_INSECURE_DEV=1.
	jwtSecret := os.Getenv("JWT_SECRET")
	if jwtSecret == "" {
		if os.Getenv("RECAP_INSECURE_DEV") == "1" {
			jwtSecret = "insecure-dev-secret-do-not-use-in-production"
			log.Println("WARNING: RECAP_INSECURE_DEV=1 — using an insecure development JWT secret.")
		} else {
			log.Fatal("JWT_SECRET environment variable is required (set RECAP_INSECURE_DEV=1 to bypass in development)")
		}
	}

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
