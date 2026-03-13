# ADR 002: Multi-Relay Topology & Contract Scoping

## 1. Context

Currently, the Synch architecture implies a single-relay connection model where an Agent or User connects to a specific relay server to communicate and form contracts. However, to fulfill the vision of a truly decentralized and flexible network (similar to Fediverse or Nostr), a Node (User or Agent) must be able to simultaneously connect to **multiple relay servers**. 

The specific requirement is:
*   A User or Agent can join multiple service nodes (Relays).
*   However, the server node should no longer be the _only_ point of access or the universal broker for all interactions.
*   **Crucially**: An Agent that has already formed a contract (e.g., belongs to a specific User or organization on Relay A) can join other nodes (Relay B, Relay C) for communication or task execution, but **should not accept new binding contracts** on those secondary nodes unless explicitly authorized.

## 2. Decision

We will implement a **Multi-Homed Client Architecture with Primary/Secondary Relay Scoping**. 

### 2.1 Multi-Multiplexing Client Layer
*   The Rust Core (`synch-sync`) and FFI layers will be updated to manage a **Connection Pool** rather than a single `websocket_url`.
*   The `SynchClient` will support `add_relay(url)` and `remove_relay(url)`.
*   Data synchronization (CRDT Deltas) remains agnostic to the transport layer. A `DeltaBatch` can be sent or received via any connected relay. CRDT properties (idempotency, causality) naturally handle duplicate messages if received from multiple relays.

### 2.2 Relay Roles: Primary (Home) vs. Secondary (Guest)
Nodes will treat their connected relays differently based on their configured identity profile:
*   **Primary Relay (Home Node)**: The server where the User or Agent considers its "home base". This is typically where it was initialized or officially deployed.
    *   *Behavior*: Accepts `BIND_REQ` (contract formations). Broadcasts public `PresenceUpdate` (depending on L0-L4 perception level).
*   **Secondary Relays (Guest/Task Nodes)**: Other servers the node connects to, perhaps to interact with specific third-party teams, APIs, or open communities.
    *   *Behavior*: **Rejects** all general `BIND_REQ` attempts automatically. Only routes `SecuredMessage` (Double Ratchet traffic) for *already established* contracts. Operates strictly in **Perception Level L4 (Stealth Mode)** or similar restricted visibility to prevent unsolicited interactions.

### 2.3 Contract Routing Profile
When Alice and Bob form a contract, the `Contract` definition in Protobuf will be augmented to include **Routing Profiles**:
```protobuf
message Contract {
    // ... existing fields (ids, signatures) ...
    
    // The preferred relay URLs where this node can be reached
    repeated string requester_relays = 10;
    repeated string target_relays = 11;
}
```
When Alice's Agent needs to send a message to Bob, the client-side router checks:
1. Are Alice and Bob currently connected to any common relays? If yes, route through the fastest common relay.
2. If no common relay, the client can attempt to connect to one of Bob's `target_relays` just to deliver the encrypted payload, acting as a temporary bridge.

### 2.4 Server-Side Blind Routing
The Go Relay server logic remains largely the same. It is a "Dumb Pipe". The restriction on "not accepting contracts" is primarily enforced at the **Client (Rust/Agent) Level**, not the Server level. 
*   If an Agent on Relay B receives a `ContractSubmission` or `BIND_REQ`, the Agent's local Rust policy engine rejects it because Relay B is not its Primary Relay. 
*   This keeps the Relay server simple and zero-knowledge.

## 3. Consequences

### Positive
*   **True Portability**: Agents are not locked to a single company's or user's infrastructure. An enterprise Agent can join a public community relay to assist users while keeping its command-and-control locked to the enterprise private relay.
*   **Enhanced Privacy**: By adopting L4 (Stealth) on secondary nodes, Agents can listen and route data without exposing their existence to the public directory of that specific relay.
*   **Resilience**: If Relay A goes down, nodes can automatically fall back to Relay B if they have it configured, ensuring uninterrupted E2E communication for existing contracts.

### Negative / Challenges
*   **Client Complexity**: The Rust core must now handle connection state machines for multiple websockets, including reconnection backoff strategies per URL.
*   **Message Fan-out**: If Alice is on Relay A and B, and Bob is on Relay A and B, a naive client might send the same message twice. The client must implement sender-side deduplication or selective routing to save bandwidth, even though CRDT handles target-side deduplication safely.
*   **Discovery**: Finding a user becomes slightly more complex if you don't know their Primary Relay. Users will need to share their NodeID *along with* at least one reachable Relay URL.

## 4. Next Steps for Implementation (Step 6 Candidate)
1. **Protobuf Update**: Add `relays` array to `Contract` and `Presence` messages.
2. **Rust Core Update**: Refactor `SynchClient` or introduce `RelayManager` to hold multiple active WebSocket connections.
3. **Rust Policy Engine**: Add `RelayRole` (Primary/Secondary) to the local node configuration. Enforce the rule: `if incoming_bind_req.origin_relay != self.primary_relay { auto_reject() }`.
4. **Go Server**: No major changes required immediately, but can be optimized to handle cross-relay federation in the distant future if client-side bridging is insufficient.
