package com.synch.mobile

import android.app.*
import android.content.Intent
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.lifecycle.LifecycleService
import okhttp3.*
import okio.ByteString
import java.util.concurrent.TimeUnit

/**
 * SynchService — 后台 WebSocket 前台服务
 *
 * 管理与 Synch Relay Server 的持久 WebSocket 连接。
 * 运行为前台服务以保持 Android 不杀后台进程。
 * 支持自动重连（指数退避），protobuf 消息收发。
 */
class SynchService : LifecycleService() {

    companion object {
        private const val TAG = "SynchService"
        private const val CHANNEL_ID = "synch_channel"
        private const val NOTIFICATION_ID = 1
    }

    private var webSocket: WebSocket? = null
    private val client = OkHttpClient.Builder()
        .readTimeout(0, TimeUnit.MILLISECONDS) // No timeout for WebSocket
        .pingInterval(30, TimeUnit.SECONDS)    // Keep-alive ping
        .build()

    private var reconnectAttempts = 0
    private val maxReconnectDelay = 60_000L
    private val baseReconnectDelay = 2_000L

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        startForeground(NOTIFICATION_ID, buildNotification("正在连接..."))
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        super.onStartCommand(intent, flags, startId)
        connect()
        return START_STICKY // Restart if killed
    }

    override fun onBind(intent: Intent): IBinder? {
        super.onBind(intent)
        return null
    }

    override fun onDestroy() {
        disconnect()
        super.onDestroy()
    }

    private fun connect() {
        val prefs = getSharedPreferences("synch_config", MODE_PRIVATE)
        val serverUrl = prefs.getString("server_url", null)
        if (serverUrl.isNullOrEmpty()) {
            Log.e(TAG, "No server URL configured")
            SynchRepository.addEvent("错误: 未配置服务器地址")
            stopSelf()
            return
        }

        // Ensure URL ends with /ws
        val wsUrl = if (serverUrl.endsWith("/ws")) serverUrl
                    else "$serverUrl/ws"

        SynchRepository.updateConnectionState(ConnectionState.CONNECTING)
        updateNotification("连接中...")
        SynchRepository.addEvent("正在连接 $wsUrl ...")

        val request = Request.Builder()
            .url(wsUrl)
            .build()

        webSocket = client.newWebSocket(request, object : WebSocketListener() {

            override fun onOpen(ws: WebSocket, response: Response) {
                Log.i(TAG, "WebSocket connected to $wsUrl")
                SynchRepository.updateConnectionState(ConnectionState.CONNECTED)
                SynchRepository.addEvent("已连接到服务器")
                reconnectAttempts = 0
                updateNotification("已连接")

                // Send initial handshake with our node ID
                val nodeId = SynchRepository.getOrCreateNodeId(this@SynchService)
                val displayName = prefs.getString("display_name", "Android 用户") ?: "Android 用户"
                SynchRepository.addEvent("节点 ID: $nodeId")

                // Send identification message (protobuf binary)
                // For now, send a simple text-encoded identification
                // Full protobuf integration requires generated Java classes
                SynchRepository.sendHandshake(ws, nodeId, displayName)
            }

            override fun onMessage(ws: WebSocket, bytes: ByteString) {
                Log.d(TAG, "Received binary message: ${bytes.size()} bytes")
                SynchRepository.handleIncomingMessage(bytes.toByteArray())
            }

            override fun onMessage(ws: WebSocket, text: String) {
                Log.d(TAG, "Received text message: $text")
                SynchRepository.addEvent("收到: $text")
            }

            override fun onClosing(ws: WebSocket, code: Int, reason: String) {
                Log.i(TAG, "WebSocket closing: $code $reason")
                ws.close(1000, null)
            }

            override fun onClosed(ws: WebSocket, code: Int, reason: String) {
                Log.i(TAG, "WebSocket closed: $code $reason")
                SynchRepository.updateConnectionState(ConnectionState.DISCONNECTED)
                SynchRepository.addEvent("连接已关闭 ($code)")
                updateNotification("已断开")
                scheduleReconnect()
            }

            override fun onFailure(ws: WebSocket, t: Throwable, response: Response?) {
                Log.e(TAG, "WebSocket failure: ${t.message}", t)
                SynchRepository.updateConnectionState(ConnectionState.DISCONNECTED)
                SynchRepository.addEvent("连接失败: ${t.message}")
                updateNotification("连接失败")
                scheduleReconnect()
            }
        })
    }

    private fun disconnect() {
        webSocket?.close(1000, "Service stopping")
        webSocket = null
        SynchRepository.updateConnectionState(ConnectionState.DISCONNECTED)
        SynchRepository.addEvent("已断开连接")
    }

    private fun scheduleReconnect() {
        if (SynchRepository.connectionState.value == ConnectionState.DISCONNECTED) {
            val delay = minOf(
                baseReconnectDelay * (1L shl minOf(reconnectAttempts, 5)),
                maxReconnectDelay
            )
            reconnectAttempts++
            SynchRepository.updateConnectionState(ConnectionState.RECONNECTING)
            SynchRepository.addEvent("${delay / 1000}秒后重连 (第${reconnectAttempts}次)")
            updateNotification("${delay / 1000}秒后重连...")

            android.os.Handler(mainLooper).postDelayed({
                if (SynchRepository.connectionState.value == ConnectionState.RECONNECTING) {
                    connect()
                }
            }, delay)
        }
    }

    // --- Notification Management ---

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Synch 同步服务",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "保持 Synch 后台连接"
                setShowBadge(false)
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(content: String): Notification {
        val intent = Intent(this, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        val pendingIntent = PendingIntent.getActivity(
            this, 0, intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("Synch 心契")
            .setContentText(content)
            .setSmallIcon(android.R.drawable.ic_menu_share)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .setSilent(true)
            .build()
    }

    private fun updateNotification(content: String) {
        val manager = getSystemService(NotificationManager::class.java)
        manager.notify(NOTIFICATION_ID, buildNotification(content))
    }
}
