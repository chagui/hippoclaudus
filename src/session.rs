use serde::Serialize;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct ExtractedSession {
    pub session_id: String,
    pub project_cwd: String,
    pub git_branch: String,
    pub first_timestamp: String,
    pub last_timestamp: String,
    pub exchanges: Vec<Exchange>,
    pub total_text_chars: usize,
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Exchange {
    pub user_message: String,
    pub assistant_response: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub file_path: Option<String>,
    pub content_preview: Option<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionTimingStats {
    pub total_duration_ms: u64,
    pub agent_time_ms: u64,
    pub user_time_ms: u64,
    pub real_user_message_count: usize,
    pub agent_turn_count: usize,
    pub avg_agent_turn_ms: u64,
    pub avg_user_interaction_ms: u64,
    pub agent_time_pct: f64,
    pub user_time_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActiveSessionInfo {
    pub session_id: String,
    pub project_name: String,
    pub project_cwd: String,
    pub git_branch: String,
    pub started_at: String,
    pub exchange_count: usize,
    pub model: String,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub total_cache_creation_tokens: u64,
    pub avg_turn_duration_ms: u64,
    pub turn_count: usize,
    /// "active" (agent working) or "waiting" (prompted user)
    pub state: String,
    pub timing: Option<SessionTimingStats>,
    pub write_count: usize,
    pub edit_count: usize,
    pub bash_count: usize,
    pub files_touched_count: usize,
    /// Canonical repo root when session is inside a Claude Code worktree.
    pub worktree_root: Option<String>,
}

/// Detect if `cwd` is inside a Claude Code worktree (`.claude/worktrees/<name>/`).
/// Returns the canonical repo root (the directory containing `.claude/`).
pub fn detect_worktree_root(cwd: &str) -> Option<String> {
    let marker = "/.claude/worktrees/";
    let idx = cwd.find(marker)?;
    // The repo root is everything before `/.claude/`
    let root = &cwd[..idx];
    if root.is_empty() {
        None
    } else {
        Some(root.to_string())
    }
}

/// Extract the worktree name from a Claude Code worktree path.
/// E.g. "/path/to/repo/.claude/worktrees/feature-a" → "feature-a"
pub fn detect_worktree_label(cwd: &str) -> Option<String> {
    let marker = "/.claude/worktrees/";
    let idx = cwd.find(marker)?;
    let after = &cwd[idx + marker.len()..];
    // Take the first path component after the marker
    let label = after.split('/').next().unwrap_or("");
    if label.is_empty() {
        None
    } else {
        Some(label.to_string())
    }
}

/// Extract last 2 path components as a project name (e.g. "chagui/knowledge-sync").
fn project_name_from_cwd(cwd: &str) -> String {
    let path = PathBuf::from(cwd);
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    let len = components.len();
    if len >= 2 {
        format!("{}/{}", components[len - 2], components[len - 1])
    } else if len == 1 {
        components[0].to_string()
    } else {
        String::new()
    }
}

/// Check if a user entry represents a real user message (not a tool result).
/// Tool results have a `sourceToolAssistantUUID` field.
fn is_real_user_message(entry: &serde_json::Value) -> bool {
    let entry_type = entry.get("type").and_then(|t| t.as_str()).unwrap_or("");
    if entry_type != "user" {
        return false;
    }
    entry.get("sourceToolAssistantUUID").is_none()
}

/// Extract tool calls from an assistant entry's message.content array.
/// Finds `type: "tool_use"` blocks and extracts tool name, file_path, and content preview.
fn extract_tool_calls(entry: &serde_json::Value, timestamp: &str) -> Vec<ToolCall> {
    let content = match entry.get("message").and_then(|m| m.get("content")) {
        Some(c) => c,
        None => return Vec::new(),
    };

    let blocks = match content.as_array() {
        Some(b) => b,
        None => return Vec::new(),
    };

    let mut calls = Vec::new();
    for block in blocks {
        let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if block_type != "tool_use" {
            continue;
        }

        let tool_name = block
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();

        if tool_name.is_empty() {
            continue;
        }

        let input = block.get("input");

        let file_path = input
            .and_then(|i| i.get("file_path"))
            .and_then(|f| f.as_str())
            .map(|s| s.to_string());

        let content_preview = match tool_name.as_str() {
            "Write" => input
                .and_then(|i| i.get("content"))
                .and_then(|c| c.as_str())
                .map(|s| truncate_str(s, 500)),
            "Edit" => input
                .and_then(|i| i.get("new_string"))
                .and_then(|c| c.as_str())
                .map(|s| truncate_str(s, 200)),
            "Bash" => input
                .and_then(|i| i.get("command"))
                .and_then(|c| c.as_str())
                .map(|s| truncate_str(s, 200)),
            _ => None,
        };

        calls.push(ToolCall {
            tool_name,
            file_path,
            content_preview,
            timestamp: timestamp.to_string(),
        });
    }

    calls
}

/// Truncate a string to max bytes on a char boundary.
fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s[..end].to_string()
}

/// Lightweight metadata extraction for active sessions.
///
/// Parses the first user entry for session metadata, assistant entries for token
/// usage and model, and system entries for turn duration. Tracks the last entry
/// type to determine session state (active vs waiting for user).
/// Also collects tool counts, files touched, and timing data.
pub fn extract_session_metadata(path: &Path) -> Result<ActiveSessionInfo, String> {
    let file = File::open(path).map_err(|e| format!("Cannot open {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);

    let mut session_id = String::new();
    let mut project_cwd = String::new();
    let mut git_branch = String::new();
    let mut started_at = String::new();
    let mut exchange_count: usize = 0;
    let mut found_first_user = false;

    let mut model = String::new();
    let mut total_input_tokens: u64 = 0;
    let mut total_output_tokens: u64 = 0;
    let mut total_cache_read_tokens: u64 = 0;
    let mut total_cache_creation_tokens: u64 = 0;
    let mut total_turn_duration_ms: u64 = 0;
    let mut turn_count: usize = 0;

    // Tool counts
    let mut write_count: usize = 0;
    let mut edit_count: usize = 0;
    let mut bash_count: usize = 0;
    let mut files_touched: HashSet<String> = HashSet::new();

    // Timing data
    let mut first_timestamp_str = String::new();
    let mut last_timestamp_str = String::new();
    // (turn_end_timestamp_str, duration_ms)
    let mut turn_end_events: Vec<(String, u64)> = Vec::new();
    let mut real_user_timestamps: Vec<String> = Vec::new();

    // Track last entry type for state detection
    // "assistant" → waiting, anything else → active
    let mut last_entry_type = String::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.trim().is_empty() {
            continue;
        }

        // Fast string-contains pre-filter: only parse lines we care about
        let is_user = line.contains("\"type\":\"user\"") || line.contains("\"type\": \"user\"");
        let is_assistant =
            line.contains("\"type\":\"assistant\"") || line.contains("\"type\": \"assistant\"");
        let is_system =
            line.contains("\"type\":\"system\"") || line.contains("\"type\": \"system\"");

        if !is_user && !is_assistant && !is_system {
            // progress, queue-operation, file-history-snapshot — skip parsing
            // but still track for state detection
            if line.contains("\"type\":\"progress\"") || line.contains("\"type\": \"progress\"") {
                last_entry_type = "progress".to_string();
            } else if line.contains("\"type\":\"queue-operation\"")
                || line.contains("\"type\": \"queue-operation\"")
            {
                // User queued input, so the agent will be processing soon
                last_entry_type = "queue-operation".to_string();
            }
            continue;
        }

        if is_user {
            let entry: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let entry_type = entry.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if entry_type != "user" {
                continue;
            }

            if let Some(ts) = entry.get("timestamp").and_then(|v| v.as_str()) {
                if first_timestamp_str.is_empty() {
                    first_timestamp_str = ts.to_string();
                }
                last_timestamp_str = ts.to_string();
            }

            let is_real = is_real_user_message(&entry);

            if !found_first_user {
                if let Some(sid) = entry.get("sessionId").and_then(|v| v.as_str()) {
                    session_id = sid.to_string();
                }
                if let Some(cwd) = entry.get("cwd").and_then(|v| v.as_str()) {
                    project_cwd = cwd.to_string();
                }
                if let Some(branch) = entry.get("gitBranch").and_then(|v| v.as_str()) {
                    git_branch = branch.to_string();
                }
                if let Some(ts) = entry.get("timestamp").and_then(|v| v.as_str()) {
                    started_at = ts.to_string();
                }
                found_first_user = true;
                // Only count real user messages for exchange_count
                if is_real {
                    exchange_count = 1;
                    real_user_timestamps.push(started_at.clone());
                }
            } else if is_real {
                exchange_count += 1;
                if let Some(ts) = entry.get("timestamp").and_then(|v| v.as_str()) {
                    real_user_timestamps.push(ts.to_string());
                }
            }

            last_entry_type = "user".to_string();
        } else if is_assistant {
            // Parse assistant entries for token usage, model, and tool calls
            let entry: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let timestamp = entry
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !timestamp.is_empty() {
                if first_timestamp_str.is_empty() {
                    first_timestamp_str = timestamp.to_string();
                }
                last_timestamp_str = timestamp.to_string();
            }

            if let Some(msg) = entry.get("message") {
                if let Some(m) = msg.get("model").and_then(|v| v.as_str()) {
                    model = m.to_string();
                }
                if let Some(usage) = msg.get("usage") {
                    total_input_tokens +=
                        usage.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    total_output_tokens +=
                        usage.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    total_cache_read_tokens += usage
                        .get("cache_read_input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    total_cache_creation_tokens += usage
                        .get("cache_creation_input_tokens")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                }
            }

            // Extract tool calls
            let tool_calls = extract_tool_calls(&entry, timestamp);
            for tc in &tool_calls {
                match tc.tool_name.as_str() {
                    "Write" => write_count += 1,
                    "Edit" => edit_count += 1,
                    "Bash" => bash_count += 1,
                    _ => {}
                }
                if let Some(ref fp) = tc.file_path {
                    files_touched.insert(fp.clone());
                }
            }

            last_entry_type = "assistant".to_string();
        } else if is_system {
            // Parse system entries for turn duration
            let entry: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };

            if let Some(ts) = entry.get("timestamp").and_then(|v| v.as_str()) {
                if first_timestamp_str.is_empty() {
                    first_timestamp_str = ts.to_string();
                }
                last_timestamp_str = ts.to_string();
            }

            if entry.get("subtype").and_then(|v| v.as_str()) == Some("turn_duration") {
                if let Some(dur) = entry.get("durationMs").and_then(|v| v.as_u64()) {
                    total_turn_duration_ms += dur;
                    turn_count += 1;
                    if let Some(ts) = entry.get("timestamp").and_then(|v| v.as_str()) {
                        turn_end_events.push((ts.to_string(), dur));
                    }
                }
            }

            last_entry_type = "system".to_string();
        }
    }

    if !found_first_user {
        return Err(format!("No user entry found in {}", path.display()));
    }

    if session_id.is_empty() {
        session_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
    }

    let worktree_root = detect_worktree_root(&project_cwd);
    let project_name = if let Some(ref root) = worktree_root {
        project_name_from_cwd(root)
    } else {
        project_name_from_cwd(&project_cwd)
    };

    let avg_turn_duration_ms = if turn_count > 0 {
        total_turn_duration_ms / turn_count as u64
    } else {
        0
    };

    // Determine session state:
    // - "assistant" as last entry → agent finished, waiting for user
    // - "queue-operation" → user queued input, agent about to process
    // - anything else (progress, system, user) → agent actively working
    let state = if last_entry_type == "assistant" {
        "waiting".to_string()
    } else {
        "active".to_string()
    };

    let timing = compute_timing_stats(
        &first_timestamp_str,
        &last_timestamp_str,
        total_turn_duration_ms,
        turn_count,
        &turn_end_events,
        &real_user_timestamps,
    );

    Ok(ActiveSessionInfo {
        session_id,
        project_name,
        project_cwd,
        git_branch,
        started_at,
        exchange_count,
        model,
        total_input_tokens,
        total_output_tokens,
        total_cache_read_tokens,
        total_cache_creation_tokens,
        avg_turn_duration_ms,
        turn_count,
        state,
        timing,
        write_count,
        edit_count,
        bash_count,
        files_touched_count: files_touched.len(),
        worktree_root,
    })
}

/// Compute timing statistics from collected session data.
///
/// - `agent_time_ms`: sum of all turn_duration durationMs values
/// - `user_time_ms`: gaps between turn end and next real user message (capped at 30 min per gap)
/// - Percentages based on accounted time (agent + user), not wall clock
fn compute_timing_stats(
    first_timestamp: &str,
    last_timestamp: &str,
    agent_time_ms: u64,
    agent_turn_count: usize,
    turn_end_events: &[(String, u64)],
    real_user_timestamps: &[String],
) -> Option<SessionTimingStats> {
    use chrono::DateTime;

    let first_dt = first_timestamp.parse::<DateTime<chrono::Utc>>().ok()?;
    let last_dt = last_timestamp.parse::<DateTime<chrono::Utc>>().ok()?;

    let total_duration_ms = (last_dt - first_dt).num_milliseconds().max(0) as u64;

    // Parse real user timestamps into DateTime for comparison
    let real_user_dts: Vec<DateTime<chrono::Utc>> = real_user_timestamps
        .iter()
        .filter_map(|ts| ts.parse::<DateTime<chrono::Utc>>().ok())
        .collect();

    // Compute user_time_ms: for each turn end, find the next real user message
    // and sum the gap (cap at 30 minutes to exclude AFK)
    let max_gap_ms: u64 = 30 * 60 * 1000; // 30 minutes
    let mut user_time_ms: u64 = 0;

    for (turn_end_ts, _dur) in turn_end_events {
        if let Ok(turn_end_dt) = turn_end_ts.parse::<DateTime<chrono::Utc>>() {
            // Find next real user message after this turn end
            if let Some(next_user_dt) = real_user_dts.iter().find(|&&dt| dt > turn_end_dt) {
                let gap = (*next_user_dt - turn_end_dt).num_milliseconds().max(0) as u64;
                if gap <= max_gap_ms {
                    user_time_ms += gap;
                }
            }
        }
    }

    let real_user_message_count = real_user_dts.len();

    let avg_agent_turn_ms = if agent_turn_count > 0 {
        agent_time_ms / agent_turn_count as u64
    } else {
        0
    };

    let user_interactions = if real_user_message_count > 1 {
        real_user_message_count - 1
    } else {
        0
    };
    let avg_user_interaction_ms = if user_interactions > 0 {
        user_time_ms / user_interactions as u64
    } else {
        0
    };

    let accounted = agent_time_ms + user_time_ms;
    let (agent_time_pct, user_time_pct) = if accounted > 0 {
        (
            (agent_time_ms as f64 / accounted as f64) * 100.0,
            (user_time_ms as f64 / accounted as f64) * 100.0,
        )
    } else {
        (0.0, 0.0)
    };

    Some(SessionTimingStats {
        total_duration_ms,
        agent_time_ms,
        user_time_ms,
        real_user_message_count,
        agent_turn_count,
        avg_agent_turn_ms,
        avg_user_interaction_ms,
        agent_time_pct,
        user_time_pct,
    })
}

/// Noise patterns to filter from extracted text.
const NOISE_PATTERNS: &[&str] = &[
    "<local-command-caveat>",
    "<command-name>",
    "[Request interrupted",
    "<local-command-stdout>",
    "<system-reminder>",
    "<user-prompt-submit-hook>",
];

/// Parse a JSONL session file and extract meaningful conversation text.
pub fn extract_session(path: &Path) -> Result<ExtractedSession, String> {
    let file = File::open(path).map_err(|e| format!("Cannot open {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);

    let mut session_id = String::new();
    let mut project_cwd = String::new();
    let mut git_branch = String::new();
    let mut first_timestamp = String::new();
    let mut last_timestamp = String::new();

    let mut pending_user_text: Option<String> = None;
    let mut exchanges = Vec::new();
    let mut total_text_chars: usize = 0;
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        if line.trim().is_empty() {
            continue;
        }

        let entry: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let entry_type = entry.get("type").and_then(|t| t.as_str()).unwrap_or("");

        // Skip non-conversation entries
        if entry_type == "file-history-snapshot" || entry_type == "progress" {
            continue;
        }

        // Capture metadata from first user entry
        if entry_type == "user" && session_id.is_empty() {
            if let Some(sid) = entry.get("sessionId").and_then(|v| v.as_str()) {
                session_id = sid.to_string();
            }
            if let Some(cwd) = entry.get("cwd").and_then(|v| v.as_str()) {
                project_cwd = cwd.to_string();
            }
            if let Some(branch) = entry.get("gitBranch").and_then(|v| v.as_str()) {
                git_branch = branch.to_string();
            }
        }

        // Track timestamps
        if let Some(ts) = entry.get("timestamp").and_then(|v| v.as_str()) {
            if first_timestamp.is_empty() {
                first_timestamp = ts.to_string();
            }
            last_timestamp = ts.to_string();
        }

        if entry_type == "user" {
            let text = extract_user_text(&entry);
            let text = filter_noise(&text);
            if !text.trim().is_empty() {
                // If we had a pending user message without an assistant response,
                // store it as an exchange with empty response
                if let Some(prev_user) = pending_user_text.take() {
                    total_text_chars += prev_user.len();
                    exchanges.push(Exchange {
                        user_message: prev_user,
                        assistant_response: String::new(),
                    });
                }
                pending_user_text = Some(text);
            }
        } else if entry_type == "assistant" {
            let ts = entry
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            tool_calls.extend(extract_tool_calls(&entry, ts));

            let text = extract_assistant_text(&entry);
            let text = filter_noise(&text);
            if !text.trim().is_empty() {
                if let Some(user_text) = pending_user_text.take() {
                    total_text_chars += user_text.len() + text.len();
                    exchanges.push(Exchange {
                        user_message: user_text,
                        assistant_response: text,
                    });
                } else {
                    // Assistant text without preceding user message — append to last exchange
                    if let Some(last) = exchanges.last_mut() {
                        last.assistant_response.push('\n');
                        last.assistant_response.push_str(&text);
                        total_text_chars += text.len();
                    }
                }
            }
        }
    }

    // Flush any remaining pending user message
    if let Some(user_text) = pending_user_text.take() {
        total_text_chars += user_text.len();
        exchanges.push(Exchange {
            user_message: user_text,
            assistant_response: String::new(),
        });
    }

    // Use filename as session_id fallback
    if session_id.is_empty() {
        session_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
    }

    Ok(ExtractedSession {
        session_id,
        project_cwd,
        git_branch,
        first_timestamp,
        last_timestamp,
        exchanges,
        total_text_chars,
        tool_calls,
    })
}

/// Extract text from a user entry's message.content.
/// Content can be a plain string or an array of content blocks.
fn extract_user_text(entry: &serde_json::Value) -> String {
    let content = match entry.get("message").and_then(|m| m.get("content")) {
        Some(c) => c,
        None => return String::new(),
    };

    // Plain string content
    if let Some(s) = content.as_str() {
        return s.to_string();
    }

    // Array of content blocks
    if let Some(blocks) = content.as_array() {
        let mut parts = Vec::new();
        for block in blocks {
            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if block_type == "text" {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    parts.push(text.to_string());
                }
            }
        }
        return parts.join("\n");
    }

    String::new()
}

/// Extract text from an assistant entry's message.content array.
/// Only extract type: "text" blocks (skip "thinking", "tool_use").
fn extract_assistant_text(entry: &serde_json::Value) -> String {
    let content = match entry.get("message").and_then(|m| m.get("content")) {
        Some(c) => c,
        None => return String::new(),
    };

    if let Some(blocks) = content.as_array() {
        let mut parts = Vec::new();
        for block in blocks {
            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if block_type == "text" {
                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                    let trimmed = text.trim();
                    if !trimmed.is_empty() {
                        parts.push(trimmed.to_string());
                    }
                }
            }
            // Skip "thinking", "tool_use", "tool_result", etc.
        }
        return parts.join("\n");
    }

    // Plain string fallback
    if let Some(s) = content.as_str() {
        return s.to_string();
    }

    String::new()
}

/// Filter noise patterns from extracted text.
fn filter_noise(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for line in text.lines() {
        if !NOISE_PATTERNS.iter().any(|pattern| line.contains(pattern)) {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
        }
    }
    result
}

/// Read the last N bytes of a file and extract state information.
/// Used as a fast path for large session files to determine current state
/// without parsing the entire file.
pub fn extract_session_state_tail(path: &Path, tail_bytes: u64) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    let file_size = metadata.len();

    if file_size > tail_bytes {
        file.seek(SeekFrom::End(-(tail_bytes as i64))).ok()?;
    }

    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;

    // Find the last complete line that has a type field
    let mut last_entry_type = String::new();

    for line in buf.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.contains("\"type\":\"assistant\"") || line.contains("\"type\": \"assistant\"") {
            last_entry_type = "assistant".to_string();
            break;
        } else if line.contains("\"type\":\"user\"") || line.contains("\"type\": \"user\"") {
            last_entry_type = "user".to_string();
            break;
        } else if line.contains("\"type\":\"progress\"") || line.contains("\"type\": \"progress\"")
        {
            last_entry_type = "progress".to_string();
            break;
        } else if line.contains("\"type\":\"system\"") || line.contains("\"type\": \"system\"") {
            last_entry_type = "system".to_string();
            break;
        } else if line.contains("\"type\":\"queue-operation\"")
            || line.contains("\"type\": \"queue-operation\"")
        {
            last_entry_type = "queue-operation".to_string();
            break;
        }
    }

    if last_entry_type.is_empty() {
        return None;
    }

    let state = if last_entry_type == "assistant" {
        "waiting"
    } else {
        "active"
    };

    Some(state.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ---- filter_noise ----

    proptest! {
        #[test]
        fn filter_noise_idempotent(s in "\\PC{0,500}") {
            let once = filter_noise(&s);
            let twice = filter_noise(&once);
            prop_assert_eq!(&once, &twice);
        }

        #[test]
        fn filter_noise_output_shorter(s in "\\PC{0,500}") {
            let result = filter_noise(&s);
            prop_assert!(result.len() <= s.len(),
                "result.len()={} > input.len()={}", result.len(), s.len());
        }

        #[test]
        fn filter_noise_no_patterns_in_output(s in "\\PC{0,500}") {
            let result = filter_noise(&s);
            for line in result.lines() {
                for pattern in NOISE_PATTERNS {
                    prop_assert!(!line.contains(*pattern),
                        "output line contains noise pattern '{}': '{}'", pattern, line);
                }
            }
        }
    }

    // ---- project_name_from_cwd ----

    proptest! {
        #[test]
        fn project_name_has_slash_for_deep_paths(
            a in "[a-zA-Z0-9_-]{1,20}",
            b in "[a-zA-Z0-9_-]{1,20}",
            c in "[a-zA-Z0-9_-]{1,20}"
        ) {
            let path = format!("/{}/{}/{}", a, b, c);
            let result = project_name_from_cwd(&path);
            prop_assert!(result.contains('/'),
                "expected '/' in result '{}' for path '{}'", result, path);
        }

        #[test]
        fn project_name_no_trailing_slash(
            a in "[a-zA-Z0-9_-]{1,10}",
            b in "[a-zA-Z0-9_-]{1,10}",
            c in "[a-zA-Z0-9_-]{1,10}"
        ) {
            let path = format!("/{}/{}/{}", a, b, c);
            let result = project_name_from_cwd(&path);
            prop_assert!(!result.ends_with('/'),
                "result ends with '/': '{}' for path '{}'", result, path);
            let expected = format!("{}/{}", b, c);
            prop_assert_eq!(&result, &expected);
        }

        #[test]
        fn project_name_empty_input(_dummy in Just(())) {
            let result = project_name_from_cwd("");
            prop_assert_eq!(result, "");
        }
    }

    // ---- extract_user_text ----

    proptest! {
        #[test]
        fn extract_user_text_null(_dummy in Just(())) {
            let result = extract_user_text(&serde_json::Value::Null);
            prop_assert_eq!(result, "");
        }

        #[test]
        fn extract_user_text_no_panic(
            key in "[a-z]{1,5}",
            val in "[a-zA-Z0-9 ]{0,50}"
        ) {
            let json = serde_json::json!({key: val});
            let _ = extract_user_text(&json);
        }
    }

    // ---- extract_assistant_text ----

    proptest! {
        #[test]
        fn extract_assistant_text_null(_dummy in Just(())) {
            let result = extract_assistant_text(&serde_json::Value::Null);
            prop_assert_eq!(result, "");
        }

        #[test]
        fn extract_assistant_text_no_panic(
            key in "[a-z]{1,5}",
            val in "[a-zA-Z0-9 ]{0,50}"
        ) {
            let json = serde_json::json!({key: val});
            let _ = extract_assistant_text(&json);
        }
    }

    // ---- detect_worktree_root ----

    #[test]
    fn worktree_root_claude_code_default() {
        let cwd = "/Users/me/Repos/org/myproject/.claude/worktrees/feature-a";
        assert_eq!(
            detect_worktree_root(cwd),
            Some("/Users/me/Repos/org/myproject".to_string())
        );
    }

    #[test]
    fn worktree_root_nested_worktree_path() {
        let cwd = "/home/dev/code/.claude/worktrees/hotfix/subdir";
        assert_eq!(
            detect_worktree_root(cwd),
            Some("/home/dev/code".to_string())
        );
    }

    #[test]
    fn worktree_root_not_a_worktree() {
        let cwd = "/Users/me/Repos/org/myproject";
        assert_eq!(detect_worktree_root(cwd), None);
    }

    #[test]
    fn worktree_root_claude_dir_but_not_worktrees() {
        let cwd = "/Users/me/Repos/org/myproject/.claude/settings";
        assert_eq!(detect_worktree_root(cwd), None);
    }

    // ---- detect_worktree_label ----

    #[test]
    fn worktree_label_simple() {
        let cwd = "/Users/me/Repos/org/myproject/.claude/worktrees/feature-a";
        assert_eq!(
            detect_worktree_label(cwd),
            Some("feature-a".to_string())
        );
    }

    #[test]
    fn worktree_label_with_subdir() {
        let cwd = "/home/dev/code/.claude/worktrees/hotfix/some/subdir";
        assert_eq!(
            detect_worktree_label(cwd),
            Some("hotfix".to_string())
        );
    }

    #[test]
    fn worktree_label_not_a_worktree() {
        let cwd = "/Users/me/Repos/org/myproject";
        assert_eq!(detect_worktree_label(cwd), None);
    }

    #[test]
    fn worktree_label_trailing_slash_only() {
        // Marker is present but name is empty (path ends at worktrees/)
        let cwd = "/Users/me/Repos/org/myproject/.claude/worktrees/";
        assert_eq!(detect_worktree_label(cwd), None);
    }

    // ---- project_name with worktree ----

    #[test]
    fn project_name_uses_root_for_worktree() {
        let worktree_cwd = "/Users/me/Repos/org/myproject/.claude/worktrees/feature-a";
        let root = detect_worktree_root(worktree_cwd).unwrap();
        let name = project_name_from_cwd(&root);
        assert_eq!(name, "org/myproject");
    }
}
