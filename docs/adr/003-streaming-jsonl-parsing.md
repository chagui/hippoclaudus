# ADR-003: Streaming JSONL parsing with pre-filters

## Status

Accepted

## Context

Claude Code sessions can run for hours, generating JSONL files that are tens of megabytes. Each line is a JSON object representing a message, tool call, progress update, or system event. Most lines are irrelevant to knowledge extraction (progress updates, queue operations, file history entries). Loading entire files into memory and deserializing every line is wasteful.

## Decision

Parse JSONL files line-by-line using `BufReader` with two optimization layers:

1. **String pre-filter before JSON parsing**: Check for `"type":"user"`, `"type":"assistant"`, or `"type":"system"` substrings before attempting `serde_json::from_str`. This skips ~70% of lines without any JSON parsing cost.

2. **Tail-based fast path for active session detection**: When checking if a session is still active (for `discover_active_sessions`), read only the last N bytes and scan backward for the last entry type — avoids reading the entire file.

Additionally, noise patterns (`<system-reminder>`, `<command-name>`, `[Request interrupted`) are stripped from extracted text using a filter that is tested for idempotency via property tests.

## Consequences

- **Memory usage is O(max line size)**, not O(file size). Practical memory footprint stays under a few MB even for large sessions.
- **Pre-filters are fragile to format changes**: If Claude Code changes the JSON field ordering or spacing, the string-matching pre-filter could miss valid lines. The filter errs on the side of inclusion — unrecognized lines fall through to JSON parsing.
- **Tail-based fast path trades accuracy for speed**: It could miss the true last entry if the file was partially written, but this only affects the active/inactive classification, not data extraction.
