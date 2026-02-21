use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

use crate::session::ToolCall;

#[derive(Debug, Clone)]
pub struct CommitRatio {
    pub files_touched: usize,
    pub files_committed: usize,
    pub ratio_pct: f64,
}

/// Compute the ratio of Claude-touched files that landed in git commits.
///
/// 1. Collect unique file paths from Write/Edit tool calls, normalize to repo-relative
/// 2. Run `git log --since --name-only` to get committed files
/// 3. Intersect and compute ratio
pub fn compute_commit_ratio(
    tool_calls: &[ToolCall],
    project_cwd: &str,
    since_days: u32,
) -> Option<CommitRatio> {
    if project_cwd.is_empty() {
        return None;
    }

    let cwd = Path::new(project_cwd);

    // Check if this is a git repo
    let git_check = Command::new("git")
        .args(["-C", project_cwd, "rev-parse", "--git-dir"])
        .output();

    match git_check {
        Ok(output) if output.status.success() => {}
        _ => return None, // Not a git repo or git not available
    }

    // Collect unique files touched by Write/Edit
    let mut touched: HashSet<String> = HashSet::new();
    for tc in tool_calls {
        if tc.tool_name != "Write" && tc.tool_name != "Edit" {
            continue;
        }
        if let Some(ref fp) = tc.file_path {
            // Normalize to repo-relative path
            let abs_path = Path::new(fp);
            if let Ok(rel) = abs_path.strip_prefix(cwd) {
                touched.insert(rel.to_string_lossy().to_string());
            } else if !fp.starts_with('/') {
                // Already relative
                touched.insert(fp.clone());
            }
            // Skip files outside the repo
        }
    }

    if touched.is_empty() {
        return Some(CommitRatio {
            files_touched: 0,
            files_committed: 0,
            ratio_pct: 0.0,
        });
    }

    // Get committed files since N days ago
    let since_arg = format!("{}days ago", since_days);
    let git_log = Command::new("git")
        .args([
            "-C",
            project_cwd,
            "log",
            &format!("--since={}", since_arg),
            "--name-only",
            "--pretty=format:",
        ])
        .output();

    let committed_files: HashSet<String> = match git_log {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.trim().to_string())
                .collect()
        }
        _ => return None,
    };

    let files_committed = touched.intersection(&committed_files).count();
    let files_touched = touched.len();

    let ratio_pct = if files_touched > 0 {
        (files_committed as f64 / files_touched as f64) * 100.0
    } else {
        0.0
    };

    Some(CommitRatio {
        files_touched,
        files_committed,
        ratio_pct,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_ratio_empty_tool_calls() {
        let result = compute_commit_ratio(&[], "/tmp/nonexistent-repo", 7);
        // Either None (no git repo) or Some with 0 touched
        if let Some(r) = result {
            assert_eq!(r.files_touched, 0);
        }
    }

    #[test]
    fn commit_ratio_non_git_dir() {
        let tc = ToolCall {
            tool_name: "Write".to_string(),
            file_path: Some("/tmp/test/file.txt".to_string()),
            content_preview: None,
            timestamp: "2025-01-15T10:00:00Z".to_string(),
        };
        let result = compute_commit_ratio(&[tc], "/tmp/definitely-not-a-git-repo", 7);
        assert!(result.is_none());
    }

    #[test]
    fn commit_ratio_empty_cwd() {
        let result = compute_commit_ratio(&[], "", 7);
        assert!(result.is_none());
    }
}
