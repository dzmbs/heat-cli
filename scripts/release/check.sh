#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/../.."

echo "=== Heat CLI release check ==="
echo ""

# 1. format check (must come first)
echo "[1/6] cargo fmt --all -- --check"
cargo fmt --all -- --check || { echo "FAIL: code is not formatted. Run 'cargo fmt --all' to fix."; exit 1; }
echo "      OK"

# 2. clippy (same flags as CI)
echo "[2/6] cargo clippy --workspace -- -D warnings"
cargo clippy --workspace -- -D warnings || { echo "FAIL: clippy found warnings."; exit 1; }
echo "      OK"

# 3. check
echo "[3/6] cargo check --workspace"
cargo check -q --workspace || { echo "FAIL: cargo check failed."; exit 1; }
echo "      OK"

# 4. test
echo "[4/6] cargo test --workspace"
cargo test -q --workspace || { echo "FAIL: tests failed."; exit 1; }
echo "      OK"

# 5. release build
echo "[5/6] cargo build --release -p heat"
cargo build --release -p heat || { echo "FAIL: release build failed."; exit 1; }
echo "      OK"

# 6. smoke test
echo "[6/6] smoke test: ./target/release/heat --help"
./target/release/heat --help >/dev/null || { echo "FAIL: heat --help did not succeed."; exit 1; }
echo "      OK"

echo ""
echo "=== All checks passed ==="
