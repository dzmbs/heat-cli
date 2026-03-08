#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

# Read version from workspace Cargo.toml
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
if [ -z "$VERSION" ]; then
    echo "ERROR: could not read version from Cargo.toml" >&2
    exit 1
fi

TARGET="${1:-}"

# Detect OS and ARCH
if [ -n "$TARGET" ]; then
    # Parse from target triple
    case "$TARGET" in
        *linux*)   OS="linux" ;;
        *darwin*)  OS="macos" ;;
        *windows*) OS="windows" ;;
        *)         OS="unknown" ;;
    esac
    case "$TARGET" in
        x86_64*)  ARCH="x86_64" ;;
        aarch64*) ARCH="aarch64" ;;
        arm*)     ARCH="arm" ;;
        i686*)    ARCH="i686" ;;
        *)        ARCH="unknown" ;;
    esac
else
    # Detect from host
    case "$(uname -s)" in
        Linux)  OS="linux" ;;
        Darwin) OS="macos" ;;
        MINGW*|MSYS*|CYGWIN*) OS="windows" ;;
        *)      OS="unknown" ;;
    esac
    case "$(uname -m)" in
        x86_64|amd64)  ARCH="x86_64" ;;
        aarch64|arm64) ARCH="aarch64" ;;
        armv7l)        ARCH="arm" ;;
        i686)          ARCH="i686" ;;
        *)             ARCH="unknown" ;;
    esac
fi

# Build the release binary
if [ -n "$TARGET" ]; then
    bash scripts/release/build.sh "$TARGET"
    BINARY="target/$TARGET/release/heat"
else
    bash scripts/release/build.sh
    BINARY="target/release/heat"
fi

# Windows binary extension
case "$OS" in
    windows) BINARY="${BINARY}.exe" ;;
esac

ARCHIVE_NAME="heat-v${VERSION}-${OS}-${ARCH}"
DIST_DIR="dist"
STAGING_DIR="${DIST_DIR}/${ARCHIVE_NAME}"

# Prepare staging directory
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR"

# Copy binary
cp "$BINARY" "$STAGING_DIR/"

# Copy README if present
if [ -f "README.md" ]; then
    cp "README.md" "$STAGING_DIR/"
fi

# Copy LICENSE if present
if [ -f "LICENSE" ]; then
    cp "LICENSE" "$STAGING_DIR/"
fi

# Package
if [ "$OS" = "windows" ]; then
    ARCHIVE="${DIST_DIR}/${ARCHIVE_NAME}.zip"
    (cd "$DIST_DIR" && zip -r "${ARCHIVE_NAME}.zip" "${ARCHIVE_NAME}/")
else
    ARCHIVE="${DIST_DIR}/${ARCHIVE_NAME}.tar.gz"
    tar -czf "$ARCHIVE" -C "$DIST_DIR" "${ARCHIVE_NAME}"
fi

# Cleanup staging
rm -rf "$STAGING_DIR"

echo "Package: $ARCHIVE"
