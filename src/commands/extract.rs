use anyhow::Result;

use claude_pulse::config::Config;
use claude_pulse::discover_sessions;
use claude_pulse::session::extract_session;
use claude_pulse::state::SyncState;

use crate::helpers::{file_mtime, session_id_from_path, truncate};

pub fn cmd_extract(config: &Config, days: u32, min_text_chars: usize) -> Result<()> {
    let state = SyncState::open(config)?;
    let sessions = discover_sessions(config, Some(days));

    let mut count = 0;
    for path in &sessions {
        let session_id = session_id_from_path(path);
        let mtime = file_mtime(path);

        if state.is_processed(&session_id, mtime) {
            continue;
        }

        match extract_session(path) {
            Ok(session) => {
                if session.total_text_chars < min_text_chars {
                    continue;
                }
                count += 1;
                println!("=== Session: {} ===", session.session_id);
                println!(
                    "Project: {} | Branch: {} | {} to {}",
                    session.project_cwd,
                    session.git_branch,
                    session.first_timestamp,
                    session.last_timestamp
                );
                println!(
                    "Exchanges: {} | Text chars: {}",
                    session.exchanges.len(),
                    session.total_text_chars
                );
                println!();
                for (i, exchange) in session.exchanges.iter().enumerate() {
                    println!("--- Exchange {} ---", i + 1);
                    println!("User: {}", truncate(&exchange.user_message, 200));
                    println!("Assistant: {}", truncate(&exchange.assistant_response, 200));
                    println!();
                }
            }
            Err(e) => {
                log::warn!("Error parsing {}: {}", path.display(), e);
            }
        }
    }

    if count == 0 {
        println!(
            "No unprocessed sessions found with >= {} chars in the last {} days.",
            min_text_chars, days
        );
    } else {
        println!("\nExtracted {} sessions.", count);
    }
    Ok(())
}
