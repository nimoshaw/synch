# Synch 项目 Agent 深度调试与性能精进流程

这是为项目进入「稳健期」准备的 **「可控调试」** 手册。在完成初步开发后，按照以下 1 到 10 步进行系统性的排错、优化和功能加固。

---

## 调试前置：全局健康检查

在开始任何专项调试前，请先执行全量构建验证当前环境：
```bash
task build:core
task build:server
task build:vcp-agent
```
如果任何一步失败，请让 Agent 优先修复编译错误。

---

## 第一步：契约一致性校验 (Protobuf Deep Dive)

**目标**：确保各端生成的代码与 `.proto` 定义完全同步，解决字段缺失或类型不匹配。

* **推荐模型**：**Claude 3.7 Sonnet** (精准处理跨语言类型映射)
* **💡 发给 Agent 的提示词**：
> "你现在是 Synch 项目的协议审计员。
> 1. 请检查 `proto/v1/` 下的所有文件，对比 `server/pkg/proto/`、`shared/ts-core/` 和 `core/crates/synch-ffi/` 生成的代码。
> 2. 特别关注可选字段（optional）在各端的处理逻辑是否一致。
> 3. 运行 `task proto:gen`，观察是否有编译警告，并修复所有不规范的定义。"

---

## 第二步：核心引擎稳定性调试 (Rust Core Debugging)

**目标**：解决 Rust 层的内存安全隐患、异步死锁以及 FFI 边界的 Panic。

* **推荐模型**：**Qwen2.5-Coder-32B** 或 **Claude 3.7 Sonnet**
* **💡 发给 Agent 的提示词**：
> "你现在是 Rust 专家。我们需要对 `core/` 目录下的同步引擎进行压力测试和调试。
> 1. 请检查 `synch-sync` 中的状态机实现，寻找可能的死锁或竞态条件。
> 2. 为 `core/crates/` 中的每个包编写更多的边缘情况（edge case）单元测试。
> 3. 使用 `cargo test` 运行所有 Rust 测试。如果发现失败，请分析 logs 并修复。
> 4. 检查 UniFFI 暴露的接口是否导致了 UAF (Use-After-Free) 或内存泄漏。"

---

## 第三步：网络中继性能优化 (Go Relay Tuning)

**目标**：调试 WebSocket 掉线重连、高并发下的内存抖动。

* **推荐模型**：**Claude 3.7 Sonnet** (擅长分析并发模型)
* **💡 发给 Agent 的提示词**：
> "你现在是后端性能工程师。请针对 `server/cmd/relay/` 进行调试。
> 1. 模拟 100 个并发客户端连接，观察服务器的内存和 CPU 占用。
> 2. 检查 WebSocket 的 Heartbeat (Ping/Pong) 机制是否健壮，能否在网络波动后快速恢复。
> 3. 为 relay 服务添加 Prometheus 基础指标监控（连接数、消息速率、错误率）。
> 4. 确保日志级别可配置，并在异常断开时输出详细堆栈。"

---

## 第四步：跨端联调：TS 插件 ↔ Go Server

**目标**：打通第一个完整的端到端链路，解决序列化格式不兼容的问题。

* **推荐模型**：**Qwen2.5-Coder-Max** 或 **Claude 3.5 Sonnet**
* **💡 发给 Agent 的提示词**：
> "你现在是全栈开发工程师。我们需要调试 TS 插件与 Go 服务端的连接。
> 1. 启动本地服务器 `task relay:up`。
> 2. 在 `clients/plugins/vcp-agent` 中启用详细的 WebSocket 调试日志。
> 3. 验证 `NodeIdentity` 在 TS 层序列化后发送到 Go 服务端，能否被正确解析（双向校验）。
> 4. 修复 TS `SynchClient` 在网络不稳定时的自动重连指数退避策略。"

---

## 第五步：移动端 UniFFI 内存与生命周期调试

**目标**：解决 Android 端的 JNI 报错、库加载失败以及后台常驻问题。

* **推荐模型**：**Claude 3.7 Sonnet** (移动原生 FFI 处理的首选)
* **💡 发给 Agent 的提示词**：
> "你现在是 Android 专家。我们需要加固移动端的 Rust 集成。
> 1. 检查 `clients/mobile/android/` 的 `.so` 库加载流程，确保支持不同架构 (arm64, x86_64)。
> 2. 调试 Kotlin 侧调用 Rust 异步函数时的生命周期管理，防止在 Activity 销毁后回调导致崩溃。
> 3. 优化 Android 后台服务的资源占用，确保在系统低内存时不被杀掉，或能优雅重启。"

---

## 第六步：同步算法精进 (CRDT/Delta Sync Polishing)

**目标**：处理复杂的冲突合并（Conflict Resolution）和增量更新效率。

* **推荐模型**：**Claude 3.7 Sonnet** 或 **Qwen2.5-Coder-32B**
* **💡 发给 Agent 的提示词**：
> "你现在是分布式系统科学家。我们要优化 Vault 的同步逻辑。
> 1. 请阅读 `core/crates/synch-sync/`，确保增量日志（Delta log）在长链同步时不会产生 O(N^2) 的性能损耗。
> 2. 实现更智能的冲突解决策略（例如 Last-Write-Wins 或自定义语义合并）。
> 3. 编写一个 Mock 测试，模拟两个节点同时修改同一个对象并在随后合并，验证最终一致性。"

---

## 第七步：E2E 自动化测试套件增强

**目标**：建立回归测试体系，确保新功能不破坏旧链路。

* **推荐模型**：**Claude 3.7 Sonnet** (对测试场景的设计非常全面)
* **💡 发给 Agent 的提示词**：
> "你现在是 QA 架构师。请加固 `tests/e2e/` 目录下的测试。
> 1. 基于 Vitest 编写更复杂的端到端场景：A节点修改 -> 中继转发 -> B节点同步 -> B节点反馈。
> 2. 编写网络故障模拟测试（如延迟增加、瞬间丢包）。
> 3. 生成 HTML 测试报告，并统计跨端代码的测试覆盖率。
> 4. 确保 `task test:e2e` 可以在本地一键复现所有线上 CI 的失败项。"

---

## 第八步：安全加固与加密链路审计

**目标**：验证端到端加密（E2EE）的安全性，防止中间人攻击。

* **推荐模型**：**Claude 3.7 Sonnet** (安全意识强)
* **💡 发给 Agent 的提示词**：
> "你现在是网络安全专家。请审计加密模块。
> 1. 检查 `core/crates/synch-crypto`，验证 X25519 密钥交换过程是否符合前向安全性（Forward Secrecy）。
> 2. 确保所有持久化在磁盘上的 Vault 数据都已经过 AES-256-GCM 或类似级别的加密。
> 3. 验证 Go Relay 服务器不具备解密客户端内容的能力（Zero-knowledge 验证）。"

---

## 第九步：部署方案加固与生产环境模拟

**目标**：通过 Docker Compose 模拟生产环境的负载均衡、SSL/TLS 卸载。

* **推荐模型**：**Claude 3.7 Sonnet** (擅长 Docker 与运维编排)
* **💡 发给 Agent 的提示词**：
> "你现在是 SRE 工程师。请优化部署方案。
> 1. 完善 `deploy/docker-compose.prod.yml`，加入 Nginx 作为反向代理，并配置 WSS (WebSocket Secure)。
> 2. 为各组件配置资源限制 (CPU/Memory Limit)，防止内存泄露拖垮宿主机。
> 3. 编写 `deploy/healthcheck.sh`，用于在 K8s 或 Docker Compose 中作为存活检查（Liveness Probe）。"

---

## 第十步：文档同步与开发者体验 (DX)

**目标**：让新加入的 Agent 或人类开发者能快速上手调试项目。

* **推荐模型**：**Claude 3.5/3.7 Sonnet**
* **💡 发给 Agent 的提示词**：
> "你现在是技术文档专家。
> 1. 更新 `docs/API.md`，反映所有在调试阶段变更的接口参数。
> 2. 编写 `docs/DEBUGGING.md`，记录常见的错误码及其解决办法（Troubleshooting）。
> 3. 优化根目录的 `Taskfile.yml`，增加如 `task debug:core` 这种带有 RUST_LOG=debug 的便捷指令。
> 4. 最后执行 `task release:check`，确保版本 v0.1.1 准备就绪（针对修复后的版本）。"

---

## 📋 调试阶段速查表 (调试重点)

| 步骤 | 聚焦领域 | 验证手段 | 常见问题 |
|:---|:---|:---|:---|
| 第1步 | 契约 (Proto) | `task proto:gen` | 类型定义不一致 |
| 第2步 | 核心 (Rust) | `cargo test` | 死锁、FFI Panic |
| 第3步 | 服务端 (Go) | `relay --bench` | 内存抖动、断线重连 |
| 第4步 | TS 联调 | Browser Logs | 序列化错误 |
| 第5步 | 移动端 | logcat / adb | 库加载失败、生命周期崩溃 |
| 第6步 | 同步算法 | Mock 冲突测试 | 最终不一致 |
| 第7步 | E2E 测试 | `task test:e2e` | 偶发性失败 (Flaky) |
| 第8步 | 安全 | 加密代码审计 | 密钥泄露、未加密存储 |
| 第9步 | 部署 | `task deploy:dev` | 权限问题、端口冲突 |
| 第10步 | 文档/DX | 读文档复现 | 文档滞后、上手难 |

---

> **当前进度**：第十步「文档同步与开发者体验 (DX)」已完成 → **本项目已完成全部 10 步深度调试与精进流程。**

