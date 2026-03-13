package storage

import (
	"os"
	"testing"

	pb "synch-server/pkg/proto/v1"
)

func TestContractPersistence(t *testing.T) {
	tmpDir, _ := os.MkdirTemp("", "badger-test-*")
	defer os.RemoveAll(tmpDir)

	store, err := NewBadgerStore(tmpDir)
	if err != nil {
		t.Fatal(err)
	}
	defer store.Close()

	c := &pb.Contract{
		ContractId:  "test-c1",
		RequesterId: []byte("alice"),
		TargetId:    []byte("bob"),
		ExpiresAt:   123456789,
	}

	if err := store.SaveContract(c); err != nil {
		t.Fatal(err)
	}

	loaded, err := store.LoadContracts()
	if err != nil {
		t.Fatal(err)
	}

	if len(loaded) != 1 || loaded[0].ContractId != c.ContractId {
		t.Errorf("expected 1 contract, got %d", len(loaded))
	}

	// Test Delete
	if err := store.DeleteContract(c.ContractId); err != nil {
		t.Fatal(err)
	}
	loaded, _ = store.LoadContracts()
	if len(loaded) != 0 {
		t.Errorf("contract should be deleted")
	}
}

func TestPresencePersistence(t *testing.T) {
	tmpDir, _ := os.MkdirTemp("", "badger-presence-*")
	defer os.RemoveAll(tmpDir)

	store, err := NewBadgerStore(tmpDir)
	if err != nil {
		t.Fatal(err)
	}
	defer store.Close()

	p := &pb.PresenceUpdate{
		NodeId:          "node-1",
		Status:          pb.PresenceStatus_PRESENCE_STATUS_ONLINE,
		PerceptionLevel: pb.PerceptionLevel_PERCEPTION_LEVEL_L2,
		PreferredRelays: []string{"http://relay1", "http://relay2"},
	}

	if err := store.SavePresence(p); err != nil {
		t.Fatal(err)
	}

	loaded, err := store.LoadPresence("node-1")
	if err != nil {
		t.Fatal(err)
	}
	if loaded.PerceptionLevel != p.PerceptionLevel || len(loaded.PreferredRelays) != 2 {
		t.Errorf("presence mismatch")
	}

	all, _ := store.LoadAllPresences()
	if len(all) != 1 {
		t.Errorf("expected 1 presence in LoadAll")
	}
}

func TestOfflineMessagePersistence(t *testing.T) {
	dbPath := "./test_db_offline"
	defer os.RemoveAll(dbPath)

	store, err := NewBadgerStore(dbPath)
	if err != nil {
		t.Fatalf("failed to open store: %v", err)
	}
	defer store.Close()

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
