# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for Hippoclaudus.

We use ADRs to document significant architectural decisions, their context, and rationale. Each record follows the format from [Michael Nygard's article](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions).

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [001](001-dual-language-rust-swift.md) | Dual-language architecture: Rust CLI + Swift menu bar | Accepted |
| [002](002-sqlite-state-management.md) | SQLite for state management | Accepted |
| [003](003-streaming-jsonl-parsing.md) | Streaming JSONL parsing with pre-filters | Accepted |
| [004](004-delegate-sync-to-claude-cli.md) | Delegate knowledge sync to `claude -p` | Accepted |
| [005](005-property-based-testing.md) | Property-based testing for parsers | Accepted |
| [006](006-session-discovery-threshold.md) | 5-minute threshold for session discovery | Accepted |
| [007](007-commit-ratio-metric.md) | Commit ratio as an AI output confidence signal | Accepted |
| [008](008-macos-only-file-layout.md) | macOS-only file layout following OS conventions | Accepted |
| [009](009-cancellable-sync-and-error-visibility.md) | Cancellable sync and error visibility | Accepted |
