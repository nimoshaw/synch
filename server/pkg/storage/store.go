package storage

import (
	pb "synch-server/pkg/proto/v1"
)

// Store defines the interface for relay persistence
type Store interface {
	// Contracts
	SaveContract(contract *pb.Contract) error
	LoadContracts() ([]*pb.Contract, error)
	DeleteContract(id string) error

	// Offline Messages
	SaveOfflineMessage(nodeID string, payload []byte) error
	LoadOfflineMessages(nodeID string) ([][]byte, error)
	ClearOfflineMessages(nodeID string) error

	// Presence
	SavePresence(presence *pb.PresenceUpdate) error
	LoadPresence(nodeID string) (*pb.PresenceUpdate, error)
	LoadAllPresences() ([]*pb.PresenceUpdate, error)

	// Privacy
	SavePerceptionLevel(nodeID string, level pb.PerceptionLevel) error
	LoadPerceptionLevel(nodeID string) (pb.PerceptionLevel, error)

	// Lifecycle
	Close() error
}
