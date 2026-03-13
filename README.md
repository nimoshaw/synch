# SYNCH 心契协议 v2.0

> 同步即契合，契约即连接。  
> Where synchronization becomes trust.

SYNCH 是一款去中心化的 Agent 通讯与多端数据同步协议。它旨在让 AI Agent 与人类用户之间实现跨平台、跨服务端的无缝连接与数据同步。

## 核心架构

- **Core (Rust)**: 提供高性能、内存安全的加密与同步逻辑。
- **Server (Go)**: 实现中继 (Relay) 与联邦 (Federation) 协议。
- **Clients**:
  - **Plugins**: VCP-Agent, OpenClaw (TypeScript)
  - **Mobile**: Android (Kotlin), iOS (Swift)
- **Protocol**: 使用 Protobuf v3 定义跨端契约。

## 目录结构

- `core/`: Rust 核心引擎
- `server/`: Go 中继服务端
- `clients/`: 各种客户端实现
- `proto/`: Protobuf 协议定义
- `docs/`: 架构设计文档 (ADR)
- `.synch/`: 项目自动化与 Agent 指令中心

## 快速开始

1. **环境初始化**:
   ```bash
   task init
   ```
2. **生成协议代码**:
   ```bash
   task proto:gen
   ```
3. **构建核心库**:
   ```bash
   task build:core
   ```

## 开发者指南

请参考 `.synch/instructions/README.md` 获取 Agent 开发流细节。
