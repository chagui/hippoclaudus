#!/bin/bash
set -e

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"

echo "==> Setting up development environment..."

# --- Platform check ---

if [[ "$(uname)" != "Darwin" ]]; then
    echo "Error: hippoclaudus only supports macOS." >&2
    exit 1
fi

# --- Required tools ---

missing=()

command -v cargo &>/dev/null || missing+=("cargo (https://rustup.rs)")
command -v swift &>/dev/null || missing+=("swift (xcode-select --install)")

if [[ ${#missing[@]} -gt 0 ]]; then
    echo "Error: missing required tools:" >&2
    for tool in "${missing[@]}"; do
        echo "  - $tool" >&2
    done
    exit 1
fi

# --- Cargo dev tools ---

echo "==> Installing Cargo dev tools..."

cargo_tools=(
    "cargo-nextest"
    "cargo-llvm-cov"
)

for tool in "${cargo_tools[@]}"; do
    if ! cargo install --list | grep -q "^${tool} "; then
        echo "    Installing ${tool}..."
        cargo install "$tool"
    else
        echo "    ${tool} already installed"
    fi
done

# --- Homebrew tools ---

brew_tools=(
    "lefthook"
    "shfmt"
    "shellcheck"
    "swiftformat"
    "yamllint"
)

if command -v brew &>/dev/null; then
    echo "==> Installing Homebrew dev tools..."
    for tool in "${brew_tools[@]}"; do
        if ! command -v "$tool" &>/dev/null; then
            echo "    Installing ${tool}..."
            brew install "$tool"
        else
            echo "    ${tool} already installed"
        fi
    done
else
    echo "Warning: Homebrew not found. Install missing tools manually:" >&2
    for tool in "${brew_tools[@]}"; do
        command -v "$tool" &>/dev/null || echo "  - $tool" >&2
    done
fi

if command -v lefthook &>/dev/null; then
    echo "==> Installing git hooks..."
    cd "$REPO_DIR"
    lefthook install
fi

echo ""
echo "==> Dev environment ready!"
echo "    Run tests:    cargo nextest run"
echo "    Coverage:     ./scripts/coverage.sh"
echo "    CI checks:    ./scripts/ci.sh"
