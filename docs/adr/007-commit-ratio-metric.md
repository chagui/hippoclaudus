# ADR-007: Commit ratio as an AI output confidence signal

## Status

Accepted

## Context

When Claude Code writes or edits files during a session, some of those changes are exploratory (debugging, temporary scaffolding) and some are production-quality (shipped features, bug fixes). Users and the system benefit from a signal that estimates how much of the AI's output was "useful" — defined as making it into git commits.

## Decision

Compute a **commit ratio** per session:

```
commit_ratio = (files_in_commits ∩ files_touched_by_ai) / files_touched_by_ai × 100%
```

Implementation (`git_stats.rs`):
1. Collect unique file paths from Write/Edit tool calls in the session, normalized to repo-relative paths.
2. Run `git log --since=<window> --name-only` to get all committed files in that time window.
3. Intersect the two sets and compute the percentage.

Edge cases:
- Non-git directories return `None` (not 0% — absence of signal, not negative signal).
- Empty tool call set returns `Some(0.0)` — the session produced no file changes.
- Files outside the repo root are excluded from the intersection.

## Consequences

- **Useful heuristic, not ground truth**: A low ratio could mean the AI explored alternatives (good) or wrote bad code (bad). Context matters.
- **Time window sensitivity**: Uses a configurable window (default 7 days). Commits made days after a session still count, which is correct for staged workflows but could over-attribute.
- **Git dependency**: Only works in git repositories. Non-git projects get no ratio.
- **Displayed in menu bar**: The Swift app shows commit ratio per session, giving users a quick quality signal.
