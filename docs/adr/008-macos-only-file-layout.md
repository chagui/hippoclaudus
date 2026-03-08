# ADR-008: macOS-only file layout following OS conventions

## Status

Accepted

## Context

Claude Pulse includes a native macOS menu bar app (Swift/AppKit) and uses LaunchAgent for scheduling. The file layout must follow platform conventions so that macOS features (Time Machine exclusions, Spotlight indexing, permission dialogs) work correctly. Cross-platform portability is not a current goal.

## Decision

Use standard macOS paths:

| Purpose | Path |
|---------|------|
| Config | `~/Library/Application Support/com.chagui.claude-pulse/config.json` |
| State DB | `~/Library/Application Support/com.chagui.claude-pulse/state.db` |
| Logs | `~/Library/Logs/com.chagui.claude-pulse/` |
| CLI binary | `~/.local/bin/claude-pulse` |
| LaunchAgent | `~/Library/LaunchAgents/com.chagui.claude-pulse.plist` |

Config files use `~/` (tilde) notation internally, expanded at load time via `expand_tilde()`. All config and state files are created with `0600` permissions (owner-only) since they contain session metadata and filesystem paths.

## Consequences

- **macOS-native behavior**: `Application Support` is automatically excluded from iCloud sync. `LaunchAgents` integrates with launchd for reliable scheduling (daily sync at 1 AM).
- **Not portable**: Linux users would need different paths (`~/.config/`, `~/.local/share/`, systemd timers). This is an explicit non-goal for now.
- **Reverse DNS naming** (`com.chagui.claude-pulse`): Follows Apple conventions, avoids collisions with other applications.
- **Tilde in config**: Makes config files portable across machines with different usernames — the same `config.json` works on any Mac.
