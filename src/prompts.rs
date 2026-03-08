use serde::Serialize;
use std::path::Path;

use crate::session::ToolCall;

#[derive(Debug, Clone, Serialize)]
pub struct PromptFileInfo {
    pub file_path: String,
    pub session_id: String,
    pub project_cwd: String,
    pub written_at: String,
    pub content_preview: String,
    pub prompt_confidence: f64,
    pub file_exists: bool,
}

/// Extensions that immediately disqualify a file as a prompt.
const CODE_EXTENSIONS: &[&str] = &[
    ".rs", ".go", ".py", ".js", ".ts", ".tsx", ".jsx", ".sh", ".yaml", ".yml", ".json", ".toml",
    ".css", ".html", ".java", ".c", ".cpp", ".h", ".rb", ".php", ".swift", ".kt", ".lock",
];

/// Content prefixes that indicate code rather than a prompt.
const CODE_CONTENT_PREFIXES: &[&str] = &["#!/", "{"];

/// Compute a confidence score (0.0 - 1.0) that a Write tool call produced a prompt file.
pub fn prompt_confidence(tool_call: &ToolCall, project_cwd: &str) -> f64 {
    let file_path = match &tool_call.file_path {
        Some(fp) => fp,
        None => return 0.0,
    };

    // Only consider Write tool calls
    if tool_call.tool_name != "Write" {
        return 0.0;
    }

    let path_lower = file_path.to_lowercase();

    // Immediate disqualifiers: code file extensions
    for ext in CODE_EXTENSIONS {
        if path_lower.ends_with(ext) {
            return 0.0;
        }
    }

    // Disqualify if content starts with code markers
    if let Some(ref preview) = tool_call.content_preview {
        let trimmed = preview.trim_start();
        for prefix in CODE_CONTENT_PREFIXES {
            if trimmed.starts_with(prefix) {
                return 0.0;
            }
        }
    }

    let mut score: f64 = 0.0;

    // Extension signals
    if path_lower.ends_with(".md") || path_lower.ends_with(".txt") {
        score += 0.15;
    } else if path_lower.ends_with(".prompt") || path_lower.ends_with(".template") {
        score += 0.25;
    }

    // Filename signals
    let filename_lower = Path::new(file_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    let mut name_score: f64 = 0.0;
    if filename_lower.contains("prompt") {
        name_score += 0.20;
    }
    if filename_lower.contains("instruction") {
        name_score += 0.20;
    }
    if filename_lower.contains("agent") {
        name_score += 0.20;
    }
    score += name_score.min(0.40);

    // Content signals
    if let Some(ref preview) = tool_call.content_preview {
        let content_lower = preview.to_lowercase();
        let mut content_score: f64 = 0.0;
        if content_lower.contains("you are") {
            content_score += 0.15;
        }
        if content_lower.contains("your task") {
            content_score += 0.10;
        }
        if content_lower.contains("instructions:") {
            content_score += 0.10;
        }
        score += content_score.min(0.30);
    }

    // Written outside project_cwd
    if !project_cwd.is_empty() && !file_path.starts_with(project_cwd) {
        score += 0.10;
    }

    // Written to src/lib/tests directories (penalty)
    let rel_path = if file_path.starts_with(project_cwd) && !project_cwd.is_empty() {
        &file_path[project_cwd.len()..]
    } else {
        file_path.as_str()
    };
    let rel_lower = rel_path.to_lowercase();
    if rel_lower.starts_with("/src/")
        || rel_lower.starts_with("/lib/")
        || rel_lower.starts_with("/tests/")
    {
        score -= 0.15;
    }

    score.clamp(0.0, 1.0)
}

/// Detect prompt files from a collection of tool calls.
pub fn detect_prompt_files(
    tool_calls: &[ToolCall],
    session_id: &str,
    project_cwd: &str,
    threshold: f64,
) -> Vec<PromptFileInfo> {
    let mut results = Vec::new();

    for tc in tool_calls {
        if tc.tool_name != "Write" {
            continue;
        }

        let confidence = prompt_confidence(tc, project_cwd);
        if confidence < threshold {
            continue;
        }

        let file_path = match &tc.file_path {
            Some(fp) => fp.clone(),
            None => continue,
        };

        let file_exists = Path::new(&file_path).exists();

        results.push(PromptFileInfo {
            file_path,
            session_id: session_id.to_string(),
            project_cwd: project_cwd.to_string(),
            written_at: tc.timestamp.clone(),
            content_preview: tc.content_preview.clone().unwrap_or_default(),
            prompt_confidence: confidence,
            file_exists,
        });
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_write_call(file_path: &str, content: Option<&str>) -> ToolCall {
        ToolCall {
            tool_name: "Write".to_string(),
            file_path: Some(file_path.to_string()),
            content_preview: content.map(|s| s.to_string()),
            timestamp: "2025-01-15T10:00:00Z".to_string(),
        }
    }

    #[test]
    fn disqualifies_code_files() {
        let tc = make_write_call("/project/src/main.rs", Some("fn main() {}"));
        assert_eq!(prompt_confidence(&tc, "/project"), 0.0);

        let tc = make_write_call("/project/app.js", Some("console.log('hi')"));
        assert_eq!(prompt_confidence(&tc, "/project"), 0.0);
    }

    #[test]
    fn disqualifies_json_content() {
        let tc = make_write_call("/project/data.txt", Some("{ \"key\": \"value\" }"));
        assert_eq!(prompt_confidence(&tc, "/project"), 0.0);
    }

    #[test]
    fn high_confidence_for_prompt_files() {
        let tc = make_write_call(
            "/home/user/prompts/agent-prompt.md",
            Some("You are a helpful assistant. Your task is to..."),
        );
        let score = prompt_confidence(&tc, "/project");
        assert!(score >= 0.5, "expected >= 0.5, got {}", score);
    }

    #[test]
    fn low_confidence_for_plain_md() {
        let tc = make_write_call(
            "/project/README.md",
            Some("# My Project\n\nThis is a readme."),
        );
        let score = prompt_confidence(&tc, "/project");
        // Should be below threshold - just .md extension and in src path
        assert!(score < 0.5, "expected < 0.5, got {}", score);
    }

    #[test]
    fn non_write_tools_score_zero() {
        let tc = ToolCall {
            tool_name: "Edit".to_string(),
            file_path: Some("/prompts/agent.md".to_string()),
            content_preview: Some("You are a helpful assistant".to_string()),
            timestamp: "2025-01-15T10:00:00Z".to_string(),
        };
        assert_eq!(prompt_confidence(&tc, "/project"), 0.0);
    }

    #[test]
    fn detect_prompt_files_filters_by_threshold() {
        let tool_calls = vec![
            make_write_call("/project/src/main.rs", Some("fn main() {}")),
            make_write_call(
                "/home/user/prompts/agent.prompt",
                Some("You are a coding assistant. Your task is to help."),
            ),
            make_write_call("/project/notes.txt", Some("Just some notes")),
        ];

        let results = detect_prompt_files(&tool_calls, "session-1", "/project", 0.5);
        assert_eq!(results.len(), 1);
        assert!(results[0].file_path.contains("agent.prompt"));
    }
}
