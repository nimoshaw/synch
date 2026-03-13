#!/bin/sh
set -e

# Default values - for internal container check
HOST=${HEALTHCHECK_HOST:-localhost}
PORT=${HEALTHCHECK_PORT:-8080}
PATH=${HEALTHCHECK_PATH:-/health}

# Check health endpoint
if command -v curl >/dev/null 2>&1; then
    curl -f http://$HOST:$PORT$PATH || exit 1
else
    # Alpine comes with wget
    wget --quiet --tries=1 --spider http://$HOST:$PORT$PATH || exit 1
fi

exit 0
