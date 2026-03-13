package main

import (
	"context"
	"errors"
	"flag"
	"log/slog"
	"net/http"
	"crypto/ed25519"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/gorilla/websocket"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"google.golang.org/protobuf/proto"

	pb "synch-server/pkg/proto/v1"
)

var (
	opsProcessed = promauto.NewCounter(prometheus.CounterOpts{
		Name: "synch_relay_messages_total",
		Help: "The total number of processed messages",
	})
	connectedClients = promauto.NewGauge(prometheus.GaugeOpts{
		Name: "synch_relay_connected_clients",
		Help: "The current number of connected clients",
	})
	errorsTotal = promauto.NewCounterVec(prometheus.CounterOpts{
		Name: "synch_relay_errors_total",
		Help: "Total number of errors by type",
	}, []string{"type"})
)

const (
	writeWait      = 10 * time.Second
	pongWait       = 60 * time.Second
	pingPeriod     = (pongWait * 9) / 10
	maxMessageSize = 1024 * 1024 // 1MB
)

var upgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
	CheckOrigin: func(r *http.Request) bool {
		return true // Allow all origins for dev
	},
}

// Client is a middleman between the websocket connection and the hub.
type Client struct {
	hub    *Hub
	conn   *websocket.Conn
	send   chan []byte
	nodeID string
}

func (c *Client) readPump() {
	defer func() {
		c.hub.unregister <- c
		c.conn.Close()
	}()
	c.conn.SetReadLimit(maxMessageSize)
	c.conn.SetReadDeadline(time.Now().Add(pongWait))
	c.conn.SetPongHandler(func(string) error {
		c.conn.SetReadDeadline(time.Now().Add(pongWait))
		return nil
	})
	for {
		_, message, err := c.conn.ReadMessage()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				slog.Error("read error", "error", err)
				errorsTotal.WithLabelValues("read_error").Inc()
			}
			break
		}
		opsProcessed.Inc()

		var syncMsg pb.SyncMessage
		if err := proto.Unmarshal(message, &syncMsg); err != nil {
			slog.Warn("unmarshal error", "error", err)
			errorsTotal.WithLabelValues("unmarshal_error").Inc()
			continue
		}

		slog.Debug("received message", "sender", syncMsg.SenderId)
		
		// Verify signature if it's a DeltaManifest
		if syncMsg.GetDelta() != nil {
			if err := verifyDeltaSignature(syncMsg.GetDelta()); err != nil {
				slog.Warn("signature verification failed", "sender", syncMsg.SenderId, "error", err)
				errorsTotal.WithLabelValues("signature_error").Inc()
				continue
			}
			slog.Debug("signature verified", "sender", syncMsg.SenderId)
		}

		c.hub.broadcast <- message
	}
}

func verifyDeltaSignature(d *pb.DeltaManifest) error {
	sig := d.ManifestSignature
	if sig == nil {
		slog.Debug("manifest signature is nil")
		return nil // For now, allow unsigned
	}

	if len(sig.SignatureBytes) == 0 || len(sig.SignerPublicKey) == 0 {
		slog.Debug("signature bytes or public key is empty")
		return nil
	}

	slog.Debug("verifying signature",
		"pubkey_len", len(sig.SignerPublicKey),
		"data_hash_len", len(sig.DataHash),
		"sig_len", len(sig.SignatureBytes))

	if !ed25519.Verify(sig.SignerPublicKey, sig.DataHash, sig.SignatureBytes) {
		slog.Warn("ed25519.Verify failed")
		return errors.New("invalid signature")
	}
	slog.Debug("ed25519.Verify success")
	return nil
}

func (c *Client) writePump() {
	ticker := time.NewTicker(pingPeriod)
	defer func() {
		ticker.Stop()
		c.conn.Close()
	}()
	for {
		select {
		case message, ok := <-c.send:
			c.conn.SetWriteDeadline(time.Now().Add(writeWait))
			if !ok {
				c.conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}

			w, err := c.conn.NextWriter(websocket.BinaryMessage)
			if err != nil {
				return
			}
			w.Write(message)

			if err := w.Close(); err != nil {
				return
			}
		case <-ticker.C:
			c.conn.SetWriteDeadline(time.Now().Add(writeWait))
			if err := c.conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}
		}
	}
}

// Hub maintains the set of active clients and broadcasts messages to them.
type Hub struct {
	clients    map[*Client]bool
	broadcast  chan []byte
	register   chan *Client
	unregister chan *Client
	mu         sync.Mutex
}

func newHub() *Hub {
	return &Hub{
		broadcast:  make(chan []byte),
		register:   make(chan *Client),
		unregister: make(chan *Client),
		clients:    make(map[*Client]bool),
	}
}

func (h *Hub) run() {
	for {
		select {
		case client := <-h.register:
			h.mu.Lock()
			h.clients[client] = true
			connectedClients.Set(float64(len(h.clients)))
			h.mu.Unlock()
			slog.Info("client registered", "count", len(h.clients))
		case client := <-h.unregister:
			h.mu.Lock()
			if _, ok := h.clients[client]; ok {
				delete(h.clients, client)
				close(client.send)
				connectedClients.Set(float64(len(h.clients)))
				slog.Info("client unregistered", "count", len(h.clients))
			}
			h.mu.Unlock()
		case message := <-h.broadcast:
			h.mu.Lock()
			for client := range h.clients {
				select {
				case client.send <- message:
				default:
					close(client.send)
					delete(h.clients, client)
				}
			}
			h.mu.Unlock()
		}
	}
}

func serveWs(hub *Hub, w http.ResponseWriter, r *http.Request) {
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

func main() {
	addr := flag.String("addr", ":8080", "http service address")
	logLevel := flag.String("log", "info", "log level (debug, info, warn, error)")
	flag.Parse()

	var level slog.Level
	switch *logLevel {
	case "debug":
		level = slog.LevelDebug
	case "info":
		level = slog.LevelInfo
	case "warn":
		level = slog.LevelWarn
	case "error":
		level = slog.LevelError
	default:
		level = slog.LevelInfo
	}

	logger := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: level}))
	slog.SetDefault(logger)

	hub := newHub()
	go hub.run()

	http.HandleFunc("/ws", func(w http.ResponseWriter, r *http.Request) {
		serveWs(hub, w, r)
	})
	http.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		w.Write([]byte("OK"))
	})
	http.Handle("/metrics", promhttp.Handler())

	srv := &http.Server{
		Addr:              *addr,
		ReadHeaderTimeout: 3 * time.Second,
	}

	done := make(chan os.Signal, 1)
	signal.Notify(done, os.Interrupt, syscall.SIGTERM)

	go func() {
		slog.Info("Relay Server starting", "addr", *addr, "logLevel", *logLevel)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			slog.Error("listen error", "error", err)
			os.Exit(1)
		}
	}()

	<-done
	slog.Info("Server stopping")

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("shutdown error", "error", err)
	}
	slog.Info("Server stopped")
}
