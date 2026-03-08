#!/usr/bin/env bash
# Run the same checks as GitHub Actions CI locally.
# Usage: ./scripts/ci.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

pass() { printf '\033[32m✓ %s\033[0m\n' "$1"; }
fail() { printf '\033[31m✗ %s\033[0m\n' "$1"; exit 1; }
step() { printf '\n\033[1m=== %s ===\033[0m\n' "$1"; }

# --- Rust ---
step "cargo fmt --check"
cargo fmt --check || fail "cargo fmt"
pass "cargo fmt"

step "cargo build"
cargo build --release || fail "cargo build"
pass "cargo build"

step "cargo clippy"
cargo clippy -- -D warnings || fail "cargo clippy"
pass "cargo clippy"

step "cargo nextest run"
cargo nextest run || fail "cargo nextest run"
pass "cargo nextest run"

# --- Swift ---
step "swift build"
(cd ClaudePulse && swift build -c release) || fail "swift build"
pass "swift build"

step "swift test"
(cd ClaudePulse && swift test) || fail "swift test"
pass "swift test"

printf '\n\033[32m=== All CI checks passed ===\033[0m\n'
