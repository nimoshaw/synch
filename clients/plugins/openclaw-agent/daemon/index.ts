import * as fs from 'fs';
import * as path from 'path';
import * as dotenv from 'dotenv';
import { fileURLToPath } from 'url';
import { SynchClient } from '../../vcp-agent/daemon/SynchClient';
import { NodeType } from '../../vcp-agent/daemon/proto/v1/sync.ts';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Load config
const configPath = path.join(__dirname, 'config.env');
if (fs.existsSync(configPath)) {
    dotenv.config({ path: configPath });
}

const CONFIG = {
    synchRelayUrl: process.env.SYNCH_RELAY_URL || 'ws://localhost:8080/ws',
    openclawApiUrl: process.env.OPENCLAW_API_URL || 'http://localhost:5000',
    debug: (process.env.DEBUG_MODE || 'false').toLowerCase() === 'true',
} as const;

/**
 * OpenClaw Agent — Synch 网络中的 OpenClaw 插件代理
 * 
 * 通过 SynchClient 连接到 Relay Server，
 * 监听来自管理员/用户的指令，调用 OpenClaw API 执行操作。
 * 
 * TODO: 实现以下功能
 * - 接收管理员加密消息中的 OpenClaw CLI 命令
 * - 调用 OpenClaw REST API 执行
 * - 将结果通过 E2EE 返回给请求者
 */
const client = new SynchClient(CONFIG.synchRelayUrl, NodeType.NODE_TYPE_AGENT, {
    onConnected: () => {
        console.log(`[OpenClaw] Connected to Synch Relay. NodeId: ${client.nodeId}`);
    },
    onDisconnected: () => {
        console.log(`[OpenClaw] Disconnected from Synch Relay`);
    },
    onMessage: (msg) => {
        if (msg.secured && msg.senderId) {
            console.log(`[OpenClaw] Received secured command from ${msg.senderId}`);
            // TODO: Parse command, execute via OpenClaw API, return result
        }
    },
});

// --- Graceful shutdown ---
process.on('SIGINT', () => { client.disconnect(); process.exit(0); });
process.on('SIGTERM', () => { client.disconnect(); process.exit(0); });

console.log(`[OpenClaw] OpenClaw Agent v0.1.0 starting...`);
console.log(`[OpenClaw] Relay: ${CONFIG.synchRelayUrl}`);
console.log(`[OpenClaw] API: ${CONFIG.openclawApiUrl}`);
client.connect();
