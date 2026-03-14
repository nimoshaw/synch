# Synch Deployment Guide

本文档提供 Synch Relay Server 在各种环境下的详细部署说明。

## 📋 前置要求

- **Docker & Docker Compose**：（推荐）容器化部署
- **Go 1.25+**：仅在从源码构建时需要

> **注意**：Synch 使用内嵌的 BadgerDB 作为持久化存储，**不需要外部数据库**（如 Redis/MySQL）。

---

## 🐋 Docker 部署

### 快速启动（开发/测试）

在项目根目录运行：
```bash
docker compose up -d
```

验证运行状态：
```bash
curl http://localhost:8080/health
```

### 多环境部署

`deploy/` 目录包含针对不同环境的 Docker Compose 配置：

| 文件 | 环境 | 说明 |
|------|------|------|
| `docker-compose.dev.yml` | 开发 | 从本地源码构建，Debug 日志 |
| `docker-compose.staging.yml` | 测试 | 使用预构建镜像 |
| `docker-compose.prod.yml` | 生产 | Nginx 反代 + SSL + 资源限制 + 健康检查 |

使用示例：
```bash
# 开发环境
docker compose -f deploy/docker-compose.dev.yml up -d --build

# 测试环境
docker compose -f deploy/docker-compose.staging.yml up -d

# 生产环境 (请先配置 SSL 证书和 nginx.conf)
docker compose -f deploy/docker-compose.prod.yml up -d
```

**生产环境架构：**
- **Nginx**：暴露 80/443 端口，处理 SSL/TLS 终止和 WebSocket Upgrade
- **Relay Server**：仅暴露 8080 给 Nginx（不直接对外），含 `/health` 健康检查
- **BadgerDB**：数据存储在 Docker Volume 中持久化

---

## 🐧 Linux 独立部署

### 方式一：一键安装脚本（推荐）

```bash
curl -sSL https://raw.githubusercontent.com/nimoshaw/synch/main/deploy/scripts/install-server.sh | sudo bash
```

脚本执行过程：
1. 检测 CPU 架构 (amd64/arm64)
2. 创建 `synch` 系统用户和组
3. 下载最新的编译好的二进制文件
4. 安装到 `/usr/local/bin/synch-relay`
5. 创建 systemd 服务（开机自启、崩溃自动重启）
6. 生成配置文件模板 `/etc/synch/.env`

安装后操作：
```bash
# 1. 编辑配置文件
sudo nano /etc/synch/.env

# 2. 启动服务
sudo systemctl start synch-relay

# 3. 设为开机自启
sudo systemctl enable synch-relay

# 4. 查看运行状态
sudo systemctl status synch-relay
```

### 方式二：手动编译安装

```bash
# 编译
cd server
go build -ldflags "-s -w" -o synch-relay ./cmd/relay

# 运行
./synch-relay -addr :8080 -mode production -log info -db /var/lib/synch/data
```

---

## ⚙️ 环境变量参考

通过环境变量或 CLI 参数配置（CLI 参数优先）：

| 环境变量 | CLI 参数 | 说明 | 默认值 |
|----------|---------|------|--------|
| `SYNCH_WS_PORT` | `-addr` | 监听地址和端口 | `:8080` |
| `SYNCH_DB_PATH` | `-db` | BadgerDB 数据目录 | `./relay_db` |
| `SYNCH_MODE` | `-mode` | 运行模式 (`development` / `production`) | `production` |
| `SYNCH_LOG_LEVEL` | `-log` | 日志级别 (`debug`/`info`/`warn`/`error`) | `info` |
| `SYNCH_ADMIN_TOKEN` | — | Admin API 鉴权 Token（空=不鉴权） | 空 |
| `SYNCH_ALLOWED_ORIGINS` | — | WebSocket 允许的 Origin（逗号分隔，空=全部允许） | 空 |

**配置文件示例** (`/etc/synch/.env`)：
```env
SYNCH_MODE=production
SYNCH_LOG_LEVEL=info
SYNCH_WS_PORT=:8080
SYNCH_DB_PATH=/var/lib/synch/data
SYNCH_ADMIN_TOKEN=your-secret-token-here
```

---

## 🛠️ 高级部署

### Helm (Kubernetes)
```bash
helm install synch-relay deploy/helm/synch -f deploy/helm/synch/values.yaml
```

### Terraform (AWS)
```bash
cd deploy/terraform
terraform init
terraform apply
```

---

## 🌏 平台支持矩阵

| 平台 | 组件 | 状态 | 安装方式 |
|------|------|------|----------|
| **Linux** | Relay Server | ✅ 稳定 | Binary, 安装脚本, Docker |
| **Docker** | 全栈 | ✅ 稳定 | Docker Compose |
| **Android** | 移动端 App | ✅ 稳定 | APK |
| **Windows** | Relay Server | ✅ 可用 | Binary (.exe) |
| **iOS** | 移动端 App | 📅 计划中 | — |
| **macOS** | Desktop Client | 📅 计划中 | — |

---

## 🔧 运维与排查

| 操作 | 命令 |
|------|------|
| 查看日志 | `journalctl -u synch-relay -f` |
| 健康检查 | `curl http://localhost:8080/health` |
| 查看指标 | `curl http://localhost:8080/metrics` |
| 编辑配置 | `sudo nano /etc/synch/.env` |
| 查看在线节点 | `curl -H "Authorization: Bearer TOKEN" http://localhost:8080/api/admin/nodes` |
| 重启服务 | `sudo systemctl restart synch-relay` |
