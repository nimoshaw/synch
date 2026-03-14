import { SynchClient } from './SynchClient';
import { SyncMessage, NodeType, Contract, ContractStatus } from './proto/v1/sync.ts';

/**
 * ToolHandlers — VCP 工具执行处理器
 * 
 * 从 index.ts 提取的模块化处理器，每个工具一个 async 函数。
 * 通过 SynchClient 发送消息到 Relay Server。
 */

interface SecureMessageArgs {
  targetId: string;
  message: string;
}

interface ContractManagerArgs {
  action: 'list' | 'create' | 'accept' | 'reject';
  contractId?: string;
  targetNodeId?: string;
}

interface PresenceQueryArgs {
  nodeId?: string;
}

/**
 * handleToolExecution — 路由工具调用到对应的处理函数
 */
export async function handleToolExecution(
  toolName: string,
  toolArgs: Record<string, unknown>,
  client: SynchClient
): Promise<Record<string, unknown>> {
  if (!client.isConnected) {
    throw new Error('Synch Relay 未连接。请检查网络或 Relay 状态。');
  }

  switch (toolName) {
    case 'SynchSecureMessage':
      return handleSecureMessage(toolArgs as unknown as SecureMessageArgs, client);
    case 'SynchContractManager':
      return handleContractManager(toolArgs as unknown as ContractManagerArgs, client);
    case 'SynchPresenceQuery':
      return handlePresenceQuery(toolArgs as unknown as PresenceQueryArgs, client);
    default:
      throw new Error(`未知工具: ${toolName}`);
  }
}

/**
 * handleSecureMessage — 发送 E2EE 加密消息
 * 
 * ⚠️ WIP: 当前实现为**明文传输**，ciphertext 字段直接包含原始消息。
 * TODO: 集成真正的 AES-256-GCM 加密 (需先通过 Contract 建立共享密钥)
 */
async function handleSecureMessage(args: SecureMessageArgs, client: SynchClient): Promise<Record<string, unknown>> {
  if (!args.targetId) throw new Error('缺少 targetId 参数');
  if (!args.message) throw new Error('缺少 message 参数');

  console.warn(`[ToolHandler] ⚠️ E2EE 尚未实现 — 消息将以明文发送至 ${args.targetId}`);
  
  // TODO: 实现真正的加密流程:
  // 1. 查找与 targetId 的活跃 Contract
  // 2. 通过 Contract 的共享密钥派生 AES-256-GCM 密钥
  // 3. 生成随机 nonce, 加密消息
  // 4. 填充正确的 senderPublicKey 和 signature
  const msg = SyncMessage.create({
    senderId: client.nodeId,
    targetId: args.targetId,
    secured: {
      contractId: "",  // TODO: 关联实际 Contract ID
      senderFingerprint: "",
      payload: {
        ciphertext: Buffer.from(args.message),  // ⚠️ 明文! 待加密实现后替换
        nonce: new Uint8Array(12),               // ⚠️ 占位,应为随机 nonce
        senderPublicKey: new Uint8Array(32),      // ⚠️ 占位,应为发送方公钥
        algorithm: 3,  // AES_256_GCM
        aadHash: new Uint8Array(0),
        contractId: "",
        ratchetSeq: 0,
        ratchetKey: new Uint8Array(0),
        prevChainLength: 0
      },
      timestamp: Date.now(),
      signature: new Uint8Array(64)              // ⚠️ 占位,应为实际签名
    }
  });

  client.send(msg);

  return {
    status: 'success',
    warning: '⚠️ E2EE 尚未实现，消息以明文发送',
    message: `消息已排队发送至 ${args.targetId}（明文模式）`,
    synchNodeId: client.nodeId,
    peersOnline: client.peers.size,
    timestamp: new Date().toISOString()
  };
}

/**
 * handleContractManager — 契约管理操作
 */
async function handleContractManager(args: ContractManagerArgs, client: SynchClient): Promise<Record<string, unknown>> {
  if (!args.action) throw new Error('缺少 action 参数');

  console.log(`[ToolHandler] Contract action: ${args.action}`);

  switch (args.action) {
    case 'list': {
      // Request contract list from relay (via handshake capabilities)
      const msg = SyncMessage.create({
        senderId: client.nodeId,
        handshake: {
          nodeId: client.nodeId,
          capabilities: ['contract-list']
        }
      });
      client.send(msg);
      return {
        status: 'success',
        action: 'list',
        message: '已请求契约列表，等待 Relay 回复',
        timestamp: new Date().toISOString()
      };
    }
    case 'create': {
      if (!args.targetNodeId) throw new Error('创建契约需要 targetNodeId 参数');
      const contractId = `contract-${Date.now().toString(36)}`;
      const msg = SyncMessage.create({
        senderId: client.nodeId,
        contractSubmission: {
          contractId: contractId,
          requesterId: Buffer.from(client.nodeId),
          targetId: Buffer.from(args.targetNodeId),
          status: ContractStatus.CONTRACT_STATUS_PENDING,
          createdAt: Date.now(),
          expiresAt: Date.now() + 86400000 * 30, // 30 days
        }
      });
      client.send(msg);
      return {
        status: 'success',
        action: 'create',
        contractId: contractId,
        targetNodeId: args.targetNodeId,
        message: `已创建契约 ${contractId} → ${args.targetNodeId}`,
        timestamp: new Date().toISOString()
      };
    }
    case 'accept':
    case 'reject': {
      if (!args.contractId) throw new Error(`${args.action} 操作需要 contractId 参数`);
      const msg = SyncMessage.create({
        senderId: client.nodeId,
        handshake: {
          nodeId: client.nodeId,
          capabilities: [`contract-${args.action}`, `cid-${args.contractId}`]
        }
      });
      client.send(msg);
      return {
        status: 'success',
        action: args.action,
        contractId: args.contractId,
        message: `契约 ${args.contractId} 已${args.action === 'accept' ? '接受' : '拒绝'}`,
        timestamp: new Date().toISOString()
      };
    }
    default:
      throw new Error(`不支持的契约操作: ${args.action}`);
  }
}

/**
 * handlePresenceQuery — 查询在线状态
 */
async function handlePresenceQuery(args: PresenceQueryArgs, client: SynchClient): Promise<Record<string, unknown>> {
  console.log(`[ToolHandler] Querying presence for: ${args.nodeId || 'all'}`);

  if (args.nodeId) {
    // Check if we know this peer
    const peer = client.peers.get(args.nodeId);
    return {
      status: 'success',
      nodeId: args.nodeId,
      online: !!peer,
      lastSeen: peer ? Number(peer.lastSeen) : null,
      perceptionLevel: peer ? peer.perceptionLevel : null,
      totalPeersKnown: client.peers.size,
      timestamp: new Date().toISOString()
    };
  }

  // Return all known peers
  const peerList = Array.from(client.peers.entries()).map(([id, p]) => ({
    nodeId: id,
    status: p.status,
    lastSeen: Number(p.lastSeen),
    perceptionLevel: p.perceptionLevel
  }));

  return {
    status: 'success',
    peers: peerList,
    totalPeers: peerList.length,
    synchState: client.state,
    timestamp: new Date().toISOString()
  };
}
