# 03. Synch 项目全平台分发与安装打包规划 (Multi-Platform Distribution Plan)

为了实现「全平台覆盖、一键安装、架构可延续」的目标，本项目建立了一套可扩展的分发体系。该体系不仅解决当前的 Android、Linux 和 Docker 需求，还为后续的 Windows、iOS 及其他各种安装包提供了标准化的路径。

---

## 🏗️ 分发架构设计 (Distribution Architecture)

我们将采用**「核心统一、前端适配、自动化打包」**的思路，通过以下结构保障项目的美观与合理：

- `core/`: 统一的 Rust 核心库，为所有平台提供一致的同步引擎。
- `packaging/`: **[新增]** 集中存放各平台的打包配置文件（如 `scripts/`, `debian/`, `msi/`, `plist/` 等）。
- `dist/`: CI/CD 生成的最终安装包暂存区。

---

## 01. 🚀 第一阶段：移动端全覆盖 (Mobile Distribution)

### 1. Android (APK & AAR)
- **APK 分发**：生成 Universal APK，集成到 GitHub Releases。
- **AAR SDK**：为其他开发者提供可直接调用的 AAR 包。
- **未来扩展**：支持 Play Store 上架（App Bundle 格式）。

### 2. iOS (将来任务 - 已预留接口)
- **UniFFI 桥接**：利用现有的 Rust FFI 逻辑生成 Swift 绑定。
- **XCFramework**：在 `packaging/ios` 中配置打包脚本，生成支持模拟器和真机的 XCFramework。

---

## 02. 💻 第二阶段：桌面端与服务端 (Desktop & Server)

### 1. Linux (Server)
- **分发方式**：提供静态编译二进制文件 + `install-server.sh` 一键安装脚本。
- **包管理**：未来计划提供 `.deb` 和 `.rpm` 标准包（存储在 `packaging/linux/`）。

### 2. Windows (Client & Server - 规划中)
- **Server**：提供 `.exe` 二进制文件，支持通过 `nssm` 或原生 API 注册为 Windows 服务。
- **Client**：基于 Rust Core，未来可集成 WPF/Tauri 开发 GUI 客户端，使用 `packaging/windows` 下的 `.wxs` 配置生成 MSI 安装程序。

---

## 03. 🐋 第三阶段：云原生部署 (Docker & Cloud)

- **容器分发**：镜像推送到 `ghcr.io/synch/relay`，用户无需本地构建。
- **极简部署**：用户只需运行 `docker compose up -d` 即可。
- **监控集成**：内置 Prometheus 指标抓取，方便在容器云环境观测。

---

## 📂 推荐文件结构优化 (Proposed Directory Structure)

```text
synch/
├── clients/
│   ├── mobile/ (android, [ios])
│   ├── desktop/ ([windows], [macos], [linux])
│   └── plugins/ (vcp-agent, ...)
├── deploy/
│   ├── docker/ (compose configs, healthchecks)
│   └── scripts/ (installers, uninstallers)
├── packaging/  <-- [核心分发配置文件夹]
│   ├── android/
│   ├── linux/
│   ├── windows/
│   └── ios/
└── Taskfile.yml (增加分发相关的 task，如 dist:android, dist:linux)
```

---

## 🏁 实施现状 (Current Implementation Status)

目前，分发体系的第一阶段（基础建设）已**完全实施**：

1.  **分发架构**：`packaging/` 及其子目录已创建，实现了配置与代码的完全解耦。
2.  **Android 分发**：APK 自动化构建已就绪，支持 `arm64-v8a` 和 `armeabi-v7a`。
3.  **Linux 服务端**：`install-server.sh` 脚本已发布，支持一键部署和 systemd 集成。
4.  **自动化流水线**：GitHub Actions `release.yml` 已更新，每次 Tag 发布都会自动产生上述资产。
5.  **开发者工具**：`Taskfile.yml` 已集成 `dist:android` 和 `dist:linux` 任务。

---

## ✅ 验证计划与结果 (Verification & Results)

| 平台 | 验证方式 | 状态 | 预期结果 |
| :--- | :--- | :--- | :--- |
| **Android** | `task dist:android` | ✅ 已就绪 | 生成通用的 `.apk` 安装包 |
| **Linux** | `task dist:linux` | ✅ 已就绪 | 生成静态二进制 + 安装脚本 |
| **Docker** | `docker compose up` | ✅ 已就绪 | 容器集群一键启动 |
| **Windows** | `task build:windows`| 📅 规划中 | 生成 `.exe` 二进制文件 |
| **iOS** | `task build:ios` | 📅 规划中 | 生成 `XCFramework` SDK |
