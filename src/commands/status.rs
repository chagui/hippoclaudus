use std::path::Path;

use anyhow::Result;

use hippoclaudus::config::Config;
use hippoclaudus::pricing::cost_usd;
use hippoclaudus::session::{extract_session_metadata, ActiveSessionInfo};
use hippoclaudus::state::SyncState;
use hippoclaudus::{discover_active_sessions, discover_sessions, vault_note_count};

use crate::helpers::{file_mtime, session_id_from_path, vault_size_kb};

/// Days of history to surface in `inactive_sessions` (non-active sessions still worth showing in the menu bar).
const INACTIVE_WINDOW_DAYS: u32 = 1;

fn session_info_to_json(info: &ActiveSessionInfo) -> serde_json::Value {
    let cost = cost_usd(
        &info.model,
        info.total_input_tokens,
        info.total_output_tokens,
        info.total_cache_read_tokens,
        info.total_cache_creation_tokens,
    );
    let mut obj = serde_json::json!({
        "session_id": info.session_id,
        "project_name": info.project_name,
        "project_cwd": info.project_cwd,
        "git_branch": info.git_branch,
        "started_at": info.started_at,
        "exchange_count": info.exchange_count,
        "model": info.model,
        "total_input_tokens": info.total_input_tokens,
        "total_output_tokens": info.total_output_tokens,
        "total_cache_read_tokens": info.total_cache_read_tokens,
        "total_cache_creation_tokens": info.total_cache_creation_tokens,
        "cost_usd": cost,
        "avg_turn_duration_ms": info.avg_turn_duration_ms,
        "turn_count": info.turn_count,
        "state": info.state,
        "write_count": info.write_count,
        "edit_count": info.edit_count,
        "bash_count": info.bash_count,
        "files_touched_count": info.files_touched_count,
    });
    if let Some(ref timing) = info.timing {
        obj["agent_time_ms"] = serde_json::json!(timing.agent_time_ms);
        obj["user_time_ms"] = serde_json::json!(timing.user_time_ms);
        obj["agent_time_pct"] = serde_json::json!(timing.agent_time_pct);
        obj["user_time_pct"] = serde_json::json!(timing.user_time_pct);
    }
    if let Some(ref wt_root) = info.worktree_root {
        obj["worktree_root"] = serde_json::json!(wt_root);
    }
    obj
}

fn paths_to_session_json(paths: &[impl AsRef<Path>]) -> Vec<serde_json::Value> {
    paths
        .iter()
        .filter_map(|path| extract_session_metadata(path.as_ref()).ok())
        .map(|info| session_info_to_json(&info))
        .collect()
}

pub fn cmd_status(config: &Config) -> Result<()> {
    let state = SyncState::open(config)?;
    let all_sessions = discover_sessions(config, None);

    let processed_sessions = state.processed_sessions()?;

    let total = all_sessions.len();
    let mut processed_count = 0;
    let mut skipped_count = 0;
    let mut knowledge_count = 0;

    for path in &all_sessions {
        let sid = session_id_from_path(path);
        let mtime = file_mtime(path);
        if state.is_processed(&sid, mtime) {
            processed_count += 1;
            if let Some(entry) = processed_sessions.get(&sid) {
                if entry.result.starts_with("skipped:") {
                    skipped_count += 1;
                } else if entry.result.starts_with("created:")
                    || entry.result.starts_with("updated:")
                {
                    knowledge_count += 1;
                }
            }
        }
    }

    let unprocessed = total.saturating_sub(processed_count);
    let vault_notes = vault_note_count(config);

    let last_run = state.last_run();

    let last_sync = last_run
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "null".to_string());

    let last_sync_ago = last_run
        .map(|dt| {
            let now = chrono::Utc::now();
            let dur = now.signed_duration_since(dt);
            if dur.num_days() > 0 {
                format!("{}d ago", dur.num_days())
            } else if dur.num_hours() > 0 {
                format!("{}h ago", dur.num_hours())
            } else if dur.num_minutes() > 0 {
                format!("{}m ago", dur.num_minutes())
            } else {
                "just now".to_string()
            }
        })
        .unwrap_or_else(|| "never".to_string());

    let vault_size_kb = vault_size_kb(config);

    let active_paths = discover_active_sessions(config);
    let active_sessions = paths_to_session_json(&active_paths);
    let active_session_count = active_sessions.len();

    // Inactive sessions: modified outside the active window but within INACTIVE_WINDOW_DAYS.
    let inactive_paths = discover_sessions(config, Some(INACTIVE_WINDOW_DAYS));
    let inactive_sessions = paths_to_session_json(&inactive_paths);
    let inactive_session_count = inactive_sessions.len();

    // Aggregate stats from DB (last 7 days)
    let aggregate_stats = state.get_all_session_stats(7).ok().map(|stats| {
        let mut agg_agent_ms: u64 = 0;
        let mut agg_user_ms: u64 = 0;
        let mut agg_writes: usize = 0;
        let mut agg_edits: usize = 0;
        let mut agg_bashes: usize = 0;
        let mut agg_files: usize = 0;
        let session_count = stats.len();
        for s in &stats {
            agg_agent_ms += s.agent_time_ms;
            agg_user_ms += s.user_time_ms;
            agg_writes += s.write_count;
            agg_edits += s.edit_count;
            agg_bashes += s.bash_count;
            agg_files += s.files_touched_count;
        }
        let accounted = agg_agent_ms + agg_user_ms;
        serde_json::json!({
            "sessions": session_count,
            "agent_time_ms": agg_agent_ms,
            "user_time_ms": agg_user_ms,
            "agent_time_pct": if accounted > 0 { agg_agent_ms as f64 / accounted as f64 * 100.0 } else { 0.0 },
            "user_time_pct": if accounted > 0 { agg_user_ms as f64 / accounted as f64 * 100.0 } else { 0.0 },
            "write_count": agg_writes,
            "edit_count": agg_edits,
            "bash_count": agg_bashes,
            "files_touched_count": agg_files,
        })
    });

    let db_path = config.db_path().display().to_string();

    let mut status = serde_json::json!({
        "last_sync": last_sync,
        "last_sync_ago": last_sync_ago,
        "sessions_total": total,
        "sessions_processed": processed_count,
        "sessions_unprocessed": unprocessed,
        "sessions_skipped": skipped_count,
        "sessions_with_knowledge": knowledge_count,
        "vault_notes": vault_notes,
        "vault_size_kb": vault_size_kb,
        "state_file": db_path,
        "active_session_count": active_session_count,
        "active_sessions": active_sessions,
        "inactive_session_count": inactive_session_count,
        "inactive_sessions": inactive_sessions,
    });
    if let Some(agg) = aggregate_stats {
        status["aggregate_stats"] = agg;
    }

    println!("{}", serde_json::to_string_pretty(&status).unwrap());
    Ok(())
}
