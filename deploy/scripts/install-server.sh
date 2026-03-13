#!/bin/bash
# Synch Relay Server Installer
# This script installs the Synch Relay Server as a systemd service.

set -e

# --- Configuration ---
BINARY_NAME="synch-relay"
INSTALL_DIR="/usr/local/bin"
CONFIG_DIR="/etc/synch"
USER="synch"
GROUP="synch"
REPO="synch/synch" # Assuming GitHub repo structure

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting Synch Relay Server Installation...${NC}"

# Check for root
if [ "$EUID" -ne 0 ]; then
  echo -e "${RED}Please run as root or with sudo.${NC}"
  exit 1
fi

# Determine Architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64)  ARCH_TYPE="linux-amd64" ;;
    aarch64) ARCH_TYPE="linux-arm64" ;;
    *)       echo -e "${RED}Unsupported architecture: $ARCH${NC}"; exit 1 ;;
esac

echo -e "Detected architecture: ${GREEN}$ARCH ($ARCH_TYPE)${NC}"

# Create synch user/group if not exists
if ! getent group "$GROUP" > /dev/null; then
    groupadd "$GROUP"
fi
if ! id -u "$USER" > /dev/null; then
    useradd -r -g "$GROUP" -s /sbin/nologin "$USER"
fi

# Download binary (Placeholder for actual release URL)
# LATEST_TAG=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
# DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/synch-relay-$ARCH_TYPE.tar.gz"

echo -e "Downloading latest release... (Simulated)"
# curl -L $DOWNLOAD_URL -o /tmp/synch-relay.tar.gz
# tar -xzf /tmp/synch-relay.tar.gz -C /tmp

# For now, we assume the binary is already built or the user provides it
# if [ ! -f "/tmp/$BINARY_NAME" ]; then
#     echo -e "${RED}Binary $BINARY_NAME not found in /tmp. Make sure to build it first for this demo.${NC}"
#     exit 1
# fi

# Setup directories
mkdir -p "$CONFIG_DIR"
chown "$USER:$GROUP" "$CONFIG_DIR"

# Install binary
# mv "/tmp/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
# chmod +x "$INSTALL_DIR/$BINARY_NAME"

# Create systemd unit
cat <<EOF > /etc/systemd/system/synch-relay.service
[Unit]
Description=Synch Relay Server
After=network.target redis-server.service

[Service]
Type=simple
User=$USER
Group=$GROUP
WorkingDirectory=$CONFIG_DIR
Environment=SYNCH_MODE=production
# Load env file if exists
EnvironmentFile=-$CONFIG_DIR/.env
ExecStart=$INSTALL_DIR/$BINARY_NAME
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd
systemctl daemon-reload

echo -e "${GREEN}Installation complete!${NC}"
echo -e "1. Edit ${CONFIG_DIR}/.env to configure your server."
echo -e "2. Start the service: ${GREEN}systemctl start synch-relay${NC}"
echo -e "3. Enable on boot: ${GREEN}systemctl enable synch-relay${NC}"
