# Synch 交互权限与契约架构开发计划 (Step 4)

本方案旨在实现 Synch 网络的核心权限模型与契约机制，将现有的“广播式中继”升级为“基于契约的端到端加密定向路由”。

## 一、 总体架构视角

### 1.1 现状分析 (As-Is)
- **中继服务器**：简单的 Hub 模式，所有连接的客户端都会收到所有广播消息。
- **加密层**：支持基础的 AES-256-GCM，但缺乏前向保密和契约绑定。
- **身份模型**：已定义基础的 `NodeIdentity`，但缺乏 `LORD`, `SUB_ADMIN` 等治理角色。

### 1.2 目标架构 (To-Be)
- **契约即路由**：中继服务器不再广播，而是根据消息中的 `ContractID` 和 `TargetID` 进行精准投递。
- **权限自治**：人类介入一次建立契约，Agent 续期执行。
- **端到端保密**：集成 Double Ratchet 算法，确保即使当前密钥泄露，历史消息依然安全。
- **离线可靠性**：中继服务器托管密文队列，支持 Agent 恢复后异步同步。

---

## 二、 阶段化开发路线图

### 第一阶段：协议与加密原语增强 (Phase 1)
> 目标：打好安全基础，支持契约表达。
- [x] **Protobuf 升级**：
    - `synch.proto`: 引入 `Contract` 结构与 `SecuredMessage` 包装。
    - `crypto.proto`: 在 `EncryptedPayload` 中增加 `ratchet_seq` 与 `contract_id`。
- [x] **Identity 核心库升级 (Rust)**：
    - 扩展 `NodeType` 枚举（Admin, Lord, SubAdmin）。
    - 兼容新协议的身份生成逻辑。
- [x] **契约核心逻辑 (Rust)**：
    - 实现 `Contract` 的双向签名与验证。
    - 实现基于 X25519 的契约根密钥派生 (KDF)。
- [x] **前向保密基础 (Rust)**：
    - 实现对称棘轮（Symmetric Ratchet）算法模型。

### 第二阶段：中继服务器重构 (Phase 2)
> 目标：实现从“广播”到“路由”的转变。
- [x] **Hub 逻辑重构 (Go)**：
    - 建立 `NodeID` 到连接的映射表.
    - 实现 `RouteRequest` 处理器.
- [x] **契约准入校验 (Go)**：
    - 中继验证消息发送方与接收方之间是否存在有效的 `ACTIVE` 契约.
- [x] **加密离线队列 (Go)**：
    - 实现基于内存/KV的密文暂存，支持 7 天过期.

### 第三阶段：Agent 交互与治理 (Phase 3)
> 目标：实现 Agent 的自主交互边界。
- [x] **契约状态机集成 (Rust)**：
    - 在 Client 端集成契约生命周期管理.
- [x] **治理权限校验 (Rust/Go)**：
    - 实现 `LORD` 对节点的运维权限检查.
- [x] **感知层级过滤 (Rust)**：
    - 实现不同感知层级（L0-L4）的广播策略基础.

---

## 三、 详细开发步骤 (Step-by-Step)

### Step 4.1: 协议模型定稿与同步
- [x] 修改 `proto/v1/synch.proto` 引入契约。
- [x] 修改 `proto/v1/crypto.proto` 增强载荷元数据。
- [x] 运行 `buf generate` 同步 Go 代码。

### Step 4.2: Rust 安全核心库 (synch-crypto) 开发
- [x] 实现 `contract.rs`：契约签发、验证、存储。
- [x] 实现 `ratchet.rs`：密钥棘轮迭代。
- [x] 修复 Rust 编译错误并验证逻辑。

### Step 4.3: Go 中继服务器核心逻辑修改
- [x] 扩展 `Hub` 结构体，增加 `clients` (map[string]*Client), `offline` 映射。
- [x] 修改 `readPump`：识别 `SyncMessage` 发送方 ID 并执行初次映射。
- [x] 实现 `DeliverOfflineMessages`：客户端上线后自动拉取.
- [x] 实现 `RouteSecuredMessage`：根据 `TargetID` 精准转发.

### Step 4.4: 契约握手流程实现 (Handshake)
- [x] 客户端发起 `BIND_REQ` (协议层已支持).
- [x] 目标端响应 `BIND_ACK` 并签署 (核心库已支持).
- [x] 中继服务器收录契约摘要以备路由检查 (Relay 端已支持).

---

## 四、 验收标准
1. **安全性**：所有消息必须包含 `ContractID`，否则中继应拒绝转发。
2. **前向性**：连续发送 100 条消息后，后续消息密钥不能推导出前序密钥。
3. **可靠性**：目标节点离线时发送消息，节点上线后应能收到 100% 完整的历史密文。
