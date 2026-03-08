#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

INSTALL_DIR="${INSTALL_DIR:-/usr/local/bin}"
BINARY="target/release/heat"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: release binary not found at $BINARY" >&2
    echo "Run 'cargo build --release' or 'scripts/release/build.sh' first." >&2
    exit 1
fi

# Ensure install directory exists
if [ ! -d "$INSTALL_DIR" ]; then
    echo "ERROR: install directory does not exist: $INSTALL_DIR" >&2
    exit 1
fi

cp "$BINARY" "$INSTALL_DIR/heat"
chmod +x "$INSTALL_DIR/heat"

VERSION=$("$INSTALL_DIR/heat" --version 2>/dev/null || echo "unknown")
echo "Installed heat to $INSTALL_DIR/heat"
echo "Version: $VERSION"
