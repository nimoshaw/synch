SYNCH 心契协议 v2.0 — 去中心化Agent通讯与多端同步架构
同步即契合，契约即连接。
Where synchronization becomes trust.

版本: v2.0
日期: 2026-03-13
状态: 架构定稿，进入实现阶段

一、核心定位演进
维度	v0.1 原始定位	v2.0 扩展定位
核心协议	Agent去中心化通讯	Agent通讯 + 多端数据同步
节点类型	Agent/Human/Bridge	新增：多端客户端作为一等节点
同步对象	消息、指令	消息、指令、文件/Vault数据
部署形态	服务端+Web	服务端 + 插件生态 + 移动端原生
设计哲学不变：人人可以是服务器，人人可以加入他人服务器，Agent跨域协作，心契相通。

二、架构全景（v2.0）
┌─────────────────────────────────────────────────────────────────────────┐
│                         心契网络层 (P2P + Relay)                         │
│    ┌─────────┐      ┌─────────┐      ┌─────────┐      ┌─────────┐      │
│    │  STUN   │◄────►│  TURN   │◄────►│  DHT    │◄────►│ 服务端   │      │
│    │  服务器  │      │  中继   │      │ 路由表  │      │ 联邦    │      │
│    └─────────┘      └─────────┘      └─────────┘      └────┬────┘      │
│                                                             │           │
│                        心契Hub WebSocket/gRPC               │           │
└─────────────────────────────────────────────────────────────┼───────────┘
                                                              │
┌─────────────────────────────────────────────────────────────┼───────────┐
│                      心契核心引擎 (Rust FFI)                  │           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │           │
│  │  加密模块   │  │  同步引擎   │  │    冲突解决(CRDT)    │  │           │
│  │ X25519+AES │  │ Delta/RSync │  │  LWW/Merge/Manual   │  │           │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │           │
│  │  身份认证   │  │  协议编解码 │  │    多服务端路由      │  │           │
│  │ Ed25519    │  │  Protobuf   │  │   心契关系图谱        │  │           │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │           │
└─────────────────────────────────────────────────────────────┼───────────┘
                                                              │
        ┌─────────────────────────┬─────────────────────────┬─┘
        ▼                         ▼                         ▼
   ┌─────────┐              ┌─────────┐                ┌─────────┐
   │ 插件端   │              │ 移动端   │                │ 服务端   │
   │(JS/TS)  │              │(原生)   │                │(Relay)  │
   │         │              │         │                │         │
   │•vcp-agent│             │•Android │                │•个人节点│
   │•openclaw│              │•iOS     │                │•公共节点│
   │•openfang│              │         │                │•联邦节点│
   └─────────┘              └─────────┘                └─────────┘
三、项目目录结构（最终版）
D:\vcp\projects\Synch\
│
├── 📁 .github/                          # CI/CD 工作流
│   └── workflows/
│       ├── build-core.yml               # Rust core 多平台编译
│       ├── build-android.yml
│       ├── build-ios.yml
│       ├── build-plugins.yml
│       └── release.yml
│
├── 📁 docs/                             # 协议文档与架构决策
│   ├── ADR-001-architecture.md          # 本文件
│   ├── ADR-002-crypto.md                # 加密与密钥体系
│   ├── ADR-003-sync-engine.md           # 同步引擎设计
│   ├── ADR-004-multi-server.md          # 多服务端心契
│   ├── ADR-005-client-types.md          # 客户端类型与能力
│   └── api/
│       ├── websocket-v1.md
│       ├── protobuf-v1.md
│       └── error-codes.md
│
├── 📁 proto/                            # === 契约核心 ===
│   └── v1/
│       ├── synch.proto                  # 主协议（消息、身份、路由）
│       ├── crypto.proto                 # 加密原语
│       ├── sync.proto                   # 数据同步引擎
│       ├── relation.proto               # 心契关系模型
│       └── vault.proto                  # Vault数据抽象
│   └── buf.yaml
│
├── 📁 core/                             # === 心契核心引擎 (Rust) ===
│   ├── Cargo.toml                       # Workspace根
│   ├── crates/
│   │   ├── synch-crypto/                # Ed25519/X25519, AES-GCM, Argon2
│   │   ├── synch-identity/              # 节点身份、证书、心契验证
│   │   ├── synch-sync/                  # Delta算法、RSync、版本向量
│   │   ├── synch-net/                   # WebSocket、P2P打洞、多服务端路由
│   │   ├── synch-storage/               # SQLite抽象、文件系统、加密存储
│   │   ├── synch-vault/                 # Vault语义、权限、ACL
│   │   └── synch-ffi/                   # UniFFI绑定生成
│   │       ├── uniffi.toml
│   │       └── src/lib.rs
│   │
│   └── bindings/                        # 生成的FFI绑定
│       ├── android/                     # Kotlin AAR
│       └── ios/                         # Swift XCFramework
│
├── 📁 server/                           # === 心契Hub服务端 (Go) ===
│   ├── cmd/
│   │   ├── relay/                       # 中继服务器
│   │   │   └── main.go
│   │   └── admin/                       # 管理后台API
│   │       └── main.go
│   ├── internal/
│   │   ├── websocket/                   # WS网关、连接管理
│   │   ├── signaling/                   # P2P信令协调
│   │   ├── turn/                        # TURN中继（可选）
│   │   ├── federation/                  # 服务端联邦协议
│   │   ├── auth/                        # 心契证书验证
│   │   └── rate/                        # 限流与配额
│   └── pkg/
│       └── proto/                       # 生成的Go代码
│
├── 📁 clients/                          # === 所有客户端实现 ===
│   │
│   ├── 📁 plugins/                      # 插件生态（JS/TS）
│   │   ├── 📁 vcp-agent/                🎯 P0 - VCP生态核心
│   │   │   ├── manifest.json            # VCP插件规范 v2
│   │   │   ├── package.json
│   │   │   ├── tsconfig.json
│   │   │   ├── vite.config.ts
│   │   │   └── src/
│   │   │       ├── index.ts             # 插件入口
│   │   │       ├── SynchClient.ts       # 封装@synch/ts-core
│   │   │       ├── ui/
│   │   │       │   ├── Panel.tsx        # VCP面板主界面
│   │   │       │   ├── VaultBrowser.tsx # Vault文件浏览
│   │   │       │   ├── SyncStatus.tsx   # 同步状态指示器
│   │   │       │   └── settings/
│   │   │       ├── hooks/
│   │   │       │   ├── useSynch.ts
│   │   │       │   ├── useVault.ts
│   │   │       │   └── useMultiServer.ts
│   │   │       └── styles/
│   │   │
│   │   ├── 📁 openclaw-agent/           🎯 P0 - OpenClaw生态核心
│   │   │   ├── manifest.json            # OpenClaw插件规范
│   │   │   └── src/                     # 结构同上，UI适配OC设计系统
│   │   │
│   │   └── 📁 openfang-agent/           # 预留 - OpenFang生态
│   │       └── ...
│   │
│   ├── 📁 mobile/                       # 移动端原生
│   │   ├── 📁 android/                  🎯 P0 - Android
│   │   │   ├── app/
│   │   │   │   ├── src/main/
│   │   │   │   │   ├── java/com/synch/
│   │   │   │   │   │   ├── SynchApplication.kt
│   │   │   │   │   │   ├── service/SynchSyncService.kt  # 后台同步服务
│   │   │   │   │   │   ├── sync/SyncManager.kt          # FFI封装
│   │   │   │   │   │   ├── ui/
│   │   │   │   │   │   └── di/AppModule.kt
│   │   │   │   │   └── jniLibs/arm64-v8a/               # Rust .so
│   │   │   │   ├── res/
│   │   │   │   └── AndroidManifest.xml
│   │   │   ├── build.gradle.kts
│   │   │   └── gradle.properties
│   │   │
│   │   └── 📁 ios/                      🎯 P0 - iOS
│   │       ├── Synch/
│   │       │   ├── SynchApp.swift
│   │       │   ├── SyncManager.swift                    # FFI封装
│   │       │   ├── BackgroundSync.swift                 # BGTaskScheduler
│   │       │   ├── Views/
│   │       │   ├── ViewModels/
│   │       │   └── Info.plist
│   │       ├── SynchCore/                               # 生成的Swift绑定
│   │       └── Synch.xcodeproj
│   │
│   ├── 📁 desktop/                      # 桌面端（预留扩展）
│   │   └── linux/
│   │       ├── generic/                 # AppImage/Flatpak（MVP）
│   │       ├── ubuntu/                  # deb包（如需要）
│   │       └── debian/                  # 独立源（如需要）
│   │
│   └── 📁 web/                          # Web/PWA（预留）
│       └── portal/                      # 浏览器访问的管理界面
│
├── 📁 shared/                           # 跨端共享代码
│   ├── 📁 ts-core/                      # TypeScript同步核心
│   │   ├── src/
│   │   │   ├── SynchClient.ts           # 统一客户端接口
│   │   │   ├── MultiServerRouter.ts     # 多服务端路由
│   │   │   ├── Protocol.ts              # Protobuf编解码
│   │   │   ├── Crypto.ts                # WASM加密包装
│   │   │   ├── VaultAPI.ts              # Vault操作抽象
│   │   │   └── types/
│   │   ├── package.json
│   │   └── tsconfig.json
│   │
│   └── 📁 wasm/                         # 备用：Rust→WASM
│       └── pkg/                         # wasm-pack输出
│
├── 📁 tools/                            # 开发工具
│   ├── gen-device-key/                  # 设备密钥生成器
│   ├── gen-synch-cert/                  # 心契证书签发工具
│   └── test-vault/                      # 测试用Vault数据
│
├── Taskfile.yml                         # 统一构建入口
├── docker-compose.yml                   # 本地Relay服务器
├── README.md                            # 快速开始
└── CONTRIBUTING.md                      # 开发指南
四、节点类型扩展（v2.0）
类型	标识	新增能力	典型形态
Agent节点	agent://{id}	工具调用、RAG记忆、Vault同步	AI助手、自动化脚本
Human节点	user://{id}	自然语言、决策、多端管理	人、管理员
Bridge节点	bridge://{id}	协议转换、外部系统对接	微信机器人、邮件网关
Plugin节点	plugin://{id} ⭐	宿主环境集成、UI渲染、受限权限	vcp-agent, openclaw-agent
Mobile节点	mobile://{id} ⭐	后台同步、推送通知、离线优先	Android/iOS App
五、多服务端心契（保留并扩展v0.1）
客户端配置（v2.0格式）
{
  "profile": {
    "nodeType": "plugin",
    "nodeId": "jarvis-vcp-plugin",
    "displayName": "Jarvis-VCP",
    "platform": "vcp-agent",
    "capabilities": ["chat", "tools", "rag-local", "vault-sync", "ui-panel"]
  },
  
  "servers": [
    {
      "id": "home-fnos",
      "name": "家里心契",
      "url": "wss://home.synch.local:8081",
      "role": "primary",
      "autoConnect": true,
      "vaults": ["/vaults/personal", "/vaults/work"],
      "relations": ["owner"]
    },
    {
      "id": "friend-bob",
      "name": "Bob的心契",
      "url": "wss://bob.synch.link:8081",
      "role": "guest",
      "autoConnect": true,
      "inviteCode": "SYNCH-XXXXXX",
      "vaults": ["/shared/project-alpha"],
      "permissions": {
        "vaults": { "/shared/project-alpha": "write" },
        "memory": "service-proxy-only"
      },
      "relations": ["authorized-user"]
    }
  ],
  
  "sync": {
    "conflictStrategy": "ask",      // auto-local | auto-remote | ask
    "bandwidthLimit": 0,            // 0=无限制 (KB/s)
    "mobileSync": "wifi-only"       // always | wifi-only | manual
  }
}
六、Vault同步核心协议
Vault作为一等实体
Vault ID = Blake3(vault_root_path + owner_pubkey)[0:16]

Vault结构：
/vaults/personal/
  ├── .synch/                      # 心契元数据（隐藏）
  │   ├── manifest.json            # Vault配置、ACL
  │   ├── version_vector           # 全局版本状态
  │   └── delta_log/               # 增量日志（可清理）
  │
  ├── notes/                       # 用户数据
  ├── projects/
  └── ...
同步状态机
┌─────────┐    connect     ┌─────────┐    auth      ┌─────────┐
│  IDLE   │ ─────────────► │CONNECTING│ ──────────► │AUTHENTICATING│
└─────────┘                └─────────┘             └────┬────┘
                                                        │
    ◄───────────────────────────────────────────────────┘
    auth_success + vault_list
    │
    ▼
┌─────────┐    select vault    ┌─────────┐    delta exchange    ┌─────────┐
│  SYNCED │ ◄───────────────── │SYNCING  │ ──────────────────► │INSPECT  │
│  (idle) │                    │ (active)│                      │DELTAS   │
└────┬────┘                    └────┬────┘                      └────┬────┘
     │                              ▲                               │
     │    local change              │    apply deltas               │
     └──────────────────────────────┘◄───────────────────────────────┘
七、核心Protobuf协议（v2.0）
// proto/v1/synch.proto
syntax = "proto3";
package synch.v1;

// ========== 基础身份 ==========

message NodeIdentity {
  string node_id = 1;             // 格式: "agent://pk" | "plugin://pk" | "mobile://pk"
  bytes public_key = 2;           // Ed25519 公钥 (32 bytes)
  string node_type = 3;           // "agent" | "human" | "plugin" | "mobile" | "bridge"
  string platform = 4;            // "vcp-agent" | "openclaw" | "android" | "ios" | ...
  repeated string capabilities = 5; // 能力声明
  uint64 registered_at = 6;
}

// ========== 多服务端会话 ==========

message ServerEndpoint {
  string server_id = 1;
  string url = 2;
  ServerRole role = 3;            // PRIMARY | SECONDARY | GUEST
  map<string, VaultPermission> vault_access = 4;
}

enum ServerRole {
  ROLE_UNSPECIFIED = 0;
  PRIMARY = 1;                    // 主服务端，拥有完整权限
  SECONDARY = 2;                  // 备用服务端
  GUEST = 3;                      // 访客身份，受限权限
}

message VaultPermission {
  string vault_id = 1;
  PermissionLevel level = 2;      // READ | WRITE | ADMIN
  bytes vault_key_encrypted = 3;  // 用设备密钥加密的Vault密钥
}

// ========== Vault同步消息 ==========

message SyncMessage {
  oneof payload {
    VaultHandshake handshake = 1;
    DeltaManifest delta = 2;
    Ack ack = 3;
    ConflictDetected conflict = 4;
    SyncProgress progress = 5;
    PresenceUpdate presence = 6;
  }
}

message VaultHandshake {
  string vault_id = 1;
  uint64 local_version = 2;       // 客户端已知版本
  repeated string capabilities = 3; // 支持的同步特性
}

message DeltaManifest {
  string vault_id = 1;
  uint64 base_version = 2;
  uint64 target_version = 3;
  repeated EntryChange changes = 4;
  repeated BlockReference blocks = 5;  // 大文件分块
}

// EntryChange, BlockReference, ConflictDetected 等定义
// 继承自 v0.1 并扩展版本向量支持
八、首发型号技术栈
型号	技术栈	核心依赖	构建输出
vcp-agent	TypeScript + Vite + VCP SDK v2	@vcp/sdk, @synch/ts-core, libsodium-wasm	.vcplugin (单文件)
openclaw-agent	TypeScript + Vite + OpenClaw SDK	@openclaw/sdk, @synch/ts-core	.ocplugin (单文件)
Android	Kotlin + Jetpack Compose + Rust FFI	synch-core (AAR via UniFFI), ktor, room, work-runtime	.apk / .aab
iOS	Swift + SwiftUI + Rust FFI	SynchCore (XCFramework), Combine, BGTaskScheduler	.ipa / App Store
九、关键设计决策（ADR汇总）
ADR	决策	理由
ADR-001	Rust核心 + UniFFI	加密性能、跨平台复用、内存安全、移动原生绑定
ADR-002	Ed25519身份 + X25519加密	现代标准、紧凑签名、前向安全
ADR-003	版本向量 > 逻辑时钟	离线场景正确性、因果一致性、冲突检测
ADR-004	Vault作为权限边界	用户心智模型、细粒度授权、并行同步
ADR-005	多服务端原生支持	非事后补丁，协议层设计
ADR-006	记忆主权绝对化	服务代理模式，记忆绝不出境（v2.0不变）
十、演进路线（更新）
阶段	目标	交付物
MVP (v0.2)	四端首发型号	vcp-agent + openclaw-agent + Android + iOS，单服务端，基础Vault同步
v0.5	多服务端心契	客户端连多服务器、心契密钥跨域、服务端联邦
v1.0	去中心化网络	自动路由、P2P直连优化、Bridge节点
v2.0	生态扩展	可选记忆共享（满足安全条件）、Linux桌面、Web门户
十一、快速开始
# 1. 环境准备
cd D:\vcp\projects\Synch
task init                    # 安装Rust, Buf, Android SDK等

# 2. 生成协议与绑定
task proto:gen               # protoc → Go/TS/Rust
task ffi:gen                 # UniFFI → Kotlin/Swift

# 3. 构建核心
task build:core              # Rust core (Android/iOS通用)

# 4. 构建首发型号
task build:vcp-agent         # dist/vcp-agent.vcplugin
task build:openclaw-agent    # dist/openclaw-agent.ocplugin
task build:android           # dist/synch-android.apk
task build:ios               # build/Synch.ipa

# 5. 本地测试
task relay:up                # docker-compose up
task test:integration        # 端到端Vault同步测试
十二、模型调用建议（整合原建议）
阶段	模块	主力	验证
1	协议状态机、多服务端路由	Claude Opus 4.6	-
2	Rust core 实现	Claude Sonnet 4.6 / qwen3-coder	Claude审关键安全模块
3	插件端 (TS)	qwen3-coder-plus	-
4	移动端 (Kotlin/Swift)	qwen3-coder-plus	-
5	安全审计	Claude Opus 4.6	-
必须双人验证: 加密模块、身份认证、心契证书、跨服务端握手

"心契一点，万物互联。"

设计定稿: 2026-03-13
版本: SYNCH v2.0SYNCH v2.0 Architecture
