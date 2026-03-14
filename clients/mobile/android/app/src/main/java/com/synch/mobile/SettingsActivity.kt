package com.synch.mobile

import android.content.Intent
import android.os.Bundle
import android.widget.Button
import android.widget.EditText
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity

/**
 * SettingsActivity — 服务器配置界面
 *
 * 用户在此配置 Relay Server 地址、管理员 Token、节点显示名。
 * 配置保存到 SharedPreferences，服务启动时读取。
 */
class SettingsActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_settings)

        supportActionBar?.title = "设置"
        supportActionBar?.setDisplayHomeAsUpEnabled(true)

        val prefs = getSharedPreferences("synch_config", MODE_PRIVATE)

        val etServerUrl = findViewById<EditText>(R.id.etServerUrl)
        val etAdminToken = findViewById<EditText>(R.id.etAdminToken)
        val etDisplayName = findViewById<EditText>(R.id.etDisplayName)
        val etNodeId = findViewById<EditText>(R.id.etNodeId)
        val btnSave = findViewById<Button>(R.id.btnSave)
        val btnTest = findViewById<Button>(R.id.btnTest)

        // Load current config
        etServerUrl.setText(prefs.getString("server_url", "ws://192.168.1.100:8080"))
        etAdminToken.setText(prefs.getString("admin_token", ""))
        etDisplayName.setText(prefs.getString("display_name", "Android 用户"))

        // Show the node ID (read-only)
        val nodeId = SynchRepository.getOrCreateNodeId(this)
        etNodeId.setText(nodeId)
        etNodeId.isEnabled = false

        btnSave.setOnClickListener {
            val serverUrl = etServerUrl.text.toString().trim()
            val adminToken = etAdminToken.text.toString().trim()
            val displayName = etDisplayName.text.toString().trim()

            if (serverUrl.isEmpty()) {
                etServerUrl.error = "请输入服务器地址"
                return@setOnClickListener
            }

            // Validate URL format
            if (!serverUrl.startsWith("ws://") && !serverUrl.startsWith("wss://")) {
                etServerUrl.error = "地址必须以 ws:// 或 wss:// 开头"
                return@setOnClickListener
            }

            prefs.edit()
                .putString("server_url", serverUrl)
                .putString("admin_token", adminToken)
                .putString("display_name", displayName)
                .apply()

            Toast.makeText(this, "配置已保存", Toast.LENGTH_SHORT).show()

            // Restart service to apply new config
            stopService(Intent(this, SynchService::class.java))
            SynchRepository.updateConnectionState(ConnectionState.DISCONNECTED)
            SynchRepository.addEvent("配置已更新，重新连接...")

            finish()
        }

        btnTest.setOnClickListener {
            val url = etServerUrl.text.toString().trim()
            if (url.isEmpty()) {
                Toast.makeText(this, "请先输入服务器地址", Toast.LENGTH_SHORT).show()
                return@setOnClickListener
            }

            // Quick HTTP health check
            val httpUrl = url
                .replace("ws://", "http://")
                .replace("wss://", "https://")
                .replace("/ws", "/health")

            Toast.makeText(this, "正在测试 $httpUrl ...", Toast.LENGTH_SHORT).show()

            Thread {
                try {
                    val client = okhttp3.OkHttpClient.Builder()
                        .connectTimeout(5, java.util.concurrent.TimeUnit.SECONDS)
                        .build()
                    val request = okhttp3.Request.Builder().url(httpUrl).build()
                    val response = client.newCall(request).execute()
                    val body = response.body?.string() ?: ""
                    runOnUiThread {
                        if (response.isSuccessful) {
                            Toast.makeText(this, "✓ 连接成功\n$body", Toast.LENGTH_LONG).show()
                        } else {
                            Toast.makeText(this, "✗ 服务器错误: ${response.code}", Toast.LENGTH_LONG).show()
                        }
                    }
                } catch (e: Exception) {
                    runOnUiThread {
                        Toast.makeText(this, "✗ 连接失败: ${e.message}", Toast.LENGTH_LONG).show()
                    }
                }
            }.start()
        }
    }

    override fun onSupportNavigateUp(): Boolean {
        finish()
        return true
    }
}
