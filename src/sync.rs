use crate::prompt::build_prompt;
use claude_pulse::config::Config;
use claude_pulse::session::ExtractedSession;
use std::process::Command;

pub struct SyncResult {
    pub created_files: Vec<String>,
    pub updated_files: Vec<String>,
    pub no_new_knowledge: bool,
    #[allow(dead_code)]
    pub claude_output: String,
}

pub fn invoke_claude(
    session: &ExtractedSession,
    dry_run: bool,
    model: &str,
    config: &Config,
) -> Result<SyncResult, String> {
    let prompt = build_prompt(session, dry_run, config);
    let vault_dir = config.vault_path();

    let allowed_tools = if dry_run {
        "Read,Glob,Grep"
    } else {
        "Read,Write,Edit,Glob,Grep"
    };

    let output = Command::new("claude")
        .args([
            "-p",
            &prompt,
            "--model",
            model,
            "--output-format",
            "text",
            "--allowedTools",
            allowed_tools,
        ])
        .current_dir(&vault_dir)
        .env_remove("CLAUDE_CODE_ENTRYPOINT")
        .output()
        .map_err(|e| format!("Failed to invoke claude: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "claude exited with status {}: {}",
            output.status, stderr
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let no_new_knowledge = stdout.contains("NO_NEW_KNOWLEDGE");

    let mut created_files = Vec::new();
    let mut updated_files = Vec::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(file) = trimmed.strip_prefix("CREATED:") {
            created_files.push(file.trim().to_string());
        } else if let Some(file) = trimmed.strip_prefix("UPDATED:") {
            updated_files.push(file.trim().to_string());
        }
    }

    Ok(SyncResult {
        created_files,
        updated_files,
        no_new_knowledge,
        claude_output: stdout,
    })
}
