---
name: save-note
description: Quickly save a single piece of knowledge to an Obsidian vault. Unlike /save-knowledge which analyzes the full conversation, this saves a focused note about a specific topic currently being discussed. Good for mid-session saves.
argument-hint: "<title> (required - short title for the note)"
---

# Quick Save Note

Save a focused note about the topic currently being discussed. This is the lightweight version of `/save-knowledge` — it saves one note about one topic without analyzing the full conversation history.

## Prerequisites

Resolve the target vault path from the `OBSIDIAN_VAULT` environment variable. If it is not set, stop and tell the user to set `OBSIDIAN_VAULT` to the absolute path of their Obsidian vault (e.g., `export OBSIDIAN_VAULT=~/Documents/Obsidian/MyVault`).

## What to do

1. **Title**: Use `$ARGUMENTS` as the note title. If no argument was provided, infer the topic from the most recent exchange in the conversation.

2. **Check existing vault**: Read `$OBSIDIAN_VAULT/_index.md` if it exists. Check if a file on this topic already exists.
   - If yes: update or append to the existing file.
   - If no: create a new file.

3. **Determine placement**: List the top-level folders inside `$OBSIDIAN_VAULT` and pick the one that best fits the topic. Create a new folder only if no existing folder fits.

4. **Write the note** with this structure:

```markdown
---
title: Title From Arguments
date: YYYY-MM-DD
tags:
  - relevant-tags
source: claude-session
status: reference|runbook|decision|how-to
---

# Title

[Distilled knowledge from the current discussion. Keep it focused and precise.]

## Related
- [[links-to-related-notes]]
```

5. **Update `$OBSIDIAN_VAULT/_index.md`** if it exists and a new file was created.

6. **Confirm** to the user what was saved and where.

## Rules

- Keep it focused — one topic per note.
- Distill, don't dump conversation verbatim.
- Preserve exact commands, configs, file paths, and versions.
- Use `[[wikilinks]]` to connect to other notes in the vault.
- Use kebab-case for file names.
