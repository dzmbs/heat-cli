#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

TARGET="${1:-}"

if [ -n "$TARGET" ]; then
    echo "Building heat (release) for target: $TARGET"
    cargo build --release --target "$TARGET"
    BINARY="target/$TARGET/release/heat"
else
    echo "Building heat (release) for host"
    cargo build --release
    BINARY="target/release/heat"
fi

# On Windows targets the binary has .exe extension
case "${TARGET}" in
    *windows*) BINARY="${BINARY}.exe" ;;
esac

if [ ! -f "$BINARY" ]; then
    echo "ERROR: expected binary not found at $BINARY" >&2
    exit 1
fi

echo "Binary: $BINARY"
