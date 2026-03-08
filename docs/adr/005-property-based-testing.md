# ADR-005: Property-based testing for parsers

## Status

Accepted

## Context

The codebase has several string-manipulation and parsing functions that must handle arbitrary input safely: `filter_noise`, `extract_user_text`, `extract_assistant_text`, `project_name_from_cwd`, `expand_tilde`, `truncate_str`, and worktree path detection. These functions process data from external sources (Claude Code JSONL files, user config, filesystem paths) where edge cases are unpredictable — empty strings, Unicode boundaries, deeply nested paths, malformed JSON.

## Decision

Use `proptest` for property-based testing alongside conventional unit tests. Each function has invariants expressed as properties:

- **filter_noise**: Idempotent (applying twice equals applying once), output never longer than input, no noise patterns remain in output.
- **project_name_from_cwd**: Deep paths contain `/`, no trailing slashes, empty input yields empty output.
- **expand_tilde**: `~/path` expands to `$HOME/path`, absolute paths pass through unchanged.
- **extract_user_text / extract_assistant_text**: Never panic on arbitrary JSON strings.
- **truncate_str**: Respects UTF-8 character boundaries (naive byte slicing would panic).
- **build_prompt**: All user messages appear in output, dry-run marker present when expected.

## Consequences

- **Catches edge cases that hand-written tests miss**: The `truncate_str` UTF-8 boundary issue was caught by property tests, not unit tests.
- **Slower test execution**: Property tests generate thousands of random inputs per run. Acceptable for a CLI tool's test suite but adds a few seconds.
- **Invariants must be correct**: A wrong property assertion gives false confidence. Each property was chosen to be obviously true about the function's contract.
- **Fuzzing targets complement property tests**: The `fuzz/` directory contains Cargo fuzz targets for deeper exploration of input space on critical paths.
