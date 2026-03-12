#!/usr/bin/env bash
set -euo pipefail

VERSION="0.4.5"
REPO="dzmbs/heat-cli"
INSTALL_DIR="${HEAT_INSTALL_DIR:-$HOME/.local/bin}"

say() {
  printf '[heat] %s\n' "$*"
}

err() {
  printf '[heat] ERROR: %s\n' "$*" >&2
  exit 1
}

detect_target() {
  local os arch

  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os" in
    Linux)  os="linux" ;;
    Darwin) os="macos" ;;
    *)      err "Unsupported operating system: $os" ;;
  esac

  case "$arch" in
    x86_64)          arch="x86_64" ;;
    arm64|aarch64)   arch="aarch64" ;;
    *)               err "Unsupported architecture: $arch" ;;
  esac

  echo "${os}-${arch}"
}

try_prebuilt() {
  local target="$1"
  local artifact="heat-v${VERSION}-${target}.tar.gz"
  local url="https://github.com/${REPO}/releases/download/v${VERSION}/${artifact}"
  local tmpdir

  say "Downloading Heat v${VERSION} for ${target}..."

  tmpdir="$(mktemp -d)"
  trap "rm -rf '$tmpdir'" EXIT

  if ! curl -fsSL "$url" -o "${tmpdir}/${artifact}"; then
    return 1
  fi

  say "Extracting..."
  tar xzf "${tmpdir}/${artifact}" -C "$tmpdir"

  mkdir -p "$INSTALL_DIR"

  if [ -f "${tmpdir}/heat" ]; then
    mv "${tmpdir}/heat" "${INSTALL_DIR}/heat"
  elif [ -f "${tmpdir}/heat-v${VERSION}-${target}/heat" ]; then
    mv "${tmpdir}/heat-v${VERSION}-${target}/heat" "${INSTALL_DIR}/heat"
  else
    err "Could not find heat binary in archive"
  fi

  chmod +x "${INSTALL_DIR}/heat"
  return 0
}

try_cargo_install() {
  if command -v cargo >/dev/null 2>&1; then
    say "Installing via cargo..."
    cargo install --git "https://github.com/${REPO}" --bin heat
    return 0
  fi
  return 1
}

try_rustup_then_cargo() {
  if command -v rustup >/dev/null 2>&1; then
    say "Rust toolchain manager found but cargo is missing."
    say "Running: rustup install stable"
    rustup install stable
    if command -v cargo >/dev/null 2>&1; then
      try_cargo_install
      return 0
    fi
  fi
  return 1
}

print_no_install_options() {
  err "Could not install Heat.

No prebuilt binary is available for your platform, and neither cargo nor rustup were found.

To install manually:
  1. Install Rust: https://rustup.rs
  2. Run: cargo install --git https://github.com/${REPO} --bin heat"
}

check_path() {
  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) return 0 ;;
  esac

  say ""
  say "Add Heat to your PATH by adding this to your shell profile:"
  say ""
  say "  export PATH=\"${INSTALL_DIR}:\$PATH\""
  say ""
}

main() {
  say "Heat CLI installer"
  say ""

  local target
  target="$(detect_target)"

  if try_prebuilt "$target"; then
    say "Installed Heat to ${INSTALL_DIR}/heat"
  elif try_cargo_install; then
    say "Installed Heat via cargo"
  elif try_rustup_then_cargo; then
    say "Installed Heat via cargo (after rustup)"
  else
    print_no_install_options
  fi

  # Verify
  local heat_bin
  if [ -x "${INSTALL_DIR}/heat" ]; then
    heat_bin="${INSTALL_DIR}/heat"
  elif command -v heat >/dev/null 2>&1; then
    heat_bin="$(command -v heat)"
  else
    check_path
    say "Installation complete. Restart your shell or update PATH, then run: heat --version"
    exit 0
  fi

  say ""
  if "$heat_bin" --version 2>/dev/null; then
    true
  fi

  check_path
  say ""
  say "Installation complete. Run 'heat --help' to get started."
}

main
