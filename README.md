# Hippoclaudus

A macOS companion tool for [Claude Code](https://claude.ai/claude-code) that monitors active sessions, extracts knowledge from conversations, and syncs learnings to an Obsidian vault.

## Components

- **Rust CLI** (`hpc`) — Session discovery, knowledge extraction, vault sync
- **Swift Menu Bar App** (`Hippo`) — Real-time session monitoring, vault search, git enrichment
- **LaunchAgent** — Daily automated sync at 1:00 AM

## Prerequisites

- macOS 14+
- [Rust](https://rustup.rs) (for building the CLI)
- Xcode Command Line Tools (for building the Swift app)
- [Claude Code](https://claude.ai/claude-code) installed
- [Obsidian](https://obsidian.md) (optional, for vault viewing)

## Installation

```bash
./scripts/install.sh
```

This will:
1. Build the Rust CLI and link it to `~/.local/bin/hpc`
2. Build the Swift menu bar app
3. Install the LaunchAgent for daily sync

## Configuration

Config is stored at `~/Library/Application Support/com.chagui.hippoclaudus/config.json`:

```json
{
  "vault_path": "~/Documents/Obsidian/Vaults/Claude",
  "vault_name": "Claude",
  "claude_projects_path": "~/.claude/projects"
}
```

`vault_name` is optional — it's the Obsidian vault name used in `obsidian://open?vault=...` URLs. If omitted, the last path component of `vault_path` is used (e.g. `Claude` above). Set it explicitly when your Obsidian-registered vault name differs from the folder name.

## CLI Usage

```bash
# Show status (JSON output for the menu bar app)
hpc status

# List unprocessed sessions from the last 7 days
hpc list --days 7

# Dry-run: preview what knowledge would be extracted
hpc sync --dry-run --days 7

# Run sync (creates/updates vault files)
hpc sync --days 7

# Extract session text without syncing
hpc extract --days 7

# Verbose logging
hpc --verbose sync --days 7
```

## Running the Menu Bar App

```bash
cd Hippo
swift build -c release
.build/release/Hippo
```

## Claude Code Skills

The `.claude/skills/` directory ships two slash commands for use inside a Claude Code session:

- `/save-note <title>` — Save a focused note about the current topic to your Obsidian vault.
- `/save-knowledge [topic]` — Extract all distinct topics from the session and organize them into vault files.

Both require an `OBSIDIAN_VAULT` environment variable pointing at the absolute path of your vault:

```bash
export OBSIDIAN_VAULT=~/Documents/Obsidian/Vaults/Claude
```

The skills are complementary to automated sync: use them for ad-hoc saves mid-session, and let the LaunchAgent handle the daily batch pass.

## Uninstallation

```bash
./scripts/uninstall.sh
```

## File Locations

| Component | Path |
|-----------|------|
| Config | `~/Library/Application Support/com.chagui.hippoclaudus/config.json` |
| State DB | `~/Library/Application Support/com.chagui.hippoclaudus/state.db` |
| CLI Binary | `~/.local/bin/hpc` |
| Logs | `~/Library/Logs/com.chagui.hippoclaudus/` |
| LaunchAgent | `~/Library/LaunchAgents/com.chagui.hippoclaudus.plist` |

## Troubleshooting

**CLI not found after install**: Ensure `~/.local/bin` is in your `PATH`.

**Menu bar app can't find binary**: Run `scripts/install.sh` to create the symlink at `~/.local/bin/hpc`.

**Sync not running automatically**: Check the LaunchAgent:
```bash
launchctl list | grep hippoclaudus
launchctl kickstart gui/$(id -u)/com.chagui.hippoclaudus
```

**View sync logs**:
```bash
tail -f ~/Library/Logs/com.chagui.hippoclaudus/sync.log
tail -f ~/Library/Logs/com.chagui.hippoclaudus/sync.err
```

**Log rotation**: Sync logs grow over time. Use macOS `newsyslog` or periodically truncate:
```bash
: > ~/Library/Logs/com.chagui.hippoclaudus/sync.log
```

## Development

```bash
# Build and check
cargo build && cargo clippy
cd Hippo && swift build

# Run tests (requires cargo-nextest)
cargo nextest run
```
