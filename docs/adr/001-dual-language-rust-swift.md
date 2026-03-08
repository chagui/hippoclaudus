# ADR-001: Dual-language architecture: Rust CLI + Swift menu bar

## Status

Accepted

## Context

Claude Pulse needs to do two fundamentally different things: heavy I/O work (scanning JSONL files, querying SQLite, running git commands, invoking `claude -p`) and providing a responsive native macOS menu bar UI. A single-language solution would compromise on one axis — Rust has no native macOS UI story, and Swift is not ideal for CLI tools with complex file I/O and subprocess orchestration.

## Decision

Split the system into two components:

- **Rust CLI** (`claude-pulse`): All data processing, session discovery, JSONL parsing, state management, and sync orchestration.
- **Swift menu bar app** (`ClaudePulse/`): Native macOS UI that invokes the Rust CLI as a subprocess and presents results.

The interface between them is **JSON over stdout**. The Swift app calls `CLIRunner.run()` (with a configurable timeout, default 30s), `CLIRunner.runCancellable()` (no timeout, process terminated on Task cancellation), or `CLIRunner.runStreaming()` (line-by-line for long operations). The CLI always outputs structured JSON (`StatusResponse`, `ActiveSessionResponse`, etc.).

## Consequences

- **CLI is independently useful**: Users can script it, run it in cron, or use it without the menu bar app.
- **Two build systems**: Cargo for Rust, Swift Package Manager for Swift. CI must build and test both.
- **JSON contract maintenance**: Changes to CLI output format must be coordinated with Swift struct definitions. No shared schema — the contract is implicit.
- **No in-process communication overhead**: The subprocess boundary adds latency (~50-100ms per invocation) but provides process isolation — a CLI crash doesn't take down the menu bar app.
