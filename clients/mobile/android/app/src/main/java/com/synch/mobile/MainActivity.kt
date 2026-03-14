package com.synch.mobile

import android.content.Intent
import android.os.Bundle
import android.view.Menu
import android.view.MenuItem
import android.view.View
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.google.android.material.floatingactionbutton.FloatingActionButton

/**
 * MainActivity — Synch 心契主界面
 *
 * 显示连接状态、在线节点列表和最近事件日志。
 * 用户可以从这里进入设置页配置服务器，或管理节点/契约。
 */
class MainActivity : AppCompatActivity() {

    private lateinit var statusIndicator: View
    private lateinit var statusText: TextView
    private lateinit var nodeCountText: TextView
    private lateinit var nodeRecyclerView: RecyclerView
    private lateinit var logRecyclerView: RecyclerView
    private lateinit var emptyView: TextView
    private lateinit var fabConnect: FloatingActionButton

    private lateinit var nodeAdapter: NodeAdapter
    private lateinit var logAdapter: LogAdapter

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        setSupportActionBar(findViewById(R.id.toolbar))
        supportActionBar?.title = "Synch 心契"

        // Bind views
        statusIndicator = findViewById(R.id.statusIndicator)
        statusText = findViewById(R.id.statusText)
        nodeCountText = findViewById(R.id.nodeCountText)
        nodeRecyclerView = findViewById(R.id.nodeRecyclerView)
        logRecyclerView = findViewById(R.id.logRecyclerView)
        emptyView = findViewById(R.id.emptyView)
        fabConnect = findViewById(R.id.fabConnect)

        // Setup adapters
        nodeAdapter = NodeAdapter()
        nodeRecyclerView.layoutManager = LinearLayoutManager(this)
        nodeRecyclerView.adapter = nodeAdapter

        logAdapter = LogAdapter()
        logRecyclerView.layoutManager = LinearLayoutManager(this).apply {
            stackFromEnd = true
        }
        logRecyclerView.adapter = logAdapter

        // Observe connection state
        SynchRepository.connectionState.observe(this) { state ->
            updateConnectionUI(state)
        }

        // Observe online nodes
        SynchRepository.onlineNodes.observe(this) { nodes ->
            nodeAdapter.submitList(nodes)
            nodeCountText.text = "在线节点: ${nodes.size}"
            emptyView.visibility = if (nodes.isEmpty()) View.VISIBLE else View.GONE
            nodeRecyclerView.visibility = if (nodes.isEmpty()) View.GONE else View.VISIBLE
        }

        // Observe event log
        SynchRepository.eventLog.observe(this) { events ->
            logAdapter.submitList(events)
            if (events.isNotEmpty()) {
                logRecyclerView.scrollToPosition(events.size - 1)
            }
        }

        // FAB: connect/disconnect toggle
        fabConnect.setOnClickListener {
            val state = SynchRepository.connectionState.value
            if (state == ConnectionState.CONNECTED || state == ConnectionState.CONNECTING) {
                stopSynchService()
            } else {
                startSynchService()
            }
        }

        // Auto-connect if server URL is configured
        val prefs = getSharedPreferences("synch_config", MODE_PRIVATE)
        val serverUrl = prefs.getString("server_url", null)
        if (!serverUrl.isNullOrEmpty()) {
            startSynchService()
        }
    }

    override fun onCreateOptionsMenu(menu: Menu): Boolean {
        menuInflater.inflate(R.menu.main_menu, menu)
        return true
    }

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        return when (item.itemId) {
            R.id.action_settings -> {
                startActivity(Intent(this, SettingsActivity::class.java))
                true
            }
            R.id.action_admin -> {
                startActivity(Intent(this, AdminActivity::class.java))
                true
            }
            else -> super.onOptionsItemSelected(item)
        }
    }

    private fun updateConnectionUI(state: ConnectionState) {
        when (state) {
            ConnectionState.CONNECTED -> {
                statusIndicator.setBackgroundColor(ContextCompat.getColor(this, R.color.status_online))
                statusText.text = "已连接"
            }
            ConnectionState.CONNECTING, ConnectionState.RECONNECTING -> {
                statusIndicator.setBackgroundColor(ContextCompat.getColor(this, R.color.status_connecting))
                statusText.text = "连接中..."
            }
            ConnectionState.DISCONNECTED -> {
                statusIndicator.setBackgroundColor(ContextCompat.getColor(this, R.color.status_offline))
                statusText.text = "未连接"
            }
        }
    }

    private fun startSynchService() {
        val prefs = getSharedPreferences("synch_config", MODE_PRIVATE)
        val url = prefs.getString("server_url", null)
        if (url.isNullOrEmpty()) {
            Toast.makeText(this, "请先在设置中配置服务器地址", Toast.LENGTH_LONG).show()
            startActivity(Intent(this, SettingsActivity::class.java))
            return
        }
        ContextCompat.startForegroundService(this, Intent(this, SynchService::class.java))
    }

    private fun stopSynchService() {
        stopService(Intent(this, SynchService::class.java))
        SynchRepository.updateConnectionState(ConnectionState.DISCONNECTED)
    }
}
