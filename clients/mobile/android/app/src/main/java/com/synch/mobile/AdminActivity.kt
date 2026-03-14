package com.synch.mobile

import android.os.Bundle
import android.widget.Button
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import org.json.JSONObject

/**
 * AdminActivity — 服务器管理界面
 *
 * 远程调用 Admin API 查看服务器状态、在线节点、契约列表。
 * 需要在设置中配置 Admin Token。
 */
class AdminActivity : AppCompatActivity() {

    private lateinit var statusView: TextView
    private lateinit var nodesView: TextView
    private lateinit var contractsView: TextView

    private val httpClient = okhttp3.OkHttpClient.Builder()
        .connectTimeout(10, java.util.concurrent.TimeUnit.SECONDS)
        .readTimeout(10, java.util.concurrent.TimeUnit.SECONDS)
        .build()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_admin)

        supportActionBar?.title = "服务器管理"
        supportActionBar?.setDisplayHomeAsUpEnabled(true)

        statusView = findViewById(R.id.tvAdminStatus)
        nodesView = findViewById(R.id.tvAdminNodes)
        contractsView = findViewById(R.id.tvAdminContracts)

        findViewById<Button>(R.id.btnRefreshStatus).setOnClickListener { fetchStatus() }
        findViewById<Button>(R.id.btnRefreshNodes).setOnClickListener { fetchNodes() }
        findViewById<Button>(R.id.btnRefreshContracts).setOnClickListener { fetchContracts() }

        // Auto-fetch on open
        fetchStatus()
        fetchNodes()
    }

    private fun getBaseUrl(): String {
        val prefs = getSharedPreferences("synch_config", MODE_PRIVATE)
        val wsUrl = prefs.getString("server_url", "") ?: ""
        return wsUrl
            .replace("ws://", "http://")
            .replace("wss://", "https://")
            .replace("/ws", "")
    }

    private fun getAdminToken(): String {
        val prefs = getSharedPreferences("synch_config", MODE_PRIVATE)
        return prefs.getString("admin_token", "") ?: ""
    }

    private fun fetchStatus() {
        adminRequest("/api/admin/status") { body ->
            try {
                val json = JSONObject(body)
                val text = buildString {
                    appendLine("连接客户端: ${json.optInt("connected_clients")}")
                    appendLine("总契约数: ${json.optInt("total_contracts")}")
                    appendLine("离线队列: ${json.optInt("offline_queues")}")
                    appendLine("内存: ${String.format("%.1f", json.optDouble("memory_mb"))} MB")
                    appendLine("Goroutines: ${json.optInt("goroutines")}")
                }
                runOnUiThread { statusView.text = text }
            } catch (e: Exception) {
                runOnUiThread { statusView.text = "解析失败: ${e.message}" }
            }
        }
    }

    private fun fetchNodes() {
        adminRequest("/api/admin/nodes") { body ->
            try {
                val json = JSONObject(body)
                val nodes = json.optJSONArray("nodes")
                if (nodes == null || nodes.length() == 0) {
                    runOnUiThread { nodesView.text = "无在线节点" }
                    return@adminRequest
                }
                val text = buildString {
                    for (i in 0 until nodes.length()) {
                        val node = nodes.getJSONObject(i)
                        appendLine("• ${node.optString("node_id").take(24)}")
                        appendLine("  类型: ${node.optString("node_type")}  感知: ${node.optString("perception_level")}")
                    }
                }
                runOnUiThread { nodesView.text = text }
            } catch (e: Exception) {
                runOnUiThread { nodesView.text = "解析失败: ${e.message}" }
            }
        }
    }

    private fun fetchContracts() {
        adminRequest("/api/admin/contracts") { body ->
            try {
                val json = JSONObject(body)
                val contracts = json.optJSONArray("contracts")
                if (contracts == null || contracts.length() == 0) {
                    runOnUiThread { contractsView.text = "无契约" }
                    return@adminRequest
                }
                val text = buildString {
                    for (i in 0 until contracts.length()) {
                        val c = contracts.getJSONObject(i)
                        appendLine("• ${c.optString("contract_id").take(16)}")
                        appendLine("  状态: ${c.optString("status")}")
                    }
                }
                runOnUiThread { contractsView.text = text }
            } catch (e: Exception) {
                runOnUiThread { contractsView.text = "解析失败: ${e.message}" }
            }
        }
    }

    private fun adminRequest(path: String, onSuccess: (String) -> Unit) {
        val baseUrl = getBaseUrl()
        if (baseUrl.isEmpty()) {
            Toast.makeText(this, "未配置服务器地址", Toast.LENGTH_SHORT).show()
            return
        }

        Thread {
            try {
                val requestBuilder = okhttp3.Request.Builder()
                    .url("$baseUrl$path")
                    .get()

                val token = getAdminToken()
                if (token.isNotEmpty()) {
                    requestBuilder.addHeader("Authorization", "Bearer $token")
                }

                val response = httpClient.newCall(requestBuilder.build()).execute()
                val body = response.body?.string() ?: ""

                if (response.isSuccessful) {
                    onSuccess(body)
                } else {
                    runOnUiThread {
                        Toast.makeText(this, "请求失败: ${response.code} $body", Toast.LENGTH_LONG).show()
                    }
                }
            } catch (e: Exception) {
                runOnUiThread {
                    Toast.makeText(this, "请求异常: ${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    override fun onSupportNavigateUp(): Boolean {
        finish()
        return true
    }
}
