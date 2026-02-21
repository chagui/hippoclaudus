use anyhow::Result;

use claude_pulse::config::Config;
use claude_pulse::git_stats::compute_commit_ratio;
use claude_pulse::session::{extract_session, extract_session_metadata};
use claude_pulse::state::SyncState;
use claude_pulse::{discover_active_sessions, discover_sessions};

use crate::helpers::session_id_from_path;

pub fn cmd_stats(
    config: &Config,
    days: u32,
    session_id: Option<&str>,
    commit_ratio: bool,
    json: bool,
) -> Result<()> {
    let state = SyncState::open(config)?;

    if let Some(sid) = session_id {
        return print_session_stats(&state, config, sid, commit_ratio, json);
    }

    // Aggregate mode: collect stats from all sessions in the time range
    let all_paths = discover_sessions(config, Some(days));
    let active_paths = discover_active_sessions(config);

    let mut total_sessions: usize = 0;
    let mut total_duration_ms: u64 = 0;
    let mut total_agent_time_ms: u64 = 0;
    let mut total_user_time_ms: u64 = 0;
    let mut total_write_count: usize = 0;
    let mut total_edit_count: usize = 0;
    let mut total_bash_count: usize = 0;
    let mut total_files_touched: usize = 0;
    let mut total_agent_turns: usize = 0;
    let mut total_user_messages: usize = 0;
    let mut commit_files_touched: usize = 0;
    let mut commit_files_committed: usize = 0;

    let all_session_paths: Vec<_> = all_paths.iter().chain(active_paths.iter()).collect();

    for path in &all_session_paths {
        let info = match extract_session_metadata(path) {
            Ok(i) => i,
            Err(_) => continue,
        };

        total_sessions += 1;
        total_write_count += info.write_count;
        total_edit_count += info.edit_count;
        total_bash_count += info.bash_count;
        total_files_touched += info.files_touched_count;

        if let Some(ref timing) = info.timing {
            total_duration_ms += timing.total_duration_ms;
            total_agent_time_ms += timing.agent_time_ms;
            total_user_time_ms += timing.user_time_ms;
            total_agent_turns += timing.agent_turn_count;
            total_user_messages += timing.real_user_message_count;
        }

        // Store stats in DB for future queries
        if let Some(ref timing) = info.timing {
            let _ = state.upsert_session_stats(
                &info.session_id,
                timing.total_duration_ms,
                timing.agent_time_ms,
                timing.user_time_ms,
                timing.real_user_message_count,
                timing.agent_turn_count,
                info.write_count,
                info.edit_count,
                info.bash_count,
                info.files_touched_count,
                None,
            );
        }

        // Compute commit ratio if requested
        if commit_ratio {
            let session = match extract_session(path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            if let Some(cr) = compute_commit_ratio(&session.tool_calls, &info.project_cwd, days) {
                commit_files_touched += cr.files_touched;
                commit_files_committed += cr.files_committed;

                // Update DB with commit ratio
                if let Some(ref timing) = info.timing {
                    let _ = state.upsert_session_stats(
                        &info.session_id,
                        timing.total_duration_ms,
                        timing.agent_time_ms,
                        timing.user_time_ms,
                        timing.real_user_message_count,
                        timing.agent_turn_count,
                        info.write_count,
                        info.edit_count,
                        info.bash_count,
                        info.files_touched_count,
                        Some(cr.ratio_pct),
                    );
                }
            }
        }
    }

    if json {
        print_aggregate_json(
            days,
            total_sessions,
            total_duration_ms,
            total_agent_time_ms,
            total_user_time_ms,
            total_agent_turns,
            total_user_messages,
            total_write_count,
            total_edit_count,
            total_bash_count,
            total_files_touched,
            commit_ratio,
            commit_files_touched,
            commit_files_committed,
        );
    } else {
        print_aggregate_text(
            days,
            total_sessions,
            total_duration_ms,
            total_agent_time_ms,
            total_user_time_ms,
            total_agent_turns,
            total_user_messages,
            total_write_count,
            total_edit_count,
            total_bash_count,
            total_files_touched,
            commit_ratio,
            commit_files_touched,
            commit_files_committed,
        );
    }

    Ok(())
}

fn print_session_stats(
    state: &SyncState,
    config: &Config,
    session_id: &str,
    commit_ratio: bool,
    json: bool,
) -> Result<()> {
    // Try DB cache first
    if let Some(stats) = state.get_session_stats(session_id) {
        if json {
            println!("{}", serde_json::to_string_pretty(&stats).unwrap());
        } else {
            println!("=== Session: {} ===", stats.session_id);
            println!(
                "Duration: {} | Agent: {} ({:.1}%) | User: {} ({:.1}%)",
                format_duration_ms(stats.total_duration_ms),
                format_duration_ms(stats.agent_time_ms),
                if stats.agent_time_ms + stats.user_time_ms > 0 {
                    stats.agent_time_ms as f64 / (stats.agent_time_ms + stats.user_time_ms) as f64
                        * 100.0
                } else {
                    0.0
                },
                format_duration_ms(stats.user_time_ms),
                if stats.agent_time_ms + stats.user_time_ms > 0 {
                    stats.user_time_ms as f64 / (stats.agent_time_ms + stats.user_time_ms) as f64
                        * 100.0
                } else {
                    0.0
                },
            );
            println!(
                "Tools: {} Write, {} Edit, {} Bash | Files: {} unique",
                stats.write_count, stats.edit_count, stats.bash_count, stats.files_touched_count
            );
            if let Some(ratio) = stats.commit_ratio_pct {
                println!("Commit ratio: {:.1}%", ratio);
            }
        }
        return Ok(());
    }

    // Fall back to live parsing: search for the session file
    let all_paths = discover_sessions(config, None);
    let active_paths = discover_active_sessions(config);
    let all: Vec<_> = all_paths.iter().chain(active_paths.iter()).collect();

    let path = all
        .iter()
        .find(|p| session_id_from_path(p) == session_id)
        .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?;

    let info = extract_session_metadata(path).map_err(|e| anyhow::anyhow!(e))?;

    let mut commit_ratio_pct: Option<f64> = None;
    if commit_ratio {
        let session = extract_session(path).map_err(|e| anyhow::anyhow!(e))?;
        if let Some(cr) = compute_commit_ratio(&session.tool_calls, &info.project_cwd, 365) {
            commit_ratio_pct = Some(cr.ratio_pct);
        }
    }

    if json {
        let mut obj = serde_json::json!({
            "session_id": info.session_id,
            "project_cwd": info.project_cwd,
            "write_count": info.write_count,
            "edit_count": info.edit_count,
            "bash_count": info.bash_count,
            "files_touched_count": info.files_touched_count,
        });
        if let Some(ref timing) = info.timing {
            obj["total_duration_ms"] = serde_json::json!(timing.total_duration_ms);
            obj["agent_time_ms"] = serde_json::json!(timing.agent_time_ms);
            obj["user_time_ms"] = serde_json::json!(timing.user_time_ms);
            obj["agent_time_pct"] = serde_json::json!(timing.agent_time_pct);
            obj["user_time_pct"] = serde_json::json!(timing.user_time_pct);
        }
        if let Some(ratio) = commit_ratio_pct {
            obj["commit_ratio_pct"] = serde_json::json!(ratio);
        }
        println!("{}", serde_json::to_string_pretty(&obj).unwrap());
    } else {
        println!("=== Session: {} ===", info.session_id);
        println!("Project: {}", info.project_cwd);
        if let Some(ref timing) = info.timing {
            println!(
                "Duration: {} | Agent: {} ({:.1}%) | User: {} ({:.1}%)",
                format_duration_ms(timing.total_duration_ms),
                format_duration_ms(timing.agent_time_ms),
                timing.agent_time_pct,
                format_duration_ms(timing.user_time_ms),
                timing.user_time_pct,
            );
        }
        println!(
            "Tools: {} Write, {} Edit, {} Bash | Files: {} unique",
            info.write_count, info.edit_count, info.bash_count, info.files_touched_count
        );
        if let Some(ratio) = commit_ratio_pct {
            println!("Commit ratio: {:.1}%", ratio);
        }
    }

    Ok(())
}

fn print_aggregate_text(
    days: u32,
    sessions: usize,
    duration_ms: u64,
    agent_ms: u64,
    user_ms: u64,
    agent_turns: usize,
    user_messages: usize,
    writes: usize,
    edits: usize,
    bashes: usize,
    files: usize,
    show_commit: bool,
    commit_touched: usize,
    commit_committed: usize,
) {
    let accounted = agent_ms + user_ms;
    let agent_pct = if accounted > 0 {
        agent_ms as f64 / accounted as f64 * 100.0
    } else {
        0.0
    };
    let user_pct = if accounted > 0 {
        user_ms as f64 / accounted as f64 * 100.0
    } else {
        0.0
    };

    let avg_turn = if agent_turns > 0 {
        agent_ms / agent_turns as u64
    } else {
        0
    };
    let avg_user = if user_messages > 1 {
        user_ms / (user_messages - 1) as u64
    } else {
        0
    };

    println!("=== Session Statistics (last {} days) ===", days);
    println!(
        "Sessions: {} | Total: {}",
        sessions,
        format_duration_ms(duration_ms)
    );
    println!(
        "Agent time: {} ({:.1}%) | User time: {} ({:.1}%)",
        format_duration_ms(agent_ms),
        agent_pct,
        format_duration_ms(user_ms),
        user_pct,
    );
    println!(
        "Avg turn: {} | Avg user interaction: {}",
        format_duration_ms(avg_turn),
        format_duration_ms(avg_user),
    );
    println!(
        "Tools: {} Write, {} Edit, {} Bash | Files: {} unique",
        writes, edits, bashes, files
    );
    if show_commit && commit_touched > 0 {
        let ratio = commit_committed as f64 / commit_touched as f64 * 100.0;
        println!(
            "Commit ratio: {}/{} = {:.1}%",
            commit_committed, commit_touched, ratio
        );
    }
}

fn print_aggregate_json(
    days: u32,
    sessions: usize,
    duration_ms: u64,
    agent_ms: u64,
    user_ms: u64,
    agent_turns: usize,
    user_messages: usize,
    writes: usize,
    edits: usize,
    bashes: usize,
    files: usize,
    show_commit: bool,
    commit_touched: usize,
    commit_committed: usize,
) {
    let accounted = agent_ms + user_ms;
    let agent_pct = if accounted > 0 {
        agent_ms as f64 / accounted as f64 * 100.0
    } else {
        0.0
    };
    let user_pct = if accounted > 0 {
        user_ms as f64 / accounted as f64 * 100.0
    } else {
        0.0
    };

    let mut obj = serde_json::json!({
        "days": days,
        "sessions": sessions,
        "total_duration_ms": duration_ms,
        "agent_time_ms": agent_ms,
        "user_time_ms": user_ms,
        "agent_time_pct": agent_pct,
        "user_time_pct": user_pct,
        "agent_turn_count": agent_turns,
        "real_user_message_count": user_messages,
        "write_count": writes,
        "edit_count": edits,
        "bash_count": bashes,
        "files_touched_count": files,
    });

    if show_commit && commit_touched > 0 {
        let ratio = commit_committed as f64 / commit_touched as f64 * 100.0;
        obj["commit_ratio_pct"] = serde_json::json!(ratio);
        obj["commit_files_touched"] = serde_json::json!(commit_touched);
        obj["commit_files_committed"] = serde_json::json!(commit_committed);
    }

    println!("{}", serde_json::to_string_pretty(&obj).unwrap());
}

fn format_duration_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}
