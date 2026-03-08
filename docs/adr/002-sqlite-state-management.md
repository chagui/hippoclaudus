# ADR-002: SQLite for state management

## Status

Accepted

## Context

Hippoclaudus needs to track which sessions have been processed, per-session statistics (tool counts, timing, commit ratios), and discovered prompt files. The original implementation used a JSON file, which required manual file locking and had no query capability. The menu bar app polls status while the CLI might be syncing, creating concurrent read/write scenarios.

## Decision

Use SQLite with WAL (Write-Ahead Logging) mode as the state store. The database lives at `~/Library/Application Support/com.chagui.hippoclaudus/state.db` with `0600` permissions.

Tables:
- `processed_sessions`: session_id, mtime, processed_at, result (created/updated/skipped), text_chars
- `metadata`: key-value store (e.g., `last_run` timestamp)
- `session_stats`: per-session metrics (duration, tool counts, commit ratio)
- `prompt_files`: AI-written files with confidence scores

Auto-migration from the old JSON state file runs on first access (`migrate_from_json()`).

## Consequences

- **Concurrent safety**: WAL mode allows the menu bar app to read status while the CLI writes sync results without locking conflicts.
- **Queryable state**: Time-windowed queries (`get_all_session_stats(days)`) and aggregations are trivial in SQL.
- **Bundled SQLite**: The `rusqlite` crate with `bundled` feature compiles SQLite from source, avoiding system library version issues at the cost of ~2s added compile time.
- **Migration path**: Existing users with JSON state files are migrated transparently on first run. The old file is left in place as a backup.
