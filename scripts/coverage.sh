#!/usr/bin/env bash
# Generate code coverage reports for both Rust and Swift components.
#
# Usage:
#   ./scripts/coverage.sh              # summary for both
#   ./scripts/coverage.sh rust         # Rust only
#   ./scripts/coverage.sh swift        # Swift only
#   ./scripts/coverage.sh --html       # open HTML reports in browser
#   ./scripts/coverage.sh swift --html # Swift HTML report only

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
HTML=false
TARGET=""

for arg in "$@"; do
    case "$arg" in
        --html) HTML=true ;;
        rust) TARGET="rust" ;;
        swift) TARGET="swift" ;;
        *)
            echo "Usage: $0 [rust|swift] [--html]"
            exit 1
            ;;
    esac
done

# ---------- Rust ----------
run_rust() {
    echo "=== Rust Coverage ==="

    if ! command -v cargo-llvm-cov &>/dev/null; then
        echo "cargo-llvm-cov not found. Install with: cargo install cargo-llvm-cov"
        exit 1
    fi

    if ! command -v cargo-nextest &>/dev/null; then
        echo "cargo-nextest not found. Install with: cargo install cargo-nextest"
        exit 1
    fi

    cd "$ROOT"
    IGNORE_RE='(src/commands/|src/main\.rs|src/sync\.rs)'
    if $HTML; then
        cargo llvm-cov nextest --ignore-filename-regex "$IGNORE_RE" --html
        echo "HTML report: target/llvm-cov/html/index.html"
        open target/llvm-cov/html/index.html
    else
        cargo llvm-cov nextest --ignore-filename-regex "$IGNORE_RE" --fail-under-lines 80
    fi
}

# ---------- Swift ----------
run_swift() {
    echo "=== Swift Coverage ==="

    cd "$ROOT/Hippo"
    swift test --enable-code-coverage

    # The test binary lives inside a .xctest bundle under the platform-specific build dir
    BIN="$(find .build -path "*/HippoPackageTests.xctest/Contents/MacOS/HippoPackageTests" -type f | head -1)"
    PROFDATA="$(find .build -name default.profdata | head -1)"

    if [[ -z "$BIN" || -z "$PROFDATA" ]]; then
        echo "Could not locate coverage artifacts."
        exit 1
    fi

    # Filter to project sources only: exclude .build (dependencies), Tests
    IGNORE='(.build/|Tests/)'

    if $HTML; then
        OUTDIR="$ROOT/target/swift-cov"
        mkdir -p "$OUTDIR"
        xcrun llvm-cov show "$BIN" \
            -instr-profile="$PROFDATA" \
            -ignore-filename-regex="$IGNORE" \
            -format=html \
            -output-dir="$OUTDIR"
        echo "HTML report: $OUTDIR/index.html"
        open "$OUTDIR/index.html"
    else
        xcrun llvm-cov report "$BIN" \
            -instr-profile="$PROFDATA" \
            -ignore-filename-regex="$IGNORE"
    fi
}

# ---------- Dispatch ----------
case "$TARGET" in
    rust) run_rust ;;
    swift) run_swift ;;
    *)
        run_rust
        echo
        run_swift
        ;;
esac
