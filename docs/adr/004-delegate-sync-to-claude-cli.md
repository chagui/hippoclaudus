# ADR-004: Delegate knowledge sync to `claude -p`

## Status

Accepted

## Context

The core value proposition of Hippoclaudus is extracting reusable knowledge from Claude Code sessions and organizing it in an Obsidian vault. This is a semantic task: deciding what constitutes useful knowledge, how to categorize it, whether to create a new file or update an existing one, and how to respect the vault's existing structure. These decisions require judgment that is difficult to encode in rules.

## Decision

Build a structured prompt from extracted session data and invoke `claude -p` (the Claude Code CLI in prompt mode) inside the vault directory. The prompt includes:

- System instructions for knowledge extraction behavior
- The vault's `_index.md` contents (so the AI understands existing structure)
- Session metadata (project, branch, timestamps)
- Formatted conversation exchanges

The `claude -p` call is scoped with `--allowedTools`:
- **Production mode**: `Read,Write,Edit,Glob,Grep` — full vault access
- **Dry-run mode**: `Read,Glob,Grep` — read-only, for previewing what would be created

Results are parsed from stdout by matching `CREATED:` and `UPDATED:` lines.

## Consequences

- **Quality scales with model capability**: As Claude improves, knowledge extraction improves without code changes.
- **Requires Claude Code CLI**: The sync feature depends on `claude` being installed and authenticated. Offline or API-outage scenarios cause sync failures.
- **Cost per sync**: Each session sync is an API call. Daily automation at 1 AM bounds this to one batch per day for non-interactive use.
- **Opaque extraction logic**: The AI decides what to extract and where to put it. Debugging "why was this session skipped?" requires inspecting the prompt and model behavior, not code.
- **Vault structure freedom**: Users can organize their vault however they want — the AI adapts to existing patterns rather than imposing a fixed schema.
