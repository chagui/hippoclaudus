use hippoclaudus::config::Config;
use hippoclaudus::session::ExtractedSession;

pub fn build_prompt(session: &ExtractedSession, dry_run: bool, config: &Config) -> String {
    let vault_path = config.vault_path().display().to_string();
    let mut prompt = String::new();

    prompt.push_str(&system_instructions(&vault_path));

    if dry_run {
        prompt.push_str("\n\n## Mode: DRY RUN\n\n");
        prompt.push_str(
            "Do NOT create or modify any files. Instead, describe what you would do:\n\
             - What new files would be created (with proposed filenames and folder)\n\
             - What existing files would be updated\n\
             - A brief summary of the knowledge content for each\n\
             - If nothing worth saving, output NO_NEW_KNOWLEDGE\n",
        );
    }

    prompt.push_str(&format!(
        "\n\n---\n\n\
         Session transcript from {} (branch: {}, {} to {}):\n\n",
        session.project_cwd, session.git_branch, session.first_timestamp, session.last_timestamp,
    ));

    for exchange in &session.exchanges {
        prompt.push_str("### User:\n");
        prompt.push_str(&exchange.user_message);
        prompt.push_str("\n\n### Assistant:\n");
        prompt.push_str(&exchange.assistant_response);
        prompt.push_str("\n\n---\n\n");
    }

    prompt
}

fn system_instructions(vault_path: &str) -> String {
    format!(
        r#"You are a knowledge extraction agent. Your job is to analyze a Claude Code session transcript and save valuable, reusable knowledge to the Obsidian vault at {vault_path}/.

## Step 1: Read the Vault Index

First, read {vault_path}/_index.md to understand what's already in the vault. Use Glob to discover existing files and Grep to check for overlapping content.

## Step 2: Analyze the Transcript

Review the session transcript below and identify **distinct knowledge topics**. A topic is a coherent unit of knowledge that someone would search for independently:
- A technical concept explained (e.g., "how Bazel rules work")
- A problem and its solution (e.g., "fixing cross-AZ costs in BuildBarn")
- A configuration or setup guide (e.g., "git maintenance for large repos")
- An architecture decision or comparison
- A runbook or step-by-step procedure

**Skip**: Trivial exchanges, greetings, debugging dead-ends that led nowhere, tool invocations, and information already well-covered in the vault.

## Step 3: Create/Update Vault Files

For each knowledge topic:

1. **Check for existing coverage** — search the vault for related files
2. **Choose the right folder** — use existing folders: buildbarn/, gitaly/, git/, bazel/, or create new ones for genuinely new domains
3. **Use kebab-case filenames** (e.g., `commit-graph-tuning.md`)

Each file MUST follow this format:

```markdown
---
title: Human-Readable Title
date: YYYY-MM-DD
tags:
  - domain-tag
  - technology-tag
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

Rules:
- **Distill, don't dump.** Extract the knowledge, not the conversation.
- **Preserve precision.** Keep exact commands, config snippets, file paths, version numbers.
- **Add wikilinks** [[note-name]] to connect related topics.
- **Use tags** from: domain (buildbarn, gitaly, git, bazel, kubernetes), technology (grpc, protobuf, helm, oci), type (reference, runbook, decision, how-to)

## Step 4: Update the Index

After writing files, update {vault_path}/_index.md to include new entries under the appropriate domain heading.

## Step 5: Report

After saving, output a summary:
- CREATED: path/filename.md — for each new file
- UPDATED: path/filename.md — for each updated file
- If nothing worth saving, output exactly: NO_NEW_KNOWLEDGE

Important: Output CREATED/UPDATED lines so the calling tool can track results."#,
        vault_path = vault_path
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use hippoclaudus::session::Exchange;
    use proptest::prelude::*;

    fn make_config() -> Config {
        Config {
            vault_path: "/tmp/test-vault".to_string(),
            vault_name: None,
            claude_projects_path: "/tmp/test-projects".to_string(),
        }
    }

    fn make_session(exchanges: Vec<(&str, &str)>) -> ExtractedSession {
        let exs: Vec<Exchange> = exchanges
            .into_iter()
            .map(|(u, a)| Exchange {
                user_message: u.to_string(),
                assistant_response: a.to_string(),
            })
            .collect();
        let total = exs
            .iter()
            .map(|e| e.user_message.len() + e.assistant_response.len())
            .sum();
        ExtractedSession {
            session_id: "test-id".to_string(),
            project_cwd: "/home/user/project".to_string(),
            git_branch: "main".to_string(),
            first_timestamp: "2025-01-01T00:00:00Z".to_string(),
            last_timestamp: "2025-01-01T01:00:00Z".to_string(),
            exchanges: exs,
            total_text_chars: total,
            tool_calls: Vec::new(),
        }
    }

    // ---- proptest properties ----

    proptest! {
        #[test]
        fn prompt_contains_user_messages(
            msg1 in "[a-zA-Z0-9 ]{5,30}",
            msg2 in "[a-zA-Z0-9 ]{5,30}"
        ) {
            let config = make_config();
            let session = make_session(vec![(&msg1, "response1"), (&msg2, "response2")]);
            let prompt = build_prompt(&session, false, &config);
            prop_assert!(prompt.contains(&msg1),
                "prompt missing user message: {}", msg1);
            prop_assert!(prompt.contains(&msg2),
                "prompt missing user message: {}", msg2);
        }

        #[test]
        fn prompt_dry_run_marker(_dummy in Just(())) {
            let config = make_config();
            let session = make_session(vec![("hello", "world")]);
            let prompt = build_prompt(&session, true, &config);
            prop_assert!(prompt.contains("DRY RUN"),
                "dry run prompt missing DRY RUN marker");
        }

        #[test]
        fn prompt_contains_vault_path(_dummy in Just(())) {
            let config = make_config();
            let session = make_session(vec![("hello", "world")]);
            let prompt = build_prompt(&session, false, &config);
            prop_assert!(prompt.contains("/tmp/test-vault"),
                "prompt missing vault path");
        }
    }

    // ---- unit tests ----

    #[test]
    fn build_prompt_contains_vault_path() {
        let config = make_config();
        let session = make_session(vec![("hello", "world")]);
        let prompt = build_prompt(&session, false, &config);
        assert!(prompt.contains("/tmp/test-vault"));
    }

    #[test]
    fn build_prompt_dry_run_adds_marker() {
        let config = make_config();
        let session = make_session(vec![("hello", "world")]);
        let prompt = build_prompt(&session, true, &config);
        assert!(prompt.contains("DRY RUN"));
        assert!(prompt.contains("Do NOT create or modify any files"));
    }

    #[test]
    fn build_prompt_no_dry_run_marker_in_normal_mode() {
        let config = make_config();
        let session = make_session(vec![("hello", "world")]);
        let prompt = build_prompt(&session, false, &config);
        assert!(!prompt.contains("DRY RUN"));
    }

    #[test]
    fn build_prompt_formats_exchanges() {
        let config = make_config();
        let session = make_session(vec![("How do I write tests?", "Use #[test] attribute.")]);
        let prompt = build_prompt(&session, false, &config);
        assert!(prompt.contains("### User:"));
        assert!(prompt.contains("### Assistant:"));
        assert!(prompt.contains("How do I write tests?"));
        assert!(prompt.contains("Use #[test] attribute."));
    }

    #[test]
    fn build_prompt_contains_session_metadata() {
        let config = make_config();
        let session = make_session(vec![("hello", "world")]);
        let prompt = build_prompt(&session, false, &config);
        assert!(prompt.contains("/home/user/project"));
        assert!(prompt.contains("main"));
    }
}
