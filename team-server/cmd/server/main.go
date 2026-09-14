package main

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"log"
	"net/http"
	"os"
	"os/signal"
	"sort"
	"strings"
	"syscall"
	"time"

	"github.com/ondemandworld/recap-team-server/internal/api"
	"github.com/ondemandworld/recap-team-server/internal/db"
	odwsync "github.com/ondemandworld/recap-team-server/internal/sync"
)

// ensureLoopbackBypassesProxy keeps loopback sibling calls (Vault/Loop) out of
// HTTP proxies: with HTTP_PROXY set and NO_PROXY missing 127.0.0.1/localhost,
// Go's ProxyFromEnvironment routes internal calls through the proxy.
func ensureLoopbackBypassesProxy() {
	for _, key := range []string{"NO_PROXY", "no_proxy"} {
		parts := map[string]bool{}
		for _, p := range strings.Split(os.Getenv(key), ",") {
			if t := strings.TrimSpace(p); t != "" {
				parts[t] = true
			}
		}
		if !parts["localhost"] || !parts["127.0.0.1"] {
			parts["localhost"] = true
			parts["127.0.0.1"] = true
			list := make([]string, 0, len(parts))
			for k := range parts {
				list = append(list, k)
			}
			sort.Strings(list)
			os.Setenv(key, strings.Join(list, ","))
		}
	}
}

func main() {
	ensureLoopbackBypassesProxy()
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
		// No configured secret: use a random ephemeral one instead of a
		// publicly-known constant. Tokens issued before a restart stop
		// working, which is annoying but far safer than a forgeable secret.
		buf := make([]byte, 32)
		if _, err := rand.Read(buf); err != nil {
			log.Fatalf("Failed to generate ephemeral JWT secret: %v", err)
		}
		jwtSecret = hex.EncodeToString(buf)
		log.Println("WARNING: JWT_SECRET not set; using a random ephemeral secret. " +
			"All tokens become invalid on restart. Set JWT_SECRET in production.")
	} else if len(jwtSecret) < 32 {
		// An explicit but short secret is brute-forceable; refuse rather than
		// run with a false sense of security.
		log.Fatalf("JWT_SECRET must be at least 32 characters (current length: %d)", len(jwtSecret))
	}

	// CORS allowlist for browser-based clients. Authentication itself is
	// bearer-token based, so this only governs which web origins may call the
	// API from a browser. Comma-separated, e.g. "https://recap.example.com".
	allowedOrigins := strings.Split(strings.TrimSpace(os.Getenv("ALLOWED_ORIGINS")), ",")
	origins := make([]string, 0, len(allowedOrigins))
	for _, o := range allowedOrigins {
		if o = strings.TrimSpace(o); o != "" {
			origins = append(origins, o)
		}
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
	// That makes the Recap→Loop chain a silent no-op while /sync still reports
	// success — warn loudly so operators know the integration is dormant.
	loopTriggerID := os.Getenv("LOOP_WEBHOOK_TRIGGER_ID")
	loopWebhookSecret := os.Getenv("LOOP_WEBHOOK_SECRET")
	if loopTriggerID == "" {
		log.Printf("WARNING: LOOP_WEBHOOK_TRIGGER_ID is not set — meeting action items will NOT be forwarded to Loop workflows. Create a webhook trigger in Loop and set LOOP_WEBHOOK_TRIGGER_ID + LOOP_WEBHOOK_SECRET to activate the Recap→Loop chain.")
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
	if len(origins) > 0 {
		server.SetAllowedOrigins(origins)
	}

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
