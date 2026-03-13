# Synch 项目 Agent 傻瓜式开发执行流程

这是为你准备的 **「复制粘贴」级别** 指令手册。按照以下 1 到 10 步顺序依次执行，由于涉及到不同的技术栈，每一步都推荐了当前阶段最适合的 AI 模型。

---

## 准备步骤：查看所有可用指令

在根目录下输入以下命令，可以先看看我们刚才补全的所有可用任务：
```bash
task --list
```
*(你也可以让 Agent 帮你执行这个命令以确认当前环境)*

---

## 第一步：核心基建与跨语言契约 (Protobuf)

**目标**：把架构文档（ADR）转化为严谨的 Proto 文件定义。

* **推荐模型**：**Claude 3.7 Sonnet** 或 **Claude 3.5 Sonnet** (对架构领悟和 Protobuf 最准)
* **💡 发给 Agent 的提示词**：
> "你现在是 Synch 项目的 Phase 1 架构师。请仔细阅读 `docs/ADR-001-architecture.md` 以及 `.synch/instructions/PHASE_1.md`。
> 1. 请在 `proto/v1/` 目录下创建 `synch.proto` (包含身份 NodeIdentity, 多服务端会话 ServerEndpoint 的定义)。
> 2. 创建 `sync.proto` (包含 Vault 同步状态机，握手和 Delta 传输结构的定义)。
> 3. 创建 `crypto.proto` (加密原语等)。
> 4. 完成后，请帮我执行 `task proto:gen`，确保基于 `buf.yaml` 的各端代码顺利生成。如果遇到依赖问题，请先执行 `task init`。"

---

## 第二步：核心引擎层实现 (Rust)

**目标**：用 Rust 实现高难度的底层网络、加密和同步 CRDT 算法，并暴露安全的 FFI。

* **推荐模型**：**Qwen2.5-Coder-32B** 或 **Claude 3.7 Sonnet** (Rust 需要极强的类型系统和所有权排错能力)
* **💡 发给 Agent 的提示词**：
> "你现在是 Synch 项目的底层核心工程师。请阅读 `proto/v1/` 下的契约以及架构文档。
> 1. 请在 `core/` 目录中建立 Rust Cargo workspace，并配置 `uniffi`。
> 2. 实现基于 `synch.proto` 和 `crypto.proto` 的基础加密模块（X25519/Ed25519）。
> 3. 实现一个基础的内存中 Vault 增量日志 (Delta log) 同步逻辑。
> 4. 完成后，请执行 `task build:core`，直到成功编译输出 release 库文件为止。"

---

## 第三步：服务端 Hub 实现 (Go)

**目标**：处理连接转发、权限控制和网络 Relay。

* **推荐模型**：**Qwen2.5-Coder** 或 **Claude 3.7 Sonnet** (对 Go 并发和 WebSocket 熟悉即可)
* **💡 发给 Agent 的提示词**：
> "你现在是 Synch 项目的后端工程师。
> 1. 我们已经在 `server/pkg/proto/` 目录下通过 buf 生成了 Go 版本的 protobuf 代码。
> 2. 请在 `server/cmd/relay/main.go` 中，搭建一个基于 WebSocket 的基础服务端应用。
> 3. 实现根据 `SynchClient` 的客户端连接，维持在线状态机，并能够接收和打印 `SyncMessage` 控制帧。
> 4. 完成后，请你提供一个 `docker-compose.yml` 包含这个 Go 服务，并帮我执行 `task relay:up` 启动它以供独立测试。"

---

## 第四步：客户端首发型号之一 (TS 插件)

**目标**：在 VCP-Agent 和 OpenClaw 环境跑通第一个插件节点。

* **推荐模型**：**Qwen2.5-Coder-Max** 或 **Claude 3.5 Sonnet** (对于 TS/前端生态极其熟悉且速度快)
* **💡 发给 Agent 的提示词**：
> "你是 Synch 项目的前端与插件开发工程师。服务端和底层契约已经就绪。
> 1. 请看 `shared/ts-core`，这里是我们使用 ts-proto 生成的跨端类型。
> 2. 请初始化 `clients/plugins/vcp-agent`，这是一个标准 Vite+TS 的开发目录。
> 3. 实现 `SynchClient.ts` 连接本地 websocket服务端 (ws://localhost:8081)。
> 4. 创建一个简单的 React/Vue 面板 (Panel)，展示当前这台设备的 NodeID 和网络连接状态。
> 5. 编码完成后，请为我执行 `task build:vcp-agent` 跑通构建流。"

---

## 第五步：移动端集成试验 (Kotlin / Swift)

**目标**：Android/iOS 的底层挂载 (调用步骤二的 Rust UniFFI 产物)。

* **推荐模型**：由于包含移动原生和 FFI，强烈建议 **Claude 3.7 Sonnet**
* **💡 发给 Agent 的提示词**：
> "你是 Synch 项目的移动端专家。我们需要将之前的 Rust FFI 集成到移动端。
> 1. 请在 `clients/mobile/android/` 初始化一个简单的能发通知的后台服务框架（Kotlin）。
> 2. 加载由 `core` 目录通过 UniFFI 编译出的安卓版动态库，并在 App 启动时打印生成的本机的 Ed25519 身份公钥。
> 3. 确保你的 gradle 脚本能顺利把 Rust binding 引用进来就行，业务逻辑暂不细化。执行构建测试通过即可。"

---

## 第六步：GitHub 仓库初始化与云端托管

**目标**：将本地项目推送到 GitHub，建立规范的分支模型和协作流程。

**推荐模型**：Claude 3.5 Sonnet 或 Qwen2.5-Coder (Git 操作与 GitHub Actions 熟悉即可)

**💡 发给 Agent 的提示词**：

> "你是 Synch 项目的 DevOps 工程师。请帮我完成 GitHub 云端托管初始化：
> 
> 1. 在根目录创建 `.gitignore`，排除 `target/`、`node_modules/`、`*.so`、`*.dylib`、`*.dll`、`build/`、`.env` 等构建产物
> 2. 初始化 git 仓库，创建初始 commit：`init: Synch project bootstrap`
> 3. 创建 GitHub 仓库（假设用户已提供 token 或通过 gh CLI），推送到 origin main
> 4. 建立分支保护规则：main 分支需 PR 审查，develop 分支作为集成分支
> 5. 创建标签规范：`v0.1.0-alpha` 格式，为当前状态打初始标签
> 
> 完成后，请执行 `git status` 确认工作区干净，并输出远程仓库 URL。"

---

## 第七步：多平台 CI/CD 流水线 (GitHub Actions)

**目标**：建立自动化构建矩阵，覆盖 Rust Core、Go Server、TS Client、Mobile 全平台。

**推荐模型**：Claude 3.7 Sonnet (复杂 YAML 编排和矩阵构建策略)

**💡 发给 Agent 的提示词**：

> "你是 Synch 项目的 CI/CD 架构师。请在 `.github/workflows/` 目录创建：
> 
> **ci.yml** - 主 CI 流水线：
> - 触发条件：PR 到 main/develop，push 到任何分支
> - 构建矩阵：ubuntu-latest (全量), macos-latest (Rust FFI 测试), windows-latest (兼容性)
> - 步骤：checkout → 缓存依赖 → `task init` → `task proto:gen` → `task build:core` → `task build:server` → `task build:vcp-agent` → 运行单元测试
> 
> **release.yml** - 发布流水线：
> - 手动触发或标签推送触发
> - 构建产物：Rust 库 (.so/.dylib/.dll), Go server 二进制, TS 插件 bundle
> - 创建 GitHub Release，上传各平台 artifact
> - 生成 changelog（基于 conventional commits）
> 
> **docker.yml** - 容器镜像：
> - 构建多架构 server 镜像 (amd64, arm64)
> - 推送到 GitHub Container Registry
> 
> 完成后，请在本地执行 `act -j build` 或提交测试 PR 验证流水线。"

---

## 第八步：集成测试与端到端验证

**目标**：在云端环境验证多组件协作，确保 Rust Core ↔ Go Server ↔ TS Client 链路通畅。

**推荐模型**：Claude 3.7 Sonnet 或 Qwen2.5-Coder-32B (测试编排和故障诊断)

**💡 发给 Agent 的提示词**：

> "你是 Synch 项目的 QA 工程师。请建立端到端测试体系：
> 
> 1. 在 `tests/e2e/` 创建测试套件：
>    - `setup.ts` - 启动测试环境（docker-compose 启动 server，加载 Rust 库）
>    - `sync.test.ts` - 测试两端 TS Client 通过 Go Server 完成 Vault 同步
>    - `crypto.test.ts` - 验证跨语言密钥交换（Rust 生成，TS 验证）
> 
> 2. 创建 `docker-compose.test.yml` - 测试专用编排：
>    - 包含 server 服务、测试 runner、网络隔离
>    - 健康检查确保服务就绪后才执行测试
> 
> 3. 在 CI 中新增 `test:e2e` job，使用 GitHub Actions 的 service containers
> 4. 生成覆盖率报告，上传 codecov
> 
> 完成后，执行 `task test:e2e` 本地验证，确保测试能在 CI 中稳定通过。"

---

## 第九步：多环境部署与配置管理

**目标**：建立 dev/staging/prod 多环境，支持云端演示和内部测试。

**推荐模型**：Claude 3.7 Sonnet (基础设施即代码和多环境策略)

**💡 发给 Agent 的提示词**：

> "你是 Synch 项目的平台工程师。请完成多环境部署配置：
> 
> 1. 在 `deploy/` 目录创建：
>    - `helm/` - Kubernetes Helm chart（可选，如用户有 K8s）
>    - `terraform/` 或 `pulumi/` - 云资源定义（AWS/GCP/Azure 任选）
>    - `docker-compose.{dev,staging,prod}.yml` - 分层配置
> 
> 2. 配置管理：
>    - 创建 `config/` 目录，按环境分离配置
>    - 使用环境变量 + 配置中心（如 consul/etcd 或简单 env 文件）
>    - 敏感信息走 GitHub Secrets / sealed secrets
> 
> 3. 部署脚本：
>    - `task deploy:dev` - 本地 docker-compose
>    - `task deploy:staging` - 推送到 staging 服务器
>    - `task deploy:prod` - 生产发布（需人工确认）
> 
> 4. 创建 `docs/DEPLOYMENT.md` 部署手册
> 
> 完成后，验证 dev 环境能完整启动所有组件。"

---

## 第十步：发布合并与项目归档

**目标**：完成 MVP 发布，建立长期维护机制，项目进入稳定迭代。

**推荐模型**：Claude 3.7 Sonnet (文档工程和维护策略)

**💡 发给 Agent 的提示词**：

> "你是 Synch 项目的发布经理。请完成 MVP 发布准备：
> 
> **版本发布**：
> - 更新所有组件版本号到 `v0.1.0`
> - 创建 GitHub Release，附带完整 changelog
> - 构建并上传各平台二进制到 release assets
> 
> **文档完善**：
> - `README.md` - 项目介绍、快速开始、架构图
> - `docs/API.md` - 自动生成 protobuf API 文档
> - `docs/CONTRIBUTING.md` - 贡献指南、代码规范
> - `docs/SECURITY.md` - 安全披露政策
> 
> **项目治理**：
> - 创建 issue 模板（bug report, feature request）
> - 创建 PR 模板，关联 checklist
> - 配置 dependabot 自动更新依赖
> - 添加 LICENSE 文件（MIT/Apache-2.0 任选）
> 
> **归档准备**：
> - 创建 `docs/ROADMAP.md` - 后续版本规划
> - 标记 good first issue，吸引外部贡献
> - 项目状态徽章（CI 状态、版本、许可证）
> 
> 完成后，执行 `task release:check` 验证所有检查项通过，宣布 v0.1.0 正式发布。"

---

## 📋 完整步骤速查表

| 步骤 | 阶段 | 关键产出 | 验证命令 |
|:---|:---|:---|:---|
| 第1步 | 契约 | Protobuf 定义、buf 生成代码 | `task proto:gen` |
| 第2步 | 核心 | Rust FFI 库、加密模块 | `task build:core` |
| 第3步 | 服务 | Go WebSocket 服务端 | `task relay:up` |
| 第4步 | 客户端 | TS 插件、VCP-Agent 节点 | `task build:vcp-agent` |
| 第5步 | 移动端 | Kotlin/Android 基础框架 | gradle build 通过 |
| 第6步 | 托管 | GitHub 仓库、分支保护 | `git remote -v` |
| 第7步 | 自动化 | CI/CD 流水线、构建矩阵 | 查看 Actions 运行状态 |
| 第8步 | 验证 | E2E 测试、覆盖率报告 | `task test:e2e` |
| 第9步 | 部署 | 多环境配置、基础设施 | `task deploy:dev` |
| 第10步 | 发布 | v0.1.0 Release、完整文档 | `task release:check` |

---

> **当前进度**：第5步已完成 → **下一步执行第6步：GitHub 仓库初始化**