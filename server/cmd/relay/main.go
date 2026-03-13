package main

import (
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/websocket"
	"google.golang.org/protobuf/proto"

	pb "synch-server/pkg/proto/v1"
)

var upgrader = websocket.Upgrader{
	CheckOrigin: func(r *http.Request) bool {
		return true // Allow all origins for dev
	},
}

// SynchClient represents a single connected client.
type SynchClient struct {
	conn       *websocket.Conn
	state      pb.SyncState
	stateMutex sync.Mutex
}

func (c *SynchClient) setState(s pb.SyncState) {
	c.stateMutex.Lock()
	defer c.stateMutex.Unlock()
	c.state = s
}

func (c *SynchClient) getState() pb.SyncState {
	c.stateMutex.Lock()
	defer c.stateMutex.Unlock()
	return c.state
}

func handleWebSocket(w http.ResponseWriter, r *http.Request) {
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Println("Upgrade error:", err)
		return
	}

	client := &SynchClient{
		conn:  conn,
		state: pb.SyncState_SYNC_STATE_CONNECTING,
	}

	log.Printf("Client connected. Initial state: %s", client.getState())

	// Read pump
	defer func() {
		client.setState(pb.SyncState_SYNC_STATE_IDLE)
		log.Printf("Client disconnected. Final state: %s", client.getState())
		conn.Close()
	}()

	for {
		messageType, message, err := conn.ReadMessage()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				log.Printf("Read error: %v", err)
			}
			break
		}

		if messageType != websocket.BinaryMessage {
			log.Println("Received non-binary message, ignoring")
			continue
		}

		var syncMsg pb.SyncMessage
		if err := proto.Unmarshal(message, &syncMsg); err != nil {
			log.Printf("Failed to unmarshal SyncMessage: %v", err)
			continue
		}

		log.Printf("Received SyncMessage: %+v", &syncMsg)
	}
}

func main() {
	http.HandleFunc("/ws", handleWebSocket)

	serverAddr := ":8080"
	log.Printf("Synch Relay Server is listening on %s", serverAddr)
	
	server := &http.Server{
		Addr:              serverAddr,
		ReadHeaderTimeout: 3 * time.Second,
	}

	if err := server.ListenAndServe(); err != nil {
		log.Fatalf("Server failed: %v", err)
	}
}
