# Synch API Documentation (v1)

This document describes the core protobuf messages and service interfaces for the Synch protocol.

## 🆔 Identity & Node

### `NodeIdentity`
Represents a unique entity in the Synch network.
- `node_id`: String (e.g., `agent://<pubkey>`)
- `public_key`: Ed25519 Public Key (32 bytes)
- `node_type`: `AGENT`, `HUMAN`, `BRIDGE`, `PLUGIN`, `MOBILE`
- `capabilities`: List of strings (e.g., `vault-sync`, `chat`)

---

## 🔒 Security & Vaults

### `VaultPermission`
Defines access rights to a specific encrypted vault.
- `vault_id`: 16-byte hash.
- `level`: `READ`, `WRITE`, `ADMIN`.
- `vault_key_encrypted`: AES-wrapped vault master key.

---

## 🔄 Synchronization

### `SyncMessage`
The primary frame for relay communication.
- `version`: Protocol version.
- `type`: `PING`, `PONG`, `DELTA`, `AUTH`, `ERROR`.
- `payload`: Encrypted byte array containing the actual command or data.

---

## 📡 Relay Server Interface

The Relay Server provides both **WebSockets** for real-time synchronization and **HTTP** for health monitoring and metrics.

### HTTP Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | `GET` | Returns `200 OK` if the server is healthy. |
| `/metrics` | `GET` | Returns Prometheus metrics (connections, message rates, etc). |

---

### WebSocket Connection Flow

1. **Handshake**: Client connects to `wss://<host>/ws` (Production) or `ws://<host>:8080/ws` (Development).
2. **Handshake Verification**:
    - Production uses **Nginx** for SSL/TLS termination and WSS upgrades.
    - Ensure headers `Upgrade: websocket` and `Connection: Upgrade` are provided.
3. **Auth**: Client sends an `AUTH` message signed by their private key.
4. **Session**: Server acknowledges and opens a sync stream.
5. **Sync**: Client and Server exchange `DELTA` updates.

For detailed message definitions, see [proto/v1/synch.proto](../proto/v1/synch.proto).

