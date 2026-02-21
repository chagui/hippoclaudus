#!/bin/bash
set -e

PLIST_NAME="com.chagui.claude-pulse"

echo "==> Unloading LaunchAgent..."
launchctl bootout "gui/$(id -u)/$PLIST_NAME" 2>/dev/null || true
rm -f ~/Library/LaunchAgents/$PLIST_NAME.plist
echo "    LaunchAgent removed"

echo "==> Removing CLI symlink..."
rm -f ~/.local/bin/claude-pulse
echo "    Symlink removed"

echo "==> Removing application data..."
rm -rf "$HOME/Library/Application Support/com.chagui.claude-pulse"
echo "    Config and state DB removed"

echo "==> Removing logs..."
rm -rf "$HOME/Library/Logs/com.chagui.claude-pulse"
echo "    Logs removed"

echo ""
echo "==> Uninstall complete!"
echo "    Note: Build artifacts in the repo (target/, .build/) were not removed."
echo "    Run 'cargo clean' and 'swift package clean' to remove them."
