use hippoclaudus::session::{extract_session, extract_session_metadata};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn test_extract_simple_session() {
    let path = fixture("simple_session.jsonl");
    let session = extract_session(&path).expect("should parse simple session");

    assert_eq!(session.session_id, "test-session-1");
    assert_eq!(session.project_cwd, "/Users/dev/my-project");
    assert_eq!(session.git_branch, "main");
    assert_eq!(session.exchanges.len(), 2);
    assert!(session.total_text_chars > 0);

    assert!(session.exchanges[0]
        .user_message
        .contains("How do I write a Rust test?"));
    assert!(session.exchanges[0]
        .assistant_response
        .contains("#[test] attribute"));

    assert!(session.exchanges[1]
        .user_message
        .contains("Can you show me an example?"));
    assert!(session.exchanges[1]
        .assistant_response
        .contains("#[cfg(test)]"));
}

#[test]
fn test_extract_session_metadata() {
    let path = fixture("simple_session.jsonl");
    let info = extract_session_metadata(&path).expect("should parse metadata");

    assert_eq!(info.session_id, "test-session-1");
    assert_eq!(info.project_cwd, "/Users/dev/my-project");
    assert_eq!(info.git_branch, "main");
    assert_eq!(info.exchange_count, 2);
    assert!(info.model.contains("sonnet"));
    assert!(info.total_input_tokens > 0);
    assert!(info.total_output_tokens > 0);
}

#[test]
fn test_noise_filtering() {
    let path = fixture("noisy_session.jsonl");
    let session = extract_session(&path).expect("should parse noisy session");

    assert_eq!(session.exchanges.len(), 1);
    // The system-reminder line should be filtered
    assert!(!session.exchanges[0]
        .user_message
        .contains("<system-reminder>"));
    assert!(session.exchanges[0]
        .user_message
        .contains("Actual user message here"));
}

#[test]
fn test_extract_nonexistent_file() {
    let path = fixture("does_not_exist.jsonl");
    let result = extract_session(&path);
    assert!(result.is_err());
}

#[test]
fn test_config_tilde_expansion() {
    // Config tilde expansion uses $HOME
    let config = hippoclaudus::config::Config::load();
    let vault = config.vault_path();
    let projects = config.projects_path();

    // Expanded paths should not contain tildes
    assert!(!vault.to_str().unwrap().contains('~'));
    assert!(!projects.to_str().unwrap().contains('~'));
}

// ---------------------------------------------------------------------------
// Tool call extraction tests
// ---------------------------------------------------------------------------

#[test]
fn test_extract_tool_calls() {
    let path = fixture("session_with_tools.jsonl");
    let session = extract_session(&path).expect("should parse session with tools");

    assert_eq!(session.session_id, "test-tools-1");
    assert_eq!(session.git_branch, "feature/tools");

    // Should have 3 tool calls: Write, Bash, Edit
    assert_eq!(session.tool_calls.len(), 3);

    assert_eq!(session.tool_calls[0].tool_name, "Write");
    assert_eq!(
        session.tool_calls[0].file_path.as_deref(),
        Some("/Users/dev/my-project/src/main.rs")
    );
    assert!(session.tool_calls[0].content_preview.is_some());

    assert_eq!(session.tool_calls[1].tool_name, "Bash");
    assert!(session.tool_calls[1].file_path.is_none());
    assert_eq!(
        session.tool_calls[1].content_preview.as_deref(),
        Some("cargo run")
    );

    assert_eq!(session.tool_calls[2].tool_name, "Edit");
    assert_eq!(
        session.tool_calls[2].file_path.as_deref(),
        Some("/Users/dev/my-project/src/main.rs")
    );
}

#[test]
fn test_metadata_with_tool_counts() {
    let path = fixture("session_with_tools.jsonl");
    let info = extract_session_metadata(&path).expect("should parse metadata with tools");

    assert_eq!(info.session_id, "test-tools-1");
    assert_eq!(info.write_count, 1);
    assert_eq!(info.edit_count, 1);
    assert_eq!(info.bash_count, 1);
    assert_eq!(info.files_touched_count, 1); // both Write and Edit touch same file

    // exchange_count should only count real user messages (not tool results)
    // Real user messages: initial prompt + "Looks great, thanks!" = 2
    assert_eq!(info.exchange_count, 2);
}

#[test]
fn test_timing_stats() {
    let path = fixture("session_with_tools.jsonl");
    let info = extract_session_metadata(&path).expect("should parse metadata");

    let timing = info.timing.expect("should have timing stats");

    // agent_time_ms should be sum of turn_duration entries: 45000 + 5000 = 50000
    assert_eq!(timing.agent_time_ms, 50000);
    assert_eq!(timing.agent_turn_count, 2);

    // Real user messages: 2
    assert_eq!(timing.real_user_message_count, 2);

    // total_duration_ms should be from first to last timestamp
    assert!(timing.total_duration_ms > 0);

    // Percentages should be valid
    assert!(timing.agent_time_pct >= 0.0 && timing.agent_time_pct <= 100.0);
    assert!(timing.user_time_pct >= 0.0 && timing.user_time_pct <= 100.0);
}

#[test]
fn test_simple_session_has_no_tool_calls() {
    let path = fixture("simple_session.jsonl");
    let session = extract_session(&path).expect("should parse simple session");

    assert!(session.tool_calls.is_empty());
}

#[test]
fn test_simple_session_metadata_zero_tool_counts() {
    let path = fixture("simple_session.jsonl");
    let info = extract_session_metadata(&path).expect("should parse metadata");

    assert_eq!(info.write_count, 0);
    assert_eq!(info.edit_count, 0);
    assert_eq!(info.bash_count, 0);
    assert_eq!(info.files_touched_count, 0);
}
