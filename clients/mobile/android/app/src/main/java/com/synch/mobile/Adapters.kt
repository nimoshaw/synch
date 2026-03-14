package com.synch.mobile

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.recyclerview.widget.DiffUtil
import androidx.recyclerview.widget.ListAdapter
import androidx.recyclerview.widget.RecyclerView

/**
 * NodeAdapter — 在线节点列表适配器
 */
class NodeAdapter : ListAdapter<NodeInfo, NodeAdapter.ViewHolder>(NodeDiffCallback()) {

    class ViewHolder(view: View) : RecyclerView.ViewHolder(view) {
        val nodeId: TextView = view.findViewById(R.id.tvNodeId)
        val nodeType: TextView = view.findViewById(R.id.tvNodeType)
        val nodeLevel: TextView = view.findViewById(R.id.tvNodeLevel)
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
        val view = LayoutInflater.from(parent.context)
            .inflate(R.layout.item_node, parent, false)
        return ViewHolder(view)
    }

    override fun onBindViewHolder(holder: ViewHolder, position: Int) {
        val node = getItem(position)
        holder.nodeId.text = node.displayName.ifEmpty { node.nodeId.take(24) }
        holder.nodeType.text = node.nodeType
        holder.nodeLevel.text = node.perceptionLevel
    }
}

class NodeDiffCallback : DiffUtil.ItemCallback<NodeInfo>() {
    override fun areItemsTheSame(oldItem: NodeInfo, newItem: NodeInfo) = oldItem.nodeId == newItem.nodeId
    override fun areContentsTheSame(oldItem: NodeInfo, newItem: NodeInfo) = oldItem == newItem
}

/**
 * LogAdapter — 事件日志列表适配器
 */
class LogAdapter : ListAdapter<EventEntry, LogAdapter.ViewHolder>(LogDiffCallback()) {

    class ViewHolder(view: View) : RecyclerView.ViewHolder(view) {
        val timestamp: TextView = view.findViewById(R.id.tvTimestamp)
        val message: TextView = view.findViewById(R.id.tvMessage)
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): ViewHolder {
        val view = LayoutInflater.from(parent.context)
            .inflate(R.layout.item_log, parent, false)
        return ViewHolder(view)
    }

    override fun onBindViewHolder(holder: ViewHolder, position: Int) {
        val event = getItem(position)
        holder.timestamp.text = event.timestamp
        holder.message.text = event.message
    }
}

class LogDiffCallback : DiffUtil.ItemCallback<EventEntry>() {
    override fun areItemsTheSame(oldItem: EventEntry, newItem: EventEntry) =
        oldItem.timestamp == newItem.timestamp && oldItem.message == newItem.message
    override fun areContentsTheSame(oldItem: EventEntry, newItem: EventEntry) = oldItem == newItem
}
