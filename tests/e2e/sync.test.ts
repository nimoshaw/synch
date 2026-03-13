import { describe, it, expect, beforeAll, afterAll } from 'vitest';
import WebSocket from 'ws';
import { SyncMessage, VaultHandshake, DeltaManifest, EntryOperation } from '../../shared/ts-core/src/proto/v1/sync';

describe('Vault Sync E2E', () => {
  const SERVER_URL = 'ws://localhost:8081/ws';
  let wsA: WebSocket;
  let wsB: WebSocket;

  const connect = (url: string): Promise<WebSocket> => {
    return new Promise((resolve, reject) => {
      const ws = new WebSocket(url);
      ws.on('open', () => resolve(ws));
      ws.on('error', reject);
      setTimeout(() => reject(new Error('Timeout connecting to ' + url)), 5000);
    });
  };

  const waitForMessage = (ws: WebSocket, predicate: (msg: SyncMessage) => boolean, timeout = 5000): Promise<SyncMessage> => {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        ws.off('message', onMessage);
        reject(new Error('Timeout waiting for message'));
      }, timeout);

      const onMessage = (data: Buffer) => {
        try {
          const msg = SyncMessage.decode(new Uint8Array(data));
          if (predicate(msg)) {
            clearTimeout(timer);
            ws.off('message', onMessage);
            resolve(msg);
          }
        } catch (e) {
          // ignore decode errors for now
        }
      };

      ws.on('message', onMessage);
    });
  };

  beforeAll(async () => {
    // Wait for containers to be ready
    await new Promise(r => setTimeout(r, 2000));
  });

  afterAll(() => {
    [wsA, wsB].forEach(ws => {
      if (ws && ws.readyState === WebSocket.OPEN) ws.close();
    });
  });

  it('Scenario: A -> Relay -> B Synchronization', async () => {
    wsA = await connect(SERVER_URL);
    wsB = await connect(SERVER_URL);

    const vaultId = 'test-vault-e2e';
    
    // 1. Node A Handshake
    const handshakeA: SyncMessage = {
      $type: 'synch.v1.SyncMessage',
      senderId: 'node-A',
      handshake: {
        $type: 'synch.v1.VaultHandshake',
        vaultId,
        nodeId: 'node-A',
        localVersion: 0,
        versionVector: {},
        capabilities: ['delta'],
      }
    };
    wsA.send(SyncMessage.encode(handshakeA).finish());

    // 2. Node B Handshake & Wait for presence/updates
    const handshakeB: SyncMessage = {
      $type: 'synch.v1.SyncMessage',
      senderId: 'node-B',
      handshake: {
        $type: 'synch.v1.VaultHandshake',
        vaultId,
        nodeId: 'node-B',
        localVersion: 0,
        versionVector: {},
        capabilities: ['delta'],
      }
    };
    wsB.send(SyncMessage.encode(handshakeB).finish());

    // 3. Node A sends a Delta
    const deltaMsg: SyncMessage = {
      $type: 'synch.v1.SyncMessage',
      senderId: 'node-A',
      delta: {
        $type: 'synch.v1.DeltaManifest',
        vaultId,
        baseVersion: 0,
        targetVersion: 1,
        totalBytes: 11,
        changes: [{
          $type: 'synch.v1.EntryChange',
          path: 'hello.txt',
          operation: EntryOperation.ENTRY_OPERATION_CREATE,
          contentHash: new Uint8Array(32),
          deltaBytes: new TextEncoder().encode('Hello E2E'),
          oldPath: '',
          blockHashes: [],
          size: 11,
          modifiedAt: Date.now(),
          originNodeId: 'node-A',
          originSequence: 1,
        }],
        blocks: [],
        manifestSignature: undefined,
      }
    };
    wsA.send(SyncMessage.encode(deltaMsg).finish());

    // 4. Node B should receive the message relayed by Go server
    const received = await waitForMessage(wsB, m => !!m.delta);
    expect(received.delta).toBeDefined();
    expect(received.delta?.vaultId).toBe(vaultId);
    expect(received.delta?.changes[0].path).toBe('hello.txt');
    expect(new TextDecoder().decode(received.delta?.changes[0].deltaBytes)).toBe('Hello E2E');
  });

  it('Scenario: Network Fault & Reconnection', async () => {
    // Simulate Node A "crashing" (closing connection)
    wsA.close();
    await new Promise(r => setTimeout(r, 500));
    expect(wsA.readyState).toBe(WebSocket.CLOSED);

    // Node B should still be connected
    expect(wsB.readyState).toBe(WebSocket.OPEN);

    // Node A "restarts" and reconnects
    wsA = await connect(SERVER_URL);
    expect(wsA.readyState).toBe(WebSocket.OPEN);

    // Verify it can still communicate
    const ping: SyncMessage = {
      $type: 'synch.v1.SyncMessage',
      senderId: 'node-A',
      presence: {
        $type: 'synch.v1.PresenceUpdate',
        nodeId: 'node-A',
        status: 1, // ONLINE
        lastSeen: Date.now(),
        activeVaultId: 'test-vault-e2e',
      }
    };
    wsA.send(SyncMessage.encode(ping).finish());
    
    // Node B should receive presence update
    const msg = await waitForMessage(wsB, m => !!m.presence);
    expect(msg.presence).toBeDefined();
    expect(msg.presence?.nodeId).toBe('node-A');
  });
});
