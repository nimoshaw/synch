# Synch Deployment Guide

This document provides detailed instructions for deploying the Synch Relay Server in various environments.

## 📋 Prerequisites
- **Redis**: Required for state management and caching.
- **Docker & Docker Compose**: (Recommended) For containerized deployment.
- **Go 1.21+**: Only if building from source on Linux.

---

## 🐋 Docker Installation

### Quick Start
Use the root `docker-compose.yml` for a standard setup:
```bash
docker-compose up -d
```

### Multi-Environment Layout
The `deploy/` directory contains specialized configurations:
- **Development**: `docker-compose.dev.yml` (builds from local source)
- **Staging**: `docker-compose.staging.yml` (uses pre-built images)
- **Production**: `docker-compose.prod.yml` (Nginx reverse proxy, WSS support, resource limits, health checks)

**Architecture in Production:**
- **Nginx**: Exposes ports 80/443, handles SSL/TLS termination and WebSocket Upgrades.
- **Relay Server**: Internal-only exposure (8080), includes `/health` endpoint and monitoring.
- **Redis**: Persistent state storage with append-only mode.

**Usage Example:**
```bash
docker compose -f deploy/docker-compose.staging.yml up -d
```

---

## 🐧 Linux Installation (Standalone Binary)

### 1. Build from Source
If you don't use Docker, you can build the binary manually:
```bash
cd server
go build -o synch-relay ./cmd/relay
```

### 2. Configuration
Create a `.env` file or export environment variables:
```bash
export SYNCH_MODE=production
export SYNCH_REDIS_URL=redis://localhost:6379
export SYNCH_WS_PORT=8081
```

### 3. Production Deployment (systemd)
Create a file at `/etc/systemd/system/synch-relay.service`:
```ini
[Unit]
Description=Synch Relay Server
After=network.target redis.service

[Service]
Type=simple
User=synch
Group=synch
WorkingDirectory=/opt/synch
EnvironmentFile=/opt/synch/.env
ExecStart=/opt/synch/synch-relay
Restart=always

[Install]
WantedBy=multi-user.target
```

---

## 🛠️ Advanced Operations

### Helm (Kubernetes)
```bash
helm install synch-relay deploy/helm/synch -f deploy/helm/synch/values.yaml
```

### Terraform (AWS)
Provision infrastructure using:
```bash
cd deploy/terraform
terraform init
terraform apply
```

## 🌏 Platform Support Matrix

| Platform | Component | Status | Install Method |
| :--- | :--- | :--- | :--- |
| **Android** | Client SDK / App | ✅ Stable | APK, AAR |
| **Linux** | Relay Server | ✅ Stable | Binary, `install-server.sh`, Docker |
| **Docker** | Full Stack | ✅ Stable | Docker Compose |
| **Windows** | Server | 🛠️ In Progress | Binary (.exe) |
| **Windows** | Desktop Client | 📅 Planned | MSI Installer (Wix) |
| **iOS** | Client SDK | 📅 Planned | XCFramework |
| **macOS** | Desktop Client | 📅 Planned | .dmg / Homebrew |

---

## 🚀 Installation Methods

### 1. Linux One-Liner (Recommended)
The fastest way to deploy a production-ready server on Linux:
```bash
curl -sSL https://raw.githubusercontent.com/nimoshaw/synch/main/deploy/scripts/install-server.sh | sudo bash
```
This script performs the following:
- Detects architecture (amd64/arm64).
- Creates a dedicated `synch` system user.
- Downloads the latest statically-linked binary.
- Sets up a `systemd` service with automatic restart.
- Creates `/etc/synch/.env` template for configuration.

### 2. Docker Compose
Ideal for quick evaluations or multi-component setups:
```bash
docker compose up -d
```
Config files are located in `deploy/docker/`.

### 3. Android APK
Direct downloads are available for testing on real devices. The APK includes both `arm64-v8a` and `armeabi-v7a` support for maximum compatibility.

---

## 🔐 Environment Variables Reference
| Variable | Description | Default |
|----------|-------------|---------|
| `SYNCH_MODE` | Operating mode (`development`, `production`) | `production` |
| `SYNCH_WS_PORT` | Port for WebSocket connections | `8081` |
| `SYNCH_REDIS_URL` | Redis connection string | `redis://localhost:6379` |
| `SYNCH_LOG_LEVEL` | Logging verbosity | `info` |

## 🛠️ Maintenance & Troubleshooting
- **Logs**: `journalctl -u synch-relay -f`
- **Health Check**: `curl http://localhost:8080/health`
- **Config**: `/etc/synch/.env`
