pub mod config;
pub mod git_stats;
pub mod prompts;
pub mod session;
pub mod state;

use config::Config;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Discover all JSONL session files across all projects.
///
/// Filters:
/// - Skip files modified < 5 min ago (still active)
/// - Skip files older than `days` days (if specified)
/// - Return sorted by mtime (newest first)
pub fn discover_sessions(config: &Config, days: Option<u32>) -> Vec<PathBuf> {
    let projects_dir = config.projects_path();

    let now = SystemTime::now();
    let five_minutes = Duration::from_secs(5 * 60);
    let max_age = days.map(|d| Duration::from_secs(d as u64 * 24 * 3600));

    let mut sessions: Vec<(PathBuf, SystemTime)> = Vec::new();

    // Walk ~/.claude/projects/*/*.jsonl
    let project_dirs = match fs::read_dir(&projects_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    for project_entry in project_dirs.flatten() {
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }

        let jsonl_files = match fs::read_dir(&project_path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for file_entry in jsonl_files.flatten() {
            let file_path = file_entry.path();

            // Only .jsonl files
            if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            let metadata = match fs::metadata(&file_path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let mtime = match metadata.modified() {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Skip files modified < 5 min ago (still active)
            if let Ok(age) = now.duration_since(mtime) {
                if age < five_minutes {
                    continue;
                }
            }

            // Skip files older than max_age
            if let Some(max) = max_age {
                if let Ok(age) = now.duration_since(mtime) {
                    if age > max {
                        continue;
                    }
                }
            }

            sessions.push((file_path, mtime));
        }
    }

    // Sort by mtime, newest first
    sessions.sort_by_key(|s| std::cmp::Reverse(s.1));

    sessions.into_iter().map(|(path, _)| path).collect()
}

/// Discover active JSONL session files (modified < 5 min ago).
///
/// This is the inverse of `discover_sessions()` — it returns files that are
/// likely still being written to by an active Claude Code session.
pub fn discover_active_sessions(config: &Config) -> Vec<PathBuf> {
    let projects_dir = config.projects_path();

    let now = SystemTime::now();
    let five_minutes = Duration::from_secs(5 * 60);

    let mut sessions: Vec<(PathBuf, SystemTime)> = Vec::new();

    let project_dirs = match fs::read_dir(&projects_dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    for project_entry in project_dirs.flatten() {
        let project_path = project_entry.path();
        if !project_path.is_dir() {
            continue;
        }

        let jsonl_files = match fs::read_dir(&project_path) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for file_entry in jsonl_files.flatten() {
            let file_path = file_entry.path();

            if file_path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }

            let metadata = match fs::metadata(&file_path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let mtime = match metadata.modified() {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Only files modified < 5 min ago (active)
            if let Ok(age) = now.duration_since(mtime) {
                if age >= five_minutes {
                    continue;
                }
            } else {
                continue;
            }

            sessions.push((file_path, mtime));
        }
    }

    // Sort by mtime, newest first
    sessions.sort_by_key(|s| std::cmp::Reverse(s.1));

    sessions.into_iter().map(|(path, _)| path).collect()
}

/// Count markdown files in the vault (excluding dotfiles and Templates/).
pub fn vault_note_count(config: &Config) -> usize {
    let vault_dir = config.vault_path();
    if !vault_dir.exists() {
        log::warn!("vault_path does not exist: {}", vault_dir.display());
        return 0;
    }
    count_md_files(&vault_dir)
}

fn count_md_files(dir: &PathBuf) -> usize {
    let mut count = 0;
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return 0,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip dotfiles and Templates
        if name.starts_with('.') || name == "Templates" {
            continue;
        }

        if path.is_dir() {
            count += count_md_files(&path);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            count += 1;
        }
    }

    count
}

/// Get the mtime of a file as seconds since UNIX epoch.
pub fn file_mtime_secs(path: &PathBuf) -> f64 {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| {
            t.duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64()
        })
        .unwrap_or(0.0)
}
