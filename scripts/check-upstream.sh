#!/bin/bash
# Check upstream repos for changes since our pinned versions.
# Run this periodically to see what we might want to pull in.

set -e
cd "$(dirname "$0")/.."

echo "=== hypersdk (SDK + CLI) ==="
echo "Pinned at: 0.2.5 / commit bdfb48a"
echo ""
if [ -d reference/hypersdk ]; then
    cd reference/hypersdk
    git fetch origin 2>/dev/null || echo "  (fetch failed — check network)"
    echo "SDK changes:"
    git log bdfb48a..origin/main --oneline -- src/ 2>/dev/null || echo "  (no changes or commit not found)"
    echo ""
    echo "CLI changes:"
    git log bdfb48a..origin/main --oneline -- hypecli/src/ 2>/dev/null || echo "  (no changes or commit not found)"
    cd ../..
else
    echo "  reference/hypersdk not found — clone it first"
fi

echo ""
echo "=== polymarket-cli ==="
echo "Pinned at: 0.1.4 / commit 3ba646b"
echo ""
if [ -d reference/polymarket-cli ]; then
    cd reference/polymarket-cli
    git fetch origin 2>/dev/null || echo "  (fetch failed — check network)"
    echo "Changes:"
    git log 3ba646b..origin/main --oneline -- src/ 2>/dev/null || echo "  (no changes or commit not found)"
    cd ../..
else
    echo "  reference/polymarket-cli not found — clone it first"
fi

echo ""
echo "Done. Review changes above and decide what to sync."
echo "After syncing, update UPSTREAM.md with new pinned versions."
