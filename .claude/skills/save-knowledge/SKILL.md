---
name: save-knowledge
description: Extract and save knowledge from the current session into an Obsidian vault. Analyzes the full conversation, identifies distinct topics, and organizes them into well-structured files. Best used at the end of a session or when switching topics.
argument-hint: "[topic hint] (optional - focus extraction on specific topic)"
---

# Save Knowledge to Obsidian Vault

You are a knowledge extraction agent. Your job is to review the current conversation and save valuable, reusable knowledge to the user's Obsidian vault.

## Prerequisites

Resolve the target vault path from the `OBSIDIAN_VAULT` environment variable. If it is not set, stop and tell the user to set `OBSIDIAN_VAULT` to the absolute path of their Obsidian vault (e.g., `export OBSIDIAN_VAULT=~/Documents/Obsidian/MyVault`). All paths below are relative to `$OBSIDIAN_VAULT`.

## Step 1: Analyze the Conversation

Review the entire conversation and identify **distinct knowledge topics**. A topic is a coherent unit of knowledge that someone would search for independently. Examples:
- A technical concept explained
- A problem and its solution
- A configuration or setup guide
- An architecture decision or comparison
- A runbook or step-by-step procedure

If the user provided an argument (`$ARGUMENTS`), focus extraction on that topic. Otherwise, extract all distinct topics worth saving.

**Skip**: Trivial exchanges, greetings, debugging dead-ends that led nowhere, and information that's already well-covered in the vault.

## Step 2: Check Existing Vault

List the top-level folders inside `$OBSIDIAN_VAULT` to learn its structure. Read `$OBSIDIAN_VAULT/_index.md` if it exists to understand what's already covered. For each topic you identified, check if a relevant file already exists:
- If yes: consider whether to **append a new section** or **update existing content**
- If no: create a new file

## Step 3: Determine File Organization

For each topic, decide:

1. **Which folder** it belongs to. Prefer existing folders. Create a new folder only for genuinely new domains that do not fit any existing one.

2. **File name** in kebab-case (e.g., `commit-graph-tuning.md`)

3. **Content type** (determines the `status` tag):
   - `reference` - Stable knowledge, API docs, concept explainers
   - `runbook` - Step-by-step actionable procedures
   - `decision` - Why a choice was made, comparisons, trade-off analysis
   - `how-to` - Practical guides for specific tasks

## Step 4: Write the Files

Each file MUST follow this format:

```markdown
---
title: Human-Readable Title
date: YYYY-MM-DD
tags:
  - domain-tag
  - technology-tag
  - type-tag
source: claude-session
status: reference|runbook|decision|how-to
---

# Title

## Context
Brief description of why this knowledge matters.

## [Main content sections]
The actual knowledge, well-structured with headers, tables, code blocks.

## Related
- [[other-note]] - Brief description of how it relates
```

Rules for content:
- **Distill, don't dump.** Extract the knowledge, not the conversation. Remove back-and-forth, false starts, and "let me check" moments.
- **Preserve precision.** Keep exact commands, config snippets, file paths, and version numbers.
- **Add wikilinks** `[[note-name]]` to connect related topics in the vault.
- **Tags** should combine a domain tag (the subject area), a technology tag (specific tool/framework), and a type tag matching the `status`. Reuse tags already present in the vault when possible.

## Step 5: Update the Index

If `$OBSIDIAN_VAULT/_index.md` exists, update it to include any new entries. Place them under the appropriate domain heading, adding a new heading if needed. If no index file exists, skip this step.

## Step 6: Report

After saving, report to the user:
- What topics were extracted
- Which files were created or updated
- How the knowledge connects to existing notes

If `$ARGUMENTS` was provided and no matching knowledge was found in the conversation, tell the user clearly rather than fabricating content.
