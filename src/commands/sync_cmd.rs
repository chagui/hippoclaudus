use anyhow::{Context, Result};

use hippoclaudus::config::Config;
use hippoclaudus::discover_sessions;
use hippoclaudus::session::extract_session;
use hippoclaudus::state::SyncState;

use crate::helpers::{file_mtime, session_id_from_path};
use crate::sync;

pub fn cmd_sync(
    config: &Config,
    dry_run: bool,
    days: u32,
    min_text_chars: usize,
    model: &str,
) -> Result<()> {
    let state = SyncState::open(config)?;
    let sessions = discover_sessions(config, Some(days));

    let mut processed = 0;
    let mut skipped = 0;

    for path in &sessions {
        let session_id = session_id_from_path(path);
        let mtime = file_mtime(path);

        if state.is_processed(&session_id, mtime) {
            continue;
        }

        let session = match extract_session(path) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Error parsing {}: {}", path.display(), e);
                continue;
            }
        };

        if session.total_text_chars < min_text_chars {
            state
                .mark_processed(
                    &session_id,
                    mtime,
                    "skipped:too_short",
                    session.total_text_chars,
                )
                .context("Failed to mark session as skipped")?;
            skipped += 1;
            continue;
        }

        println!(
            "Processing session {} ({} chars)...",
            session_id, session.total_text_chars
        );

        match sync::invoke_claude(&session, dry_run, model, config) {
            Ok(result) => {
                if result.no_new_knowledge {
                    println!("  -> No new knowledge");
                    if !dry_run {
                        state.mark_processed(
                            &session_id,
                            mtime,
                            "no_new_knowledge",
                            session.total_text_chars,
                        )?;
                    }
                } else {
                    for f in &result.created_files {
                        println!("  -> Created: {}", f);
                    }
                    for f in &result.updated_files {
                        println!("  -> Updated: {}", f);
                    }
                    if !dry_run {
                        let result_str = result
                            .created_files
                            .iter()
                            .map(|f| format!("created:{}", f))
                            .chain(
                                result
                                    .updated_files
                                    .iter()
                                    .map(|f| format!("updated:{}", f)),
                            )
                            .collect::<Vec<_>>()
                            .join(",");
                        state.mark_processed(
                            &session_id,
                            mtime,
                            &result_str,
                            session.total_text_chars,
                        )?;
                    }
                }
                processed += 1;
            }
            Err(e) => {
                log::error!("Sync error for {}: {}", session_id, e);
            }
        }
    }

    println!(
        "\nDone. Processed: {}, Skipped (too short): {}",
        processed, skipped
    );
    Ok(())
}
