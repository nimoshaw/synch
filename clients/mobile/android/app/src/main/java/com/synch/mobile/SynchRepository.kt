package com.synch.mobile

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import androidx.lifecycle.LiveData
import androidx.lifecycle.MutableLiveData
import okhttp3.WebSocket
import java.text.SimpleDateFormat
import java.util.*

/**
 * ConnectionState — WebSocket 连接状态
 */
enum class ConnectionState {
    DISCONNECTED, CONNECTING, CONNECTED, RECONNECTING
}

/**
 * NodeInfo — 在线节点信息
 */
data class NodeInfo(
    val nodeId: String,
    val nodeType: String = "Unknown",
    val perceptionLevel: String = "L0",
    val displayName: String = "",
    val capabilities: List<String> = emptyList(),
    val lastSeen: Long = System.currentTimeMillis()
)

/**
 * EventEntry — 事件日志条目
 */
data class EventEntry(
    val timestamp: String,
    val message: String
)

/**
 * SynchRepository — 全局数据仓库（单例）
 *
 * 管理连接状态、在线节点列表、事件日志。
 * 通过 LiveData 提供响应式 UI 更新。
 */
object SynchRepository {
    private const val TAG = "SynchRepo"
    private const val MAX_LOG_ENTRIES = 200
    private const val MAX_NODES = 500

    private val mainHandler = Handler(Looper.getMainLooper())
    private val dateFormat = SimpleDateFormat("HH:mm:ss", Locale.getDefault())

    // --- Connection State ---
    private val _connectionState = MutableLiveData(ConnectionState.DISCONNECTED)
    val connectionState: LiveData<ConnectionState> = _connectionState

    // --- Online Nodes ---
    private val _onlineNodes = MutableLiveData<List<NodeInfo>>(emptyList())
    val onlineNodes: LiveData<List<NodeInfo>> = _onlineNodes
    private val nodesMap = mutableMapOf<String, NodeInfo>()

    // --- Event Log ---
    private val _eventLog = MutableLiveData<List<EventEntry>>(emptyList())
    val eventLog: LiveData<List<EventEntry>> = _eventLog
    private val events = mutableListOf<EventEntry>()

    // --- WebSocket reference for sending ---
    private var activeWebSocket: WebSocket? = null

    fun updateConnectionState(state: ConnectionState) {
        mainHandler.post { _connectionState.value = state }
    }

    fun addEvent(message: String) {
        val entry = EventEntry(
            timestamp = dateFormat.format(Date()),
            message = message
        )
        mainHandler.post {
            events.add(entry)
            if (events.size > MAX_LOG_ENTRIES) {
                events.removeAt(0)
            }
            _eventLog.value = events.toList()
        }
        Log.d(TAG, "[${entry.timestamp}] $message")
    }

    fun updateNode(node: NodeInfo) {
        mainHandler.post {
            nodesMap[node.nodeId] = node
            _onlineNodes.value = nodesMap.values.toList()
        }
    }

    fun removeNode(nodeId: String) {
        mainHandler.post {
            nodesMap.remove(nodeId)
            _onlineNodes.value = nodesMap.values.toList()
        }
    }

    fun clearNodes() {
        mainHandler.post {
            nodesMap.clear()
            _onlineNodes.value = emptyList()
        }
    }

    /**
     * getOrCreateNodeId — 获取或生成持久化的节点 ID
     * 格式: mobile://<random-hex>
     */
    fun getOrCreateNodeId(context: Context): String {
        val prefs = context.getSharedPreferences("synch_config", Context.MODE_PRIVATE)
        var nodeId = prefs.getString("node_id", null)
        if (nodeId.isNullOrEmpty()) {
            nodeId = "mobile://" + UUID.randomUUID().toString().replace("-", "").take(16)
            prefs.edit().putString("node_id", nodeId).apply()
        }
        return nodeId
    }

    /**
     * sendHandshake — 使用 protobuf 生成类发送握手消息
     */
    fun sendHandshake(ws: WebSocket, nodeId: String, displayName: String) {
        activeWebSocket = ws

        val handshake = synch.v1.Sync.VaultHandshake.newBuilder()
            .setNodeId(nodeId)
            .setNodeType(synch.v1.Synch.NodeType.NODE_TYPE_MOBILE)
            .addAllCapabilities(listOf("e2ee", "presence", "sync"))
            .build()

        val msg = synch.v1.Sync.SyncMessage.newBuilder()
            .setSenderId(nodeId)
            .setHandshake(handshake)
            .build()

        ws.send(okio.ByteString.of(*msg.toByteArray()))
        addEvent("握手已发送: $nodeId (capabilities: e2ee, presence, sync)")
    }

    /**
     * handleIncomingMessage — 使用 protobuf 生成类解析消息
     */
    fun handleIncomingMessage(data: ByteArray) {
        try {
            val msg = synch.v1.Sync.SyncMessage.parseFrom(data)
            val senderId = msg.senderId

            when {
                msg.hasHandshake() -> {
                    val h = msg.handshake
                    addEvent("← [Handshake] from ${senderId.take(20)} type=${h.nodeType.name}")
                    if (h.nodeId.isNotEmpty()) {
                        updateNode(NodeInfo(
                            nodeId = h.nodeId,
                            nodeType = h.nodeType.name,
                            capabilities = h.capabilitiesList,
                            lastSeen = System.currentTimeMillis()
                        ))
                    }
                }
                msg.hasPresence() -> {
                    val p = msg.presence
                    val pNodeId = p.nodeId.ifEmpty { senderId }
                    addEvent("← [Presence] ${pNodeId.take(20)} → ${p.status.name}")
                    if (p.status == synch.v1.Sync.PresenceStatus.PRESENCE_STATUS_OFFLINE) {
                        removeNode(pNodeId)
                    } else if (pNodeId.isNotEmpty()) {
                        updateNode(NodeInfo(
                            nodeId = pNodeId,
                            perceptionLevel = p.perceptionLevel.name,
                            lastSeen = if (p.lastSeen > 0) p.lastSeen else System.currentTimeMillis()
                        ))
                    }
                }
                msg.hasSecured() -> {
                    val s = msg.secured
                    addEvent("← [Secured] from ${senderId.take(20)} contract=${s.contractId.take(12)}")
                }
                msg.hasContractSubmission() -> {
                    addEvent("← [Contract] from ${senderId.take(20)} id=${msg.contractSubmission.contractId.take(12)}")
                }
                msg.hasDelta() -> {
                    addEvent("← [Delta] from ${senderId.take(20)} vault=${msg.delta.vaultId.take(12)}")
                }
                else -> {
                    if (senderId.isNotEmpty()) {
                        addEvent("← [Unknown] from ${senderId.take(20)}")
                    }
                }
            }
        } catch (e: Exception) {
            addEvent("消息解析失败: ${e.message}")
            Log.e(TAG, "Failed to parse protobuf message", e)
        }
    }

    /**
     * sendPresence — 发送在线状态到 relay
     */
    fun sendPresence(nodeId: String, status: synch.v1.Sync.PresenceStatus) {
        val presence = synch.v1.Sync.PresenceUpdate.newBuilder()
            .setNodeId(nodeId)
            .setStatus(status)
            .setLastSeen(System.currentTimeMillis())
            .build()

        val msg = synch.v1.Sync.SyncMessage.newBuilder()
            .setSenderId(nodeId)
            .setPresence(presence)
            .build()

        sendRawMessage(msg.toByteArray())
    }

    /**
     * sendRawMessage — 发送原始二进制消息到 relay
     */
    fun sendRawMessage(data: ByteArray): Boolean {
        val ws = activeWebSocket ?: return false
        return ws.send(okio.ByteString.of(*data))
    }
}
