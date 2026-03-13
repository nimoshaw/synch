# Synch Debugging & Troubleshooting Guide

This guide helps developers and agents diagnose and fix common issues in the Synch ecosystem.

## 🛠️ Diagnostics Tools

### Relay Server Logs
The relay server uses structured JSON logging. You can adjust the verbosity via the `-log` flag or `LOG_LEVEL` environment variable.
```bash
# Set log level to debug
export LOG_LEVEL=debug
task relay:up
```

### Prometheus Metrics
Monitor real-time status via the metrics endpoint:
```bash
curl http://localhost:8080/metrics | grep synch_relay
```
Key metrics:
- `synch_relay_connected_clients`: Current active connections.
- `synch_relay_messages_total`: Total messages processed.
- `synch_relay_errors_total`: Errors categorized by type.

---

## 🛑 Common Issues

### 1. WebSocket Connection Failures
**Symptoms**: Client cannot connect, "Upgrade" header errors, or immediate disconnects.
- **Check Nginx (Prod)**: Ensure `nginx.conf` has `proxy_set_header Upgrade $http_upgrade;`.
- **Check Origin**: The relay server may block origins if not configured (currently set to allow all in dev).
- **Check Health**: `curl http://localhost:8080/health`.

### 2. Protobuf Serialization Errors
**Symptoms**: Server logs `unmarshal error` or clients receive garbled data.
- **Action**: Run `task proto:gen` to ensure all ends are using the same contract.
- **Check**: Compare `.proto` files if versions differ between client and server.

### 3. Identity Verification Failures
**Symptoms**: Messages are rejected with `signature verification failed`.
- **Action**: Verify the public key format. Synch uses Ed25519 (32 bytes).
- **Debugging**: Enable `DEBUG` log level on the relay to see hex-encoded signature details.

### 4. Rust Core Panic (FFI)
**Symptoms**: Mobile or TS-Core crashes when calling into the Rust engine.
- **Action**: Check `RUST_BACKTRACE=1`.
- **Common Cause**: Null pointers in FFI or UniFFI version mismatches.

---

## 🧪 Quick Debug Commands

| Command | Purpose |
|---------|---------|
| `task debug:core` | Build Rust core with full debug info and logs. |
| `task debug:relay` | Start relay with JSON debug logs. |
| `task test:e2e` | Run end-to-end tests to pinpoint failures. |
