# ADR-009: Cancellable sync and error visibility

## Status

Accepted

## Context

The menu bar app's sync operation (`syncNow`) invoked `claude -p` via `CLIRunner.run()` with a 30-second default timeout. Each session sync through `claude -p` typically takes 30-120+ seconds, so the process was killed before completing. The timeout error was only visible in `EmptyStateView` — when active sessions existed, the error was invisible, making it appear as though sync succeeded silently.

Additionally, the `refresh()` method chained tag scanning sequentially after git enrichment and repo scanning. Tags scan the vault directory and have no dependency on session enrichment, but were blocked by slow git operations (3 commands per session with worktree detection).

## Decision

Three changes:

1. **Cancellable sync**: Replace the timeout-based `CLIRunner.run()` for sync with a new `CLIRunner.runCancellable()` that runs without a timeout. The underlying `Process` is terminated via `withTaskCancellationHandler` when the Swift `Task` is cancelled. The user can cancel sync explicitly through a cancel button in the bottom bar.

2. **Error visibility**: Show sync errors as a compact icon in the `BottomBar` with the full message in a tooltip (`.help()`), keeping the status text visible alongside. Previously, errors replaced the entire status line or were hidden when sessions were active.

3. **Parallel tag refresh**: Run `vaultTagProvider.refresh()` in a separate `Task` from enrichment/repo scanning. Both tasks are tracked and cancelled on re-entry to prevent overlapping scans.

## Consequences

- **No silent failures**: Sync errors are always visible regardless of session state.
- **User-controlled lifecycle**: Sync runs until completion or explicit cancellation. No arbitrary timeout to tune.
- **Faster tag display**: Tags appear immediately on launch instead of waiting for all git enrichment to complete.
- **Cancellation semantics**: `Process.terminate()` sends SIGTERM. If the child process (`claude -p`) doesn't handle SIGTERM gracefully, it may leave partial state. The Rust CLI's sync already writes to the state DB per-session, so partial completion is safe.
- **Refresh task tracking**: `refreshTasks` array prevents overlapping scans but adds bookkeeping. Each `refresh()` call cancels prior tasks before spawning new ones.
