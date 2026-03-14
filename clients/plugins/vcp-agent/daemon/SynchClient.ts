import WebSocket from 'ws';
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';
import { SyncMessage, PresenceUpdate, PresenceStatus, VaultHandshake } from './proto/v1/sync';
import { NodeType } from './proto/v1/synch';
import { loadOrCreateExchangeKeys, PeerKeyStore, deriveSharedKey, decrypt, type KeyPair } from './crypto.js';

type ConnectionState = "disconnected" | "connecting" | "connected" | "reconnecting";

export interface SynchClientEvents {
  onConnected?: () => void;
  onDisconnected?: () => void;
  onMessage?: (msg: SyncMessage) => void;
  onPresence?: (update: PresenceUpdate) => void;
}

/**
 * SynchClient — Synch Relay WebSocket 客户端
 * 
 * 生产级特性:
 * - 持久化 nodeId (写入 ~/.synch/node.id)
 * - 连接后自动发送 VaultHandshake + PresenceUpdate
 * - 指数退避重连
 * - 事件回调接口
 */
export class SynchClient {
  private ws: WebSocket | null = null;
  private url: string;
  private reconnectAttempts: number = 0;
  private maxReconnectDelay: number = 30000;
  private baseReconnectDelay: number = 1000;
  private reconnectTimer: NodeJS.Timeout | null = null;
  private events: SynchClientEvents;
  
  public state: ConnectionState = "disconnected";
  public nodeId: string;
  public nodeType: NodeType;

  // Track online peers
  public peers: Map<string, PresenceUpdate> = new Map();

  // E2EE key management
  public exchangeKeys: KeyPair;
  public peerKeyStore: PeerKeyStore = new PeerKeyStore();

  constructor(url: string, nodeType: NodeType = NodeType.NODE_TYPE_AGENT, events: SynchClientEvents = {}) {
    this.url = url;
    this.nodeType = nodeType;
    this.events = events;
    this.nodeId = this.loadOrCreateNodeId();
    this.exchangeKeys = loadOrCreateExchangeKeys();
    console.log(`[SynchClient] X25519 public key: ${Buffer.from(this.exchangeKeys.publicKey).toString('hex').substring(0, 16)}...`);
  }

  /**
   * loadOrCreateNodeId - 从 ~/.synch/node.id 读取或创建持久 nodeId
   */
  private loadOrCreateNodeId(): string {
    const synchDir = path.join(os.homedir(), '.synch');
    const idFile = path.join(synchDir, 'node.id');
    
    try {
      if (fs.existsSync(idFile)) {
        const stored = fs.readFileSync(idFile, 'utf-8').trim();
        if (stored.length > 0) {
          console.log(`[SynchClient] Loaded persistent nodeId: ${stored}`);
          return stored;
        }
      }
    } catch (e) {
      // Fall through to create new ID
    }

    // Generate new persistent ID
    const id = `plugin://vcp-agent-${Date.now().toString(36)}-${Math.random().toString(36).substring(2, 8)}`;
    try {
      fs.mkdirSync(synchDir, { recursive: true });
      fs.writeFileSync(idFile, id, 'utf-8');
      console.log(`[SynchClient] Created persistent nodeId: ${id}`);
    } catch (e) {
      console.warn(`[SynchClient] Could not persist nodeId: ${e}`);
    }
    return id;
  }

  public connect() {
    if (this.ws && (this.ws.readyState === WebSocket.CONNECTING || this.ws.readyState === WebSocket.OPEN)) {
      return;
    }

    if (this.state !== "reconnecting") {
      this.state = "connecting";
    }
    
    console.log(`[SynchClient] Connecting to ${this.url} (Attempt ${this.reconnectAttempts + 1})...`);

    try {
      this.ws = new WebSocket(this.url);
      
      this.ws.on('open', () => {
        console.log("[SynchClient] ✓ Connected to Synch Relay");
        this.state = "connected";
        this.reconnectAttempts = 0;
        
        // Step 1: Send VaultHandshake
        this.sendHandshake();
        
        // Step 2: Announce presence
        this.sendPresence(PresenceStatus.PRESENCE_STATUS_ONLINE);

        this.events.onConnected?.();
      });

      this.ws.on('close', (code, reason) => {
        const reasonStr = reason?.toString() || '';
        console.log(`[SynchClient] Connection closed: ${code} ${reasonStr}`);
        this.state = "disconnected";
        this.events.onDisconnected?.();
        this.scheduleReconnect();
      });

      this.ws.on('error', (error) => {
        console.error("[SynchClient] WebSocket error:", error.message);
      });

      this.ws.on('message', (data: Buffer) => {
        try {
          const syncMsg = SyncMessage.decode(new Uint8Array(data));
          this.handleMessage(syncMsg);
        } catch (e) {
          console.error("[SynchClient] Failed to decode message:", e);
        }
      });
    } catch (e) {
      console.error("[SynchClient] Failed to create WebSocket:", e);
      this.state = "disconnected";
      this.scheduleReconnect();
    }
  }

  /**
   * sendHandshake - 发送 VaultHandshake 消息，声明节点类型和能力
   */
  private sendHandshake() {
    const msg = SyncMessage.create({
      senderId: this.nodeId,
      handshake: {
        nodeId: this.nodeId,
        nodeType: this.nodeType,
        capabilities: [
          // 协议能力
          'e2ee', 'contract', 'presence', 'sync',
          // Agent 服务能力 (供客户端展示可用服务)
          'service:secure-message',
          'service:contract-manager',
          'service:presence-query',
        ],
      }
    });
    this.send(msg);
    console.log(`[SynchClient] Handshake sent: type=${NodeType[this.nodeType]}, id=${this.nodeId}`);
  }

  /**
   * sendPresence - 发送在线/离线状态到 Relay
   */
  public sendPresence(status: PresenceStatus) {
    const msg = SyncMessage.create({
      senderId: this.nodeId,
      presence: {
        nodeId: this.nodeId,
        status: status,
        perceptionLevel: 0, // L0 = open
        lastSeen: Date.now(),
      }
    });
    this.send(msg);
  }

  private scheduleReconnect() {
    if (this.reconnectTimer) return;

    this.state = "reconnecting";
    const delay = Math.min(
      this.baseReconnectDelay * Math.pow(2, this.reconnectAttempts),
      this.maxReconnectDelay
    );
    
    console.log(`[SynchClient] Reconnecting in ${delay}ms...`);
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.reconnectAttempts++;
      this.connect();
    }, delay);
  }

  public disconnect() {
    // Announce offline before disconnecting
    if (this.state === 'connected') {
      this.sendPresence(PresenceStatus.PRESENCE_STATUS_OFFLINE);
    }

    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    if (this.ws) {
      this.ws.removeAllListeners();
      this.ws.close(1000, 'Client shutting down');
      this.ws = null;
    }
    this.state = "disconnected";
    this.peers.clear();
  }

  private handleMessage(msg: SyncMessage) {
    // Route based on payload type
    if (msg.handshake) {
      console.log(`[SynchClient] ← Handshake from: ${msg.handshake.nodeId} (type: ${NodeType[msg.handshake.nodeType]})`);
    }
    
    if (msg.presence) {
      const p = msg.presence;
      if (p.status === PresenceStatus.PRESENCE_STATUS_OFFLINE) {
        this.peers.delete(p.nodeId);
      } else {
        this.peers.set(p.nodeId, p);
      }
      console.log(`[SynchClient] ← Presence: ${p.nodeId} → ${PresenceStatus[p.status]} (peers: ${this.peers.size})`);
      this.events.onPresence?.(p);
    }

    if (msg.secured) {
      const secured = msg.secured;
      const senderId = msg.senderId;
      console.log(`[SynchClient] ← Secured message from ${senderId} (contract: ${secured.contractId})`);

      // Extract and store sender's public key for future messages
      if (secured.payload?.senderPublicKey && secured.payload.senderPublicKey.length === 32) {
        this.peerKeyStore.set(senderId, secured.payload.senderPublicKey);
      }

      // Attempt decryption
      if (secured.payload?.ciphertext && secured.payload?.nonce && this.exchangeKeys) {
        const peerPubKey = this.peerKeyStore.get(senderId);
        if (peerPubKey) {
          try {
            const sharedKey = deriveSharedKey(this.exchangeKeys.secretKey, peerPubKey);
            const plaintext = decrypt(sharedKey, secured.payload.ciphertext, secured.payload.nonce);
            const text = new TextDecoder().decode(plaintext);
            console.log(`[SynchClient] ✓ Decrypted message from ${senderId}: ${text.substring(0, 100)}`);
          } catch (e) {
            console.warn(`[SynchClient] ⚠ Decryption failed (may be plaintext):`, (e as Error).message);
          }
        }
      }
    }

    // Forward all messages to callback
    this.events.onMessage?.(msg);
  }

  public send(msg: SyncMessage) {
    if (this.ws && this.state === 'connected') {
      const data = SyncMessage.encode(msg).finish();
      this.ws.send(data);
    } else {
      console.warn(`[SynchClient] Cannot send: state=${this.state}`);
    }
  }

  /**
   * isConnected - 检查是否已连接
   */
  public get isConnected(): boolean {
    return this.state === 'connected';
  }
}
