# ADR-006: 5-minute threshold for session discovery

## Status

Accepted

## Context

Claude Code writes to JSONL session files as conversations progress. A sync process that reads a file while it's still being written to would extract an incomplete session, potentially missing the most important exchanges (conclusions, final code). The system needs to distinguish "active" sessions (still being written) from "complete" sessions (safe to process).

## Decision

Use file modification time (`mtime`) with a 5-minute threshold:

- `discover_sessions()`: Returns files with mtime **older** than 5 minutes — considered complete and safe to process.
- `discover_active_sessions()`: Returns files with mtime **newer** than 5 minutes — considered still active, shown in the menu bar status UI but not synced.

Both functions also accept a `max_age_days` parameter to skip sessions older than a configurable window (default: 7 days for sync, configurable for status).

## Consequences

- **Simple and reliable**: No need for file locks, inotify watchers, or process detection. Mtime is universally available.
- **5-minute gap**: A session that had its last exchange 4 minutes ago is still classified as "active" even if the user has moved on. This delays processing by at most 5 minutes — acceptable for a daily sync workflow.
- **False negatives in rapid usage**: If the CLI is invoked manually right after ending a session, the latest session might not appear. The daily LaunchAgent job at 1 AM eliminates this concern for automated use.
- **No "session end" signal**: Claude Code doesn't write an explicit end-of-session marker. Mtime-based heuristics are the pragmatic alternative.
