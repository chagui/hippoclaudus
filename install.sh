#!/bin/bash
set -e

CLI_ONLY=false
for arg in "$@"; do
    case "$arg" in
        --cli-only) CLI_ONLY=true ;;
    esac
done

REPO_DIR="$(cd "$(dirname "$0")" && pwd)"
CLI_NAME="claude-pulse"
PLIST_NAME="com.chagui.claude-pulse.plist"
APP_SUPPORT_DIR="$HOME/Library/Application Support/com.chagui.claude-pulse"
LOG_DIR="$HOME/Library/Logs/com.chagui.claude-pulse"

# --- Prerequisite checks ---

if [[ "$(uname)" != "Darwin" ]]; then
    echo "Error: claude-pulse only supports macOS." >&2
    exit 1
fi

if ! command -v cargo &>/dev/null; then
    echo "Error: cargo not found. Install Rust via https://rustup.rs" >&2
    exit 1
fi

if [[ "$CLI_ONLY" == false ]] && ! command -v swift &>/dev/null; then
    echo "Error: swift not found. Install Xcode Command Line Tools: xcode-select --install" >&2
    exit 1
fi

# --- Build & Install ---

echo "==> Building Rust CLI..."
cd "$REPO_DIR"
cargo build --release

echo "==> Installing CLI binary..."
mkdir -p ~/.local/bin
ln -sf "$REPO_DIR/target/release/$CLI_NAME" ~/.local/bin/$CLI_NAME
echo "    Linked to ~/.local/bin/$CLI_NAME"

if [[ "$CLI_ONLY" == false ]]; then
    echo "==> Building Swift menubar app..."
    cd "$REPO_DIR/ClaudePulse"
    swift build -c release
    echo "    Built successfully"
fi

echo "==> Creating directories..."
mkdir -p "$APP_SUPPORT_DIR"
mkdir -p "$LOG_DIR"

echo "==> Installing LaunchAgent..."
# Template the plist: replace __HOME__ with actual $HOME
sed "s|__HOME__|$HOME|g" "$REPO_DIR/$PLIST_NAME" > ~/Library/LaunchAgents/$PLIST_NAME

# Unload first if already loaded (ignore errors)
launchctl bootout "gui/$(id -u)/com.chagui.claude-pulse" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" ~/Library/LaunchAgents/$PLIST_NAME
echo "    LaunchAgent loaded (runs daily at 1:00 AM)"

echo ""
echo "==> Installation complete!"
echo "    Config: $APP_SUPPORT_DIR/config.json"
echo "    State DB: $APP_SUPPORT_DIR/state.db"
echo "    Logs: $LOG_DIR/"
echo "    Test with: claude-pulse status"
echo "    Manual sync: claude-pulse sync --dry-run --days 7"
echo "    LaunchAgent: launchctl kickstart gui/$(id -u)/com.chagui.claude-pulse"
