import * as fs from 'fs';
import * as path from 'path';
import * as dotenv from 'dotenv';
import { fileURLToPath } from 'url';
import WebSocket from 'ws';
import { SynchClient } from './SynchClient';
import { handleToolExecution } from './handlers';
import { NodeType } from './proto/v1/sync.ts';

// Define __dirname for ESM
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Load config
const configPath = path.join(__dirname, 'config.env');
if (fs.existsSync(configPath)) {
    dotenv.config({ path: configPath });
}

// --- Configuration ---
const CONFIG = {
    vcpMainServerUrl: process.env.VCP_MAIN_SERVER_URL || 'ws://localhost:3000',
    vcpKey: process.env.VCP_KEY || '',
    serverName: process.env.SERVER_NAME || 'SynchAgent-1',
    synchRelayUrl: process.env.SYNCH_RELAY_URL || 'ws://localhost:8080/ws',
    debug: (process.env.DEBUG_MODE || 'false').toLowerCase() === 'true',
} as const;

// --- Tool Manifests ---
const SYNCH_TOOLS = [
    {
        name: 'SynchSecureMessage',
        displayName: 'Synch 加密通信',
        description: '通过 Synch 中继发送端到端加密消息。',
        pluginType: 'synchronous',
        entryPoint: { type: 'builtin', command: 'internal' },
        communication: { protocol: 'direct', timeout: 30000 },
        capabilities: {
            invocationCommands: [{
                commandIdentifier: 'SynchSecureMessage',
                description: '发送 E2EE 加密消息。\n参数:\n- targetId (必需): 接收方节点 ID\n- message (必需): 消息内容',
                example: '<<<[TOOL_REQUEST]>>>\ntool_name:「始」SynchSecureMessage「末」,\ntargetId:「始」node-abc123「末」,\nmessage:「始」你好「末」\n<<<[END_TOOL_REQUEST]>>>'
            }]
        }
    },
    {
        name: 'SynchContractManager',
        displayName: 'Synch 契约管理',
        description: '管理网络中的加密契约（list/create/accept/reject）。',
        pluginType: 'synchronous',
        entryPoint: { type: 'builtin', command: 'internal' },
        communication: { protocol: 'direct', timeout: 30000 },
        capabilities: {
            invocationCommands: [{
                commandIdentifier: 'SynchContractManager',
                description: '管理契约。\n参数:\n- action (必需): list | create | accept | reject\n- contractId (可选): 指定契约 ID\n- targetNodeId (可选): 创建契约时的目标节点',
                example: '<<<[TOOL_REQUEST]>>>\ntool_name:「始」SynchContractManager「末」,\naction:「始」list「末」\n<<<[END_TOOL_REQUEST]>>>'
            }]
        }
    },
    {
        name: 'SynchPresenceQuery',
        displayName: 'Synch 在线状态查询',
        description: '查询网络中节点的在线状态和已知节点列表。',
        pluginType: 'synchronous',
        entryPoint: { type: 'builtin', command: 'internal' },
        communication: { protocol: 'direct', timeout: 15000 },
        capabilities: {
            invocationCommands: [{
                commandIdentifier: 'SynchPresenceQuery',
                description: '查询节点在线状态。\n参数:\n- nodeId (可选): 指定查询的节点 ID，不传则返回全部',
                example: '<<<[TOOL_REQUEST]>>>\ntool_name:「始」SynchPresenceQuery「末」\n<<<[END_TOOL_REQUEST]>>>'
            }]
        }
    }
];

// --- SynchClient Instance ---
const synchClient = new SynchClient(CONFIG.synchRelayUrl, NodeType.NODE_TYPE_AGENT, {
    onConnected: () => console.log(`[Daemon] Synch Relay connected. NodeId: ${synchClient.nodeId}`),
    onDisconnected: () => console.log(`[Daemon] Synch Relay disconnected`),
    onPresence: (p) => {
        if (CONFIG.debug) console.log(`[Daemon] Presence update: ${p.nodeId} → ${p.status}`);
    },
});

// --- VCP Daemon ---
class SynchVCPDaemon {
    private ws: WebSocket | null = null;
    private reconnectInterval = 5000;
    private maxReconnectInterval = 60000;
    private reconnectTimeoutId: ReturnType<typeof setTimeout> | null = null;
    private stopped = false;

    connect() {
        if (this.stopped) return;
        
        // Connect to Synch Relay first
        synchClient.connect();

        if (!CONFIG.vcpKey) {
            console.error(`[Daemon] VCP_KEY 未配置。请在 daemon/config.env 中设置。`);
            console.log(`[Daemon] 仅 Synch Relay 模式运行（无 VCP 集成）`);
            return;
        }

        const url = `${CONFIG.vcpMainServerUrl.replace(/^http/, 'ws')}/vcp-distributed-server/VCP_Key=${CONFIG.vcpKey}`;
        console.log(`[Daemon] Connecting to VCPToolBox...`);
        
        this.ws = new WebSocket(url);

        this.ws.on('open', () => {
            console.log(`[Daemon] ✓ Connected to VCPToolBox`);
            this.reconnectInterval = 5000;
            this.registerTools();
        });

        this.ws.on('message', (data) => {
            this.handleVCPMessage(data.toString());
        });

        this.ws.on('close', () => {
            console.log(`[Daemon] VCPToolBox disconnected`);
            this.scheduleReconnect();
        });

        this.ws.on('error', (err) => {
            console.error(`[Daemon] VCP WebSocket error: ${err.message}`);
        });
    }

    private registerTools() {
        this.send({
            type: 'register_tools',
            data: { serverName: CONFIG.serverName, tools: SYNCH_TOOLS }
        });
        console.log(`[Daemon] Registered ${SYNCH_TOOLS.length} tools with VCPToolBox`);
    }

    private async handleVCPMessage(raw: string) {
        try {
            const msg = JSON.parse(raw);
            if (CONFIG.debug) console.log(`[Daemon] ← VCP: ${msg.type}`);

            if (msg.type === 'execute_tool') {
                const { requestId, toolName, toolArgs } = msg.data;
                console.log(`[Daemon] → Executing: ${toolName} (${requestId})`);
                
                try {
                    const result = await handleToolExecution(toolName, toolArgs, synchClient);
                    this.send({
                        type: 'tool_result',
                        data: { requestId, status: 'success', result }
                    });
                } catch (err) {
                    const errorMessage = err instanceof Error ? err.message : String(err);
                    console.error(`[Daemon] Tool error: ${errorMessage}`);
                    this.send({
                        type: 'tool_result',
                        data: { requestId, status: 'error', error: errorMessage }
                    });
                }
            }
        } catch (e) {
            console.error(`[Daemon] Failed to parse VCP message:`, e);
        }
    }

    private send(payload: Record<string, unknown>) {
        if (this.ws && this.ws.readyState === WebSocket.OPEN) {
            this.ws.send(JSON.stringify(payload));
        }
    }

    private scheduleReconnect() {
        if (this.stopped) return;
        console.log(`[Daemon] VCP reconnect in ${this.reconnectInterval / 1000}s...`);
        if (this.reconnectTimeoutId) clearTimeout(this.reconnectTimeoutId);
        this.reconnectTimeoutId = setTimeout(() => this.connect(), this.reconnectInterval);
        this.reconnectInterval = Math.min(this.reconnectInterval * 2, this.maxReconnectInterval);
    }

    stop() {
        this.stopped = true;
        if (this.reconnectTimeoutId) clearTimeout(this.reconnectTimeoutId);
        synchClient.disconnect();
        if (this.ws) {
            this.ws.removeAllListeners('close');
            this.ws.close(1000, 'Daemon shutting down');
            this.ws = null;
        }
        console.log(`[Daemon] Graceful shutdown complete.`);
    }
}

// --- Main ---
const daemon = new SynchVCPDaemon();

process.on('SIGINT', () => { daemon.stop(); process.exit(0); });
process.on('SIGTERM', () => { daemon.stop(); process.exit(0); });

console.log(`[Daemon] Synch VCP Agent v0.2.0 starting...`);
console.log(`[Daemon] Relay: ${CONFIG.synchRelayUrl}`);
console.log(`[Daemon] VCP: ${CONFIG.vcpKey ? 'configured' : 'NOT configured (relay-only mode)'}`);
daemon.connect();
