package main

import (
	"context"
	"encoding/json"
	"flag"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"runtime"
	"strings"
	"syscall"
	"time"

	"github.com/gorilla/websocket"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"synch-server/pkg/storage"
)

// Version is injected at build time via -ldflags "-X main.Version=..."
var Version = "dev"

const (
	writeWait      = 10 * time.Second
	pongWait       = 60 * time.Second
	pingPeriod     = (pongWait * 9) / 10
	maxMessageSize = 1024 * 1024 // 1MB
)

// Config holds the relay server configuration, resolved from flags → env vars → defaults.
type Config struct {
	Addr           string
	DBPath         string
	LogLevel       string
	Mode           string // "development" or "production"
	AdminToken     string
	AllowedOrigins []string
}

// resolveConfig merges CLI flags with environment variables. Flags take precedence.
func resolveConfig() *Config {
	addr := flag.String("addr", "", "http service address (env: SYNCH_WS_PORT)")
	dbPath := flag.String("db", "", "path to badger database (env: SYNCH_DB_PATH)")
	logLevel := flag.String("log", "", "log level: debug, info, warn, error (env: SYNCH_LOG_LEVEL)")
	mode := flag.String("mode", "", "operating mode: development, production (env: SYNCH_MODE)")
	flag.Parse()

	cfg := &Config{
		Addr:     envOrDefault("SYNCH_WS_PORT", ":8080"),
		DBPath:   envOrDefault("SYNCH_DB_PATH", "./relay_db"),
		LogLevel: envOrDefault("SYNCH_LOG_LEVEL", "info"),
		Mode:     envOrDefault("SYNCH_MODE", "production"),
		AdminToken: os.Getenv("SYNCH_ADMIN_TOKEN"),
	}

	// CLI flags override env vars
	if *addr != "" {
		cfg.Addr = *addr
	}
	if *dbPath != "" {
		cfg.DBPath = *dbPath
	}
	if *logLevel != "" {
		cfg.LogLevel = *logLevel
	}
	if *mode != "" {
		cfg.Mode = *mode
	}

	// Parse allowed origins
	origins := envOrDefault("SYNCH_ALLOWED_ORIGINS", "")
	if origins != "" && origins != "*" {
		for _, o := range strings.Split(origins, ",") {
			cfg.AllowedOrigins = append(cfg.AllowedOrigins, strings.TrimSpace(o))
		}
	}

	return cfg
}

func envOrDefault(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func parseLogLevel(s string) slog.Level {
	switch strings.ToLower(s) {
	case "debug":
		return slog.LevelDebug
	case "info":
		return slog.LevelInfo
	case "warn":
		return slog.LevelWarn
	case "error":
		return slog.LevelError
	default:
		return slog.LevelInfo
	}
}

// makeUpgrader creates a WebSocket upgrader with origin checking based on mode.
func makeUpgrader(cfg *Config) websocket.Upgrader {
	return websocket.Upgrader{
		ReadBufferSize:  4096,
		WriteBufferSize: 4096,
		CheckOrigin: func(r *http.Request) bool {
			if cfg.Mode == "development" {
				return true
			}
			// In production: if no whitelist configured, allow all (LAN use case)
			if len(cfg.AllowedOrigins) == 0 {
				return true
			}
			origin := r.Header.Get("Origin")
			for _, allowed := range cfg.AllowedOrigins {
				if origin == allowed {
					return true
				}
			}
			slog.Warn("rejected WebSocket origin", "origin", origin)
			return false
		},
	}
}

func serveWs(hub *Hub, upgrader *websocket.Upgrader, w http.ResponseWriter, r *http.Request) {
	// Rate limit by IP
	if !hub.limiter.ConnLimiter.Allow(r.RemoteAddr) {
		http.Error(w, "Too many connections", http.StatusTooManyRequests)
		slog.Warn("connection rate limit exceeded", "ip", r.RemoteAddr)
		return
	}

	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		slog.Error("upgrade error", "error", err)
		return
	}
	client := &Client{hub: hub, conn: conn, send: make(chan []byte, 256)}
	client.hub.register <- client

	go client.writePump()
	go client.readPump()
}

// healthResponse is the JSON structure returned by /health
type healthResponse struct {
	Status           string `json:"status"`
	Version          string `json:"version"`
	Mode             string `json:"mode"`
	Uptime           string `json:"uptime"`
	UptimeSeconds    int64  `json:"uptime_seconds"`
	ConnectedClients int    `json:"connected_clients"`
	GoVersion        string `json:"go_version"`
}

func main() {
	cfg := resolveConfig()

	level := parseLogLevel(cfg.LogLevel)
	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: level}))
	slog.SetDefault(logger)

	store, err := storage.NewBadgerStore(cfg.DBPath)
	if err != nil {
		slog.Error("failed to initialize store", "error", err)
		os.Exit(1)
	}
	defer store.Close()

	hub := newHub(store)
	go hub.run()

	upgrader := makeUpgrader(cfg)
	startTime := time.Now()

	// --- Routes ---
	mux := http.NewServeMux()

	// WebSocket endpoint
	mux.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		serveWs(hub, &upgrader, w, r)
	})

	// Health check (public, returns JSON)
	mux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		hub.mu.RLock()
		clientCount := len(hub.clients)
		hub.mu.RUnlock()

		uptime := time.Since(startTime)
		resp := healthResponse{
			Status:           "ok",
			Version:          Version,
			Mode:             cfg.Mode,
			Uptime:           formatDuration(uptime),
			UptimeSeconds:    int64(uptime.Seconds()),
			ConnectedClients: clientCount,
			GoVersion:        runtime.Version(),
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	})

	// Prometheus metrics
	mux.Handle("/metrics", promhttp.Handler())

	// Admin API
	admin := NewAdminAPI(hub, cfg.AdminToken)
	admin.RegisterRoutes(mux)

	srv := &http.Server{
		Addr:              cfg.Addr,
		Handler:           mux,
		ReadHeaderTimeout: 5 * time.Second,
		IdleTimeout:       120 * time.Second,
	}

	done := make(chan os.Signal, 1)
	signal.Notify(done, os.Interrupt, syscall.SIGTERM)

	go func() {
		slog.Info("Synch Relay Server starting",
			"version", Version,
			"addr", cfg.Addr,
			"mode", cfg.Mode,
			"logLevel", cfg.LogLevel,
			"dbPath", cfg.DBPath,
		)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			slog.Error("listen error", "error", err)
			os.Exit(1)
		}
	}()

	<-done
	slog.Info("Server stopping, initiating graceful shutdown...")

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	// Close the hub before HTTP server so WebSocket clients get a clean close
	hub.shutdown()

	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("shutdown error", "error", err)
	}
	slog.Info("Synch Relay Server stopped")
}

// formatDuration returns a human-readable uptime string
func formatDuration(d time.Duration) string {
	days := int(d.Hours()) / 24
	hours := int(d.Hours()) % 24
	minutes := int(d.Minutes()) % 60
	if days > 0 {
		return fmt.Sprintf("%dd %dh %dm", days, hours, minutes)
	}
	if hours > 0 {
		return fmt.Sprintf("%dh %dm", hours, minutes)
	}
	return fmt.Sprintf("%dm", minutes)
}
