# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What is Hippoclaudus

A macOS companion tool for Claude Code that monitors active sessions, extracts knowledge from JSONL conversation logs, and syncs it to an Obsidian vault. Two components: a Rust CLI (`hpc`) and a Swift menu bar app (`Hippo/`).

## Dev Setup

```bash
./scripts/setup.sh  # installs cargo-nextest, cargo-llvm-cov, lefthook, git hooks
```

Pre-commit hooks (via [lefthook](https://github.com/evilmartians/lefthook), config in `lefthook.yml`):
- **Rust**: `cargo fmt --check`, `cargo clippy -- -D warnings`
- **Shell**: `shfmt -d`, `shellcheck`
- **Swift**: `swiftformat --lint`, `swift build -c release`
- **YAML**: `yamllint`

## Build & Test Commands

```bash
# Rust CLI
cargo build --release
cargo nextest run             # unit + property-based + integration tests (requires cargo-nextest)
cargo fmt --check             # formatting
cargo clippy -- -D warnings   # lint (CI treats warnings as errors)

# Swift menu bar app
cd Hippo && swift build -c release
cd Hippo && swift test  # uses swift-testing framework

# Install/uninstall (builds both, symlinks CLI to ~/.local/bin, installs LaunchAgent)
./scripts/install.sh [--cli-only]
./scripts/uninstall.sh
```

## Architecture

**Rust CLI** (`src/`): Session discovery, JSONL parsing, knowledge extraction, sync orchestration via `claude -p`.

- `main.rs` → CLI entry point, clap subcommand dispatch
- `lib.rs` → Public API surface (`discover_sessions`, `discover_active_sessions`, `vault_note_count`)
- `session.rs` → JSONL streaming parser. Extracts message pairs, tool calls (Write/Edit/Bash), timestamps, git metadata. Filters noise patterns.
- `config.rs` → Config loading from `~/Library/Application Support/com.chagui.hippoclaudus/config.json`
- `state.rs` → SQLite state DB (WAL mode, 0600 perms). Tracks processed sessions, stats, prompt files.
- `sync.rs` → Invokes `claude -p` with extracted knowledge. Dry-run restricts tools to Read/Glob/Grep; production adds Write/Edit. Parses CREATED:/UPDATED: markers from stdout.
- `git_stats.rs` → Commit ratio: % of Claude-touched files that made it into git commits
- `commands/` → Subcommand handlers: `status`, `list`, `extract`, `sync_cmd`, `stats`, `prompts`

**Swift Menu Bar App** (`Hippo/`): Real-time session monitoring, vault search, git enrichment.

- `StatusProvider.swift` → Fetches CLI `status` output on a refresh timer
- `CLIRunner.swift` → Executes Rust CLI binary
- `SearchProvider.swift` → Vault search
- `GitEnrichmentProvider.swift` → Branch/status/recent commits overlay

**Data flow**: JSONL session files (`~/.claude/projects/`) → Rust CLI parses & extracts → `claude -p` syncs to Obsidian vault. Menu bar app polls CLI for status JSON.

**Session discovery**: Walks `~/.claude/projects/*/*.jsonl`, skips files modified <5 min ago (still active), filters by age. Worktree detection extracts canonical repo root from `.claude/worktrees/<name>/` paths.

## Key File Locations

- Config: `~/Library/Application Support/com.chagui.hippoclaudus/config.json`
- State DB: `~/Library/Application Support/com.chagui.hippoclaudus/state.db`
- Logs: `~/Library/Logs/com.chagui.hippoclaudus/`
- LaunchAgent: `com.chagui.hippoclaudus.plist` (daily sync at 1:00 AM)

## Testing

Rust tests use `proptest` for property-based testing and `tempfile` for isolated state DB tests. Integration tests live in `tests/`. Swift tests use Apple's `swift-testing` framework. Fuzzing targets are in `fuzz/`.

## Code Coverage

```bash
# Both components
./scripts/coverage.sh          # summary
./scripts/coverage.sh --html   # HTML reports, opens in browser

# Individual
./scripts/coverage.sh rust     # Rust only
./scripts/coverage.sh swift    # Swift only
```

Rust coverage requires `cargo-llvm-cov` (`cargo install cargo-llvm-cov`) and `cargo-nextest` (`cargo install cargo-nextest`). Swift coverage is built-in via `swift test --enable-code-coverage`.

**After any code change, run `./scripts/coverage.sh` and verify coverage does not regress.** Target thresholds:
- Rust: **80%** line coverage (`cargo llvm-cov --fail-under-lines 80`)
- Swift testable logic (models, providers, parsers): **60%** line coverage. SwiftUI views (`ContentView.swift`, `HippoApp.swift`) and process-dependent providers (`GitEnrichmentProvider.swift`) are excluded from this target since they require mocking infrastructure that doesn't exist yet.

## CI

GitHub Actions (`.github/workflows/ci.yml`) runs on macOS: fmt check → build (Rust + Swift) → clippy → test.
