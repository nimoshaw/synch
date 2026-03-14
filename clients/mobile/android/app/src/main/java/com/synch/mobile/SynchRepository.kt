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
     * sendHandshake — 发送握手消息到 relay
     * 
     * NOTE: 完整的 protobuf 集成需要生成的 Java 类。
     * 此处使用手动构建的 protobuf 字节作为临时方案。
     * 待 buf generate 生成 Java/Kotlin protobuf 类后替换。
     */
    fun sendHandshake(ws: WebSocket, nodeId: String, displayName: String) {
        activeWebSocket = ws
        // Build a minimal SyncMessage with sender_id field (field 7, string)
        // protobuf wire format: field_number << 3 | wire_type
        // field 7 string: (7 << 3 | 2) = 58 = 0x3A
        val senderBytes = nodeId.toByteArray(Charsets.UTF_8)
        val msg = ByteArray(2 + senderBytes.size)
        msg[0] = 0x3A.toByte() // field 7, wire type 2 (length-delimited)
        msg[1] = senderBytes.size.toByte()
        System.arraycopy(senderBytes, 0, msg, 2, senderBytes.size)
        
        ws.send(okio.ByteString.of(*msg))
        addEvent("握手已发送: $nodeId")
    }

    /**
     * handleIncomingMessage — 处理来自 relay 的 protobuf 消息
     */
    fun handleIncomingMessage(data: ByteArray) {
        // Simple protobuf field parsing for SyncMessage
        // Parse sender_id (field 7) and payload type
        try {
            var offset = 0
            var senderId = ""
            var payloadType = "unknown"

            while (offset < data.size) {
                val tag = data[offset].toInt() and 0xFF
                val fieldNumber = tag shr 3
                val wireType = tag and 0x07
                offset++

                when (wireType) {
                    0 -> { // Varint
                        while (offset < data.size && data[offset].toInt() and 0x80 != 0) offset++
                        offset++
                    }
                    2 -> { // Length-delimited
                        if (offset >= data.size) break
                        val length = data[offset].toInt() and 0xFF
                        offset++
                        when (fieldNumber) {
                            7 -> senderId = String(data, offset, minOf(length, data.size - offset), Charsets.UTF_8)
                            1 -> payloadType = "Handshake"
                            2 -> payloadType = "Delta"
                            6 -> {
                                payloadType = "Presence"
                                // Try to extract node_id from presence (basic parsing)
                                if (length > 2 && offset + length <= data.size) {
                                    val presenceData = data.copyOfRange(offset, offset + length)
                                    parsePresenceUpdate(presenceData, senderId)
                                }
                            }
                            10 -> payloadType = "Secured"
                            11 -> payloadType = "Contract"
                        }
                        offset += length
                    }
                    else -> break // Unknown wire type
                }
            }

            if (senderId.isNotEmpty() || payloadType != "unknown") {
                addEvent("← [$payloadType] from ${senderId.take(20)}")
            }
        } catch (e: Exception) {
            addEvent("消息解析失败: ${e.message}")
        }
    }

    private fun parsePresenceUpdate(data: ByteArray, fallbackNodeId: String) {
        // Extract node_id from PresenceUpdate (field 1, string)
        try {
            var offset = 0
            var nodeId = fallbackNodeId
            while (offset < data.size) {
                val tag = data[offset].toInt() and 0xFF
                val fieldNumber = tag shr 3
                val wireType = tag and 0x07
                offset++
                when (wireType) {
                    0 -> { while (offset < data.size && data[offset].toInt() and 0x80 != 0) offset++; offset++ }
                    2 -> {
                        if (offset >= data.size) break
                        val len = data[offset].toInt() and 0xFF
                        offset++
                        if (fieldNumber == 1) {
                            nodeId = String(data, offset, minOf(len, data.size - offset), Charsets.UTF_8)
                        }
                        offset += len
                    }
                    else -> break
                }
            }
            if (nodeId.isNotEmpty()) {
                updateNode(NodeInfo(nodeId = nodeId, lastSeen = System.currentTimeMillis()))
            }
        } catch (_: Exception) {}
    }

    /**
     * sendMessage — 发送原始二进制消息到 relay
     */
    fun sendRawMessage(data: ByteArray): Boolean {
        val ws = activeWebSocket ?: return false
        return ws.send(okio.ByteString.of(*data))
    }
}
