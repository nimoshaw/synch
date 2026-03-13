package storage

import (
	"os"
	"testing"

	pb "synch-server/pkg/proto/v1"
)

func TestBadgerStore(t *testing.T) {
	dbPath := "./test_db"
	defer os.RemoveAll(dbPath)

	store, err := NewBadgerStore(dbPath)
	if err != nil {
		t.Fatalf("failed to open store: %v", err)
	}
	defer store.Close()

	// Test Contract Persistence
	contract := &pb.Contract{
		ContractId: "test-c-1",
		Capabilities: []string{"chat", "files"},
	}
	if err := store.SaveContract(contract); err != nil {
		t.Errorf("failed to save contract: %v", err)
	}

	contracts, err := store.LoadContracts()
	if err != nil {
		t.Errorf("failed to load contracts: %v", err)
	}
	if len(contracts) != 1 || contracts[0].ContractId != "test-c-1" {
		t.Errorf("unexpected contracts: %v", contracts)
	}

	// Test Offline Message Persistence
	nodeID := "node-1"
	payload := []byte("hello world")
	if err := store.SaveOfflineMessage(nodeID, payload); err != nil {
		t.Errorf("failed to save message: %v", err)
	}

	msgs, err := store.LoadOfflineMessages(nodeID)
	if err != nil {
		t.Errorf("failed to load messages: %v", err)
	}
	if len(msgs) != 1 || string(msgs[0]) != "hello world" {
		t.Errorf("unexpected messages: %v", msgs)
	}

	// Test Clear
	if err := store.ClearOfflineMessages(nodeID); err != nil {
		t.Errorf("failed to clear messages: %v", err)
	}
	msgs, _ = store.LoadOfflineMessages(nodeID)
	if len(msgs) != 0 {
		t.Errorf("expected 0 messages after clear, got %d", len(msgs))
	}
}
