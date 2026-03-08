# Claude Pulse

A macOS companion tool for [Claude Code](https://claude.ai/claude-code) that monitors active sessions, extracts knowledge from conversations, and syncs learnings to an Obsidian vault.

## Components

- **Rust CLI** (`claude-pulse`) — Session discovery, knowledge extraction, vault sync
- **Swift Menu Bar App** (`ClaudePulse`) — Real-time session monitoring, vault search, git enrichment
- **LaunchAgent** — Daily automated sync at 1:00 AM

## Prerequisites

- macOS 14+
- [Rust](https://rustup.rs) (for building the CLI)
- Xcode Command Line Tools (for building the Swift app)
- [Claude Code](https://claude.ai/claude-code) installed
- [Obsidian](https://obsidian.md) (optional, for vault viewing)

## Installation

```bash
./install.sh
```

This will:
1. Build the Rust CLI and link it to `~/.local/bin/claude-pulse`
2. Build the Swift menu bar app
3. Install the LaunchAgent for daily sync

## Configuration

Config is stored at `~/Library/Application Support/com.chagui.claude-pulse/config.json`:

```json
{
  "vault_path": "~/Documents/Obsidian/Vaults/Claude",
  "claude_projects_path": "~/.claude/projects"
}
```

## CLI Usage

```bash
# Show status (JSON output for the menu bar app)
claude-pulse status

# List unprocessed sessions from the last 7 days
claude-pulse list --days 7

# Dry-run: preview what knowledge would be extracted
claude-pulse sync --dry-run --days 7

# Run sync (creates/updates vault files)
claude-pulse sync --days 7

# Extract session text without syncing
claude-pulse extract --days 7

# Verbose logging
claude-pulse --verbose sync --days 7
```

## Running the Menu Bar App

```bash
cd ClaudePulse
swift build -c release
.build/release/ClaudePulse
```

## Uninstallation

```bash
./uninstall.sh
```

## File Locations

| Component | Path |
|-----------|------|
| Config | `~/Library/Application Support/com.chagui.claude-pulse/config.json` |
| State DB | `~/Library/Application Support/com.chagui.claude-pulse/state.db` |
| CLI Binary | `~/.local/bin/claude-pulse` |
| Logs | `~/Library/Logs/com.chagui.claude-pulse/` |
| LaunchAgent | `~/Library/LaunchAgents/com.chagui.claude-pulse.plist` |

## Troubleshooting

**CLI not found after install**: Ensure `~/.local/bin` is in your `PATH`.

**Menu bar app can't find binary**: Run `install.sh` to create the symlink at `~/.local/bin/claude-pulse`.

**Sync not running automatically**: Check the LaunchAgent:
```bash
launchctl list | grep claude-pulse
launchctl kickstart gui/$(id -u)/com.chagui.claude-pulse
```

**View sync logs**:
```bash
tail -f ~/Library/Logs/com.chagui.claude-pulse/sync.log
tail -f ~/Library/Logs/com.chagui.claude-pulse/sync.err
```

**Log rotation**: Sync logs grow over time. Use macOS `newsyslog` or periodically truncate:
```bash
: > ~/Library/Logs/com.chagui.claude-pulse/sync.log
```

## Development

```bash
# Build and check
cargo build && cargo clippy
cd ClaudePulse && swift build

# Run tests (requires cargo-nextest)
cargo nextest run
```
