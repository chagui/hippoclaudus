use anyhow::Result;

use claude_pulse::config::Config;
use claude_pulse::prompts::detect_prompt_files;
use claude_pulse::session::extract_session;
use claude_pulse::state::SyncState;
use claude_pulse::{discover_active_sessions, discover_sessions};

pub fn cmd_prompts(config: &Config, days: u32, threshold: f64, json: bool) -> Result<()> {
    let state = SyncState::open(config)?;

    // Scan sessions and extract tool calls for prompt detection
    let all_paths = discover_sessions(config, Some(days));
    let active_paths = discover_active_sessions(config);

    let mut all_prompts = Vec::new();

    for path in all_paths.iter().chain(active_paths.iter()) {
        let session = match extract_session(path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let prompts = detect_prompt_files(
            &session.tool_calls,
            &session.session_id,
            &session.project_cwd,
            threshold,
        );

        // Store in DB
        for p in &prompts {
            let _ = state.upsert_prompt_file(
                &p.file_path,
                &p.session_id,
                &p.project_cwd,
                &p.written_at,
                &p.content_preview,
                p.prompt_confidence,
            );
        }

        all_prompts.extend(prompts);
    }

    // Also include previously discovered prompts from DB
    let db_prompts = state.get_prompt_files(Some(days))?;
    for dbp in &db_prompts {
        if !all_prompts
            .iter()
            .any(|p| p.file_path == dbp.file_path && p.session_id == dbp.session_id)
        {
            all_prompts.push(claude_pulse::prompts::PromptFileInfo {
                file_path: dbp.file_path.clone(),
                session_id: dbp.session_id.clone(),
                project_cwd: dbp.project_cwd.clone(),
                written_at: dbp.written_at.clone(),
                content_preview: dbp.content_preview.clone(),
                prompt_confidence: dbp.prompt_confidence,
                file_exists: std::path::Path::new(&dbp.file_path).exists(),
            });
        }
    }

    // Sort by confidence descending
    all_prompts.sort_by(|a, b| {
        b.prompt_confidence
            .partial_cmp(&a.prompt_confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&all_prompts).unwrap());
    } else {
        if all_prompts.is_empty() {
            println!(
                "No prompt files detected in the last {} days (threshold: {}).",
                days, threshold
            );
            return Ok(());
        }

        println!(
            "{:<60} {:<12} {:<10} {:<6} {}",
            "PATH", "SESSION", "DATE", "CONF", "EXISTS"
        );
        println!("{}", "-".repeat(100));

        for p in &all_prompts {
            let short_path = if p.file_path.len() > 58 {
                format!("...{}", &p.file_path[p.file_path.len() - 55..])
            } else {
                p.file_path.clone()
            };
            let short_session = if p.session_id.len() > 10 {
                format!("{}...", &p.session_id[..8])
            } else {
                p.session_id.clone()
            };
            let date = if p.written_at.len() >= 10 {
                &p.written_at[..10]
            } else {
                &p.written_at
            };
            let exists = if p.file_exists { "yes" } else { "no" };
            println!(
                "{:<60} {:<12} {:<10} {:<6.2} {}",
                short_path, short_session, date, p.prompt_confidence, exists
            );
        }

        println!("\n{} prompt file(s) found.", all_prompts.len());
    }

    Ok(())
}
