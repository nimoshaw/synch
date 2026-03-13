package storage

import (
	"fmt"
	"time"

	"github.com/dgraph-io/badger/v4"
	"google.golang.org/protobuf/proto"
	pb "synch-server/pkg/proto/v1"
)

type BadgerStore struct {
	db *badger.DB
}

func NewBadgerStore(path string) (*BadgerStore, error) {
	opts := badger.DefaultOptions(path).WithLoggingLevel(badger.WARNING)
	db, err := badger.Open(opts)
	if err != nil {
		return nil, fmt.Errorf("failed to open badger: %w", err)
	}
	return &BadgerStore{db: db}, nil
}

func (s *BadgerStore) Close() error {
	return s.db.Close()
}

// Contract Keys: c:{contract_id}
func contractKey(id string) []byte {
	return []byte("c:" + id)
}

func (s *BadgerStore) SaveContract(contract *pb.Contract) error {
	data, err := proto.Marshal(contract)
	if err != nil {
		return err
	}
	return s.db.Update(func(txn *badger.Txn) error {
		return txn.Set(contractKey(contract.ContractId), data)
	})
}

func (s *BadgerStore) LoadContracts() ([]*pb.Contract, error) {
	var contracts []*pb.Contract
	err := s.db.View(func(txn *badger.Txn) error {
		it := txn.NewIterator(badger.DefaultIteratorOptions)
		defer it.Close()
		prefix := []byte("c:")
		for it.Seek(prefix); it.ValidForPrefix(prefix); it.Next() {
			item := it.Item()
			err := item.Value(func(v []byte) error {
				c := &pb.Contract{}
				if err := proto.Unmarshal(v, c); err != nil {
					return err
				}
				contracts = append(contracts, c)
				return nil
			})
			if err != nil {
				return err
			}
		}
		return nil
	})
	return contracts, err
}

// Offline Message Keys: o:{node_id}:{nanos}
func offlineKeyPrefix(nodeID string) []byte {
	return []byte("o:" + nodeID + ":")
}

func (s *BadgerStore) SaveOfflineMessage(nodeID string, payload []byte) error {
	key := []byte(fmt.Sprintf("o:%s:%d", nodeID, time.Now().UnixNano()))
	return s.db.Update(func(txn *badger.Txn) error {
		return txn.Set(key, payload)
	})
}

func (s *BadgerStore) LoadOfflineMessages(nodeID string) ([][]byte, error) {
	var messages [][]byte
	err := s.db.View(func(txn *badger.Txn) error {
		it := txn.NewIterator(badger.DefaultIteratorOptions)
		defer it.Close()
		prefix := offlineKeyPrefix(nodeID)
		for it.Seek(prefix); it.ValidForPrefix(prefix); it.Next() {
			item := it.Item()
			err := item.Value(func(v []byte) error {
				msg := make([]byte, len(v))
				copy(msg, v)
				messages = append(messages, msg)
				return nil
			})
			if err != nil {
				return err
			}
		}
		return nil
	})
	return messages, err
}

func (s *BadgerStore) ClearOfflineMessages(nodeID string) error {
	return s.db.Update(func(txn *badger.Txn) error {
		it := txn.NewIterator(badger.DefaultIteratorOptions)
		defer it.Close()
		prefix := offlineKeyPrefix(nodeID)
		for it.Seek(prefix); it.ValidForPrefix(prefix); it.Next() {
			if err := txn.Delete(it.Item().KeyCopy(nil)); err != nil {
				return err
			}
		}
		return nil
	})
}

// Perception Keys: p:{node_id}
func perceptionKey(id string) []byte {
	return []byte("p:" + id)
}

func (s *BadgerStore) DeleteContract(id string) error {
	return s.db.Update(func(txn *badger.Txn) error {
		return txn.Delete(contractKey(id))
	})
}

// Presence Keys: pr:{node_id}
func presenceKey(id string) []byte {
	return []byte("pr:" + id)
}

func (s *BadgerStore) SavePresence(presence *pb.PresenceUpdate) error {
	data, err := proto.Marshal(presence)
	if err != nil {
		return err
	}
	return s.db.Update(func(txn *badger.Txn) error {
		return txn.Set(presenceKey(presence.NodeId), data)
	})
}

func (s *BadgerStore) LoadPresence(nodeID string) (*pb.PresenceUpdate, error) {
	var presence *pb.PresenceUpdate
	err := s.db.View(func(txn *badger.Txn) error {
		item, err := txn.Get(presenceKey(nodeID))
		if err != nil {
			return err
		}
		return item.Value(func(v []byte) error {
			presence = &pb.PresenceUpdate{}
			return proto.Unmarshal(v, presence)
		})
	})
	return presence, err
}

func (s *BadgerStore) LoadAllPresences() ([]*pb.PresenceUpdate, error) {
	var presences []*pb.PresenceUpdate
	err := s.db.View(func(txn *badger.Txn) error {
		it := txn.NewIterator(badger.DefaultIteratorOptions)
		defer it.Close()
		prefix := []byte("pr:")
		for it.Seek(prefix); it.ValidForPrefix(prefix); it.Next() {
			item := it.Item()
			err := item.Value(func(v []byte) error {
				p := &pb.PresenceUpdate{}
				if err := proto.Unmarshal(v, p); err != nil {
					return err
				}
				presences = append(presences, p)
				return nil
			})
			if err != nil {
				return err
			}
		}
		return nil
	})
	return presences, err
}

func (s *BadgerStore) SavePerceptionLevel(nodeID string, level pb.PerceptionLevel) error {
	return s.db.Update(func(txn *badger.Txn) error {
		return txn.Set(perceptionKey(nodeID), []byte{byte(level)})
	})
}

func (s *BadgerStore) LoadPerceptionLevel(nodeID string) (pb.PerceptionLevel, error) {
	var level pb.PerceptionLevel
	err := s.db.View(func(txn *badger.Txn) error {
		item, err := txn.Get(perceptionKey(nodeID))
		if err != nil {
			if err == badger.ErrKeyNotFound {
				level = pb.PerceptionLevel_PERCEPTION_LEVEL_L0 // Default
				return nil
			}
			return err
		}
		return item.Value(func(v []byte) error {
			if len(v) > 0 {
				level = pb.PerceptionLevel(v[0])
			}
			return nil
		})
	})
	return level, err
}
