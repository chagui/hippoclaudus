use filetime::{set_file_mtime, FileTime};
use rusqlite::Connection;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

use hippoclaudus::config::Config;
use hippoclaudus::git_stats::compute_commit_ratio;
use hippoclaudus::session::ToolCall;
use hippoclaudus::state::SyncState;

// ---------------------------------------------------------------------------
// SyncState — inject an in-memory connection to avoid disk I/O and WAL threads
// ---------------------------------------------------------------------------

fn temp_state() -> SyncState {
    let conn = Connection::open_in_memory().expect("should open in-memory connection");
    SyncState::from_connection(conn).expect("should initialize schema")
}

#[test]
fn state_open_creates_db() {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let state = SyncState::open_at(&db_path).expect("should open DB");
    assert!(db_path.exists());
    let _sessions = state.processed_sessions().unwrap();
}

#[test]
fn state_mark_and_is_processed_roundtrip() {
    let state = temp_state();

    assert!(!state.is_processed("session-1", 1234.567));

    state
        .mark_processed("session-1", 1234.567, "test:roundtrip", 5000)
        .unwrap();

    assert!(state.is_processed("session-1", 1234.567));
}

#[test]
fn state_is_processed_false_for_different_mtime() {
    let state = temp_state();

    state
        .mark_processed("session-1", 1234.567, "test:mtime", 5000)
        .unwrap();

    assert!(!state.is_processed("session-1", 9999.999));
}

#[test]
fn state_processed_sessions_returns_marked() {
    let state = temp_state();

    state
        .mark_processed("id1", 100.0, "created:a.md", 1000)
        .unwrap();
    state
        .mark_processed("id2", 200.0, "no_new_knowledge", 500)
        .unwrap();
    state
        .mark_processed("id3", 300.0, "skipped:too_short", 100)
        .unwrap();

    let sessions = state.processed_sessions().unwrap();
    assert!(sessions.contains_key("id1"));
    assert!(sessions.contains_key("id2"));
    assert!(sessions.contains_key("id3"));

    assert_eq!(sessions["id1"].result, "created:a.md");
    assert_eq!(sessions["id2"].text_chars, 500);
}

#[test]
fn state_last_run_after_mark() {
    let state = temp_state();

    state
        .mark_processed("session-1", 100.0, "test:lastrun", 1000)
        .unwrap();

    let last = state.last_run();
    assert!(last.is_some());
}

// ---------------------------------------------------------------------------
// session_stats / prompt_files — new tables
// ---------------------------------------------------------------------------

#[test]
fn session_stats_upsert_and_get() {
    let state = temp_state();

    state
        .upsert_session_stats(
            "sess-1",
            60000,
            45000,
            15000,
            5,
            3,
            10,
            20,
            30,
            8,
            Some(75.5),
        )
        .unwrap();

    let row = state
        .get_session_stats("sess-1")
        .expect("should find stats");
    assert_eq!(row.session_id, "sess-1");
    assert_eq!(row.total_duration_ms, 60000);
    assert_eq!(row.agent_time_ms, 45000);
    assert_eq!(row.user_time_ms, 15000);
    assert_eq!(row.real_user_message_count, 5);
    assert_eq!(row.agent_turn_count, 3);
    assert_eq!(row.write_count, 10);
    assert_eq!(row.edit_count, 20);
    assert_eq!(row.bash_count, 30);
    assert_eq!(row.files_touched_count, 8);
    assert_eq!(row.commit_ratio_pct, Some(75.5));
}

#[test]
fn session_stats_get_all() {
    let state = temp_state();

    state
        .upsert_session_stats("s1", 1000, 800, 200, 2, 1, 1, 2, 3, 4, None)
        .unwrap();
    state
        .upsert_session_stats("s2", 2000, 1600, 400, 4, 2, 5, 6, 7, 8, Some(50.0))
        .unwrap();

    let all = state.get_all_session_stats(7).unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn session_stats_not_found() {
    let state = temp_state();
    assert!(state.get_session_stats("nonexistent").is_none());
}

#[test]
fn prompt_files_upsert_and_get() {
    let state = temp_state();

    state
        .upsert_prompt_file(
            "/home/user/prompts/agent.md",
            "sess-1",
            "/home/user/project",
            "2025-01-15T10:00:00Z",
            "You are a helpful assistant",
            0.85,
        )
        .unwrap();

    let files = state.get_prompt_files(None).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_path, "/home/user/prompts/agent.md");
    assert_eq!(files[0].session_id, "sess-1");
    assert_eq!(files[0].prompt_confidence, 0.85);
}

// ---------------------------------------------------------------------------
// discover_sessions / discover_active_sessions — using tempdir
// ---------------------------------------------------------------------------

#[test]
fn discover_sessions_empty_dir() {
    let dir = TempDir::new().unwrap();
    let config = Config {
        vault_path: "/tmp/vault".to_string(),
        claude_projects_path: dir.path().to_str().unwrap().to_string(),
    };
    let sessions = hippoclaudus::discover_sessions(&config, Some(7));
    assert!(sessions.is_empty());
}

#[test]
fn discover_active_sessions_finds_recent_jsonl() {
    let dir = TempDir::new().unwrap();
    let project_dir = dir.path().join("project1");
    std::fs::create_dir_all(&project_dir).unwrap();

    let file_path = project_dir.join("session1.jsonl");
    std::fs::write(
        &file_path,
        r#"{"type":"user","message":{"content":"hello"}}"#,
    )
    .unwrap();

    let config = Config {
        vault_path: "/tmp/vault".to_string(),
        claude_projects_path: dir.path().to_str().unwrap().to_string(),
    };

    let active = hippoclaudus::discover_active_sessions(&config);
    assert_eq!(active.len(), 1);
    assert!(active[0].to_str().unwrap().contains("session1.jsonl"));

    let completed = hippoclaudus::discover_sessions(&config, Some(7));
    assert!(completed.is_empty());
}

#[test]
fn discover_sessions_skips_non_jsonl() {
    let dir = TempDir::new().unwrap();
    let project_dir = dir.path().join("project1");
    std::fs::create_dir_all(&project_dir).unwrap();

    std::fs::write(project_dir.join("notes.txt"), "not a session").unwrap();
    std::fs::write(project_dir.join("config.json"), "{}").unwrap();

    let config = Config {
        vault_path: "/tmp/vault".to_string(),
        claude_projects_path: dir.path().to_str().unwrap().to_string(),
    };
    let sessions = hippoclaudus::discover_sessions(&config, Some(7));
    assert!(sessions.is_empty());
}

#[test]
fn discover_active_sessions_nonexistent_dir() {
    let config = Config {
        vault_path: "/tmp/vault".to_string(),
        claude_projects_path: "/nonexistent/path".to_string(),
    };
    let sessions = hippoclaudus::discover_active_sessions(&config);
    assert!(sessions.is_empty());
}

// ---------------------------------------------------------------------------
// discover_sessions — age filtering with old mtimes
// ---------------------------------------------------------------------------

#[test]
fn discover_sessions_finds_old_jsonl() {
    let dir = TempDir::new().unwrap();
    let project_dir = dir.path().join("project1");
    std::fs::create_dir_all(&project_dir).unwrap();

    let file_path = project_dir.join("old_session.jsonl");
    std::fs::write(&file_path, r#"{"type":"user","message":{"content":"hi"}}"#).unwrap();

    // Set mtime to 1 hour ago (past the 5-min active threshold)
    let one_hour_ago = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - 3600;
    set_file_mtime(&file_path, FileTime::from_unix_time(one_hour_ago as i64, 0)).unwrap();

    let config = Config {
        vault_path: "/tmp/vault".to_string(),
        claude_projects_path: dir.path().to_str().unwrap().to_string(),
    };

    let sessions = hippoclaudus::discover_sessions(&config, Some(7));
    assert_eq!(sessions.len(), 1);
    assert!(sessions[0].to_str().unwrap().contains("old_session.jsonl"));
}

#[test]
fn discover_sessions_filters_by_max_age() {
    let dir = TempDir::new().unwrap();
    let project_dir = dir.path().join("project1");
    std::fs::create_dir_all(&project_dir).unwrap();

    let file_path = project_dir.join("ancient.jsonl");
    std::fs::write(&file_path, r#"{"type":"user","message":{"content":"hi"}}"#).unwrap();

    // Set mtime to 30 days ago
    let thirty_days_ago = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - 30 * 24 * 3600;
    set_file_mtime(
        &file_path,
        FileTime::from_unix_time(thirty_days_ago as i64, 0),
    )
    .unwrap();

    let config = Config {
        vault_path: "/tmp/vault".to_string(),
        claude_projects_path: dir.path().to_str().unwrap().to_string(),
    };

    // Should be excluded with 7-day filter
    let sessions = hippoclaudus::discover_sessions(&config, Some(7));
    assert!(sessions.is_empty());

    // Should be included with no age filter
    let sessions = hippoclaudus::discover_sessions(&config, None);
    assert_eq!(sessions.len(), 1);
}

#[test]
fn discover_sessions_sorted_newest_first() {
    let dir = TempDir::new().unwrap();
    let project_dir = dir.path().join("project1");
    std::fs::create_dir_all(&project_dir).unwrap();

    let older = project_dir.join("older.jsonl");
    let newer = project_dir.join("newer.jsonl");
    std::fs::write(&older, "{}").unwrap();
    std::fs::write(&newer, "{}").unwrap();

    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    set_file_mtime(
        &older,
        FileTime::from_unix_time((now_secs - 7200) as i64, 0),
    )
    .unwrap();
    set_file_mtime(
        &newer,
        FileTime::from_unix_time((now_secs - 3600) as i64, 0),
    )
    .unwrap();

    let config = Config {
        vault_path: "/tmp/vault".to_string(),
        claude_projects_path: dir.path().to_str().unwrap().to_string(),
    };

    let sessions = hippoclaudus::discover_sessions(&config, Some(7));
    assert_eq!(sessions.len(), 2);
    assert!(sessions[0].to_str().unwrap().contains("newer.jsonl"));
    assert!(sessions[1].to_str().unwrap().contains("older.jsonl"));
}

// ---------------------------------------------------------------------------
// vault_note_count / file_mtime_secs
// ---------------------------------------------------------------------------

#[test]
fn vault_note_count_counts_md_files() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("note1.md"), "# Note 1").unwrap();
    std::fs::write(dir.path().join("note2.md"), "# Note 2").unwrap();
    std::fs::write(dir.path().join("other.txt"), "not a note").unwrap();

    // Nested directory
    let sub = dir.path().join("subdir");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("nested.md"), "# Nested").unwrap();

    let config = Config {
        vault_path: dir.path().to_str().unwrap().to_string(),
        claude_projects_path: "/tmp/projects".to_string(),
    };

    assert_eq!(hippoclaudus::vault_note_count(&config), 3);
}

#[test]
fn vault_note_count_skips_dotfiles_and_templates() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("visible.md"), "# Visible").unwrap();

    let dot_dir = dir.path().join(".hidden");
    std::fs::create_dir_all(&dot_dir).unwrap();
    std::fs::write(dot_dir.join("secret.md"), "# Hidden").unwrap();

    let templates = dir.path().join("Templates");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::write(templates.join("template.md"), "# Template").unwrap();

    let config = Config {
        vault_path: dir.path().to_str().unwrap().to_string(),
        claude_projects_path: "/tmp/projects".to_string(),
    };

    assert_eq!(hippoclaudus::vault_note_count(&config), 1);
}

#[test]
fn vault_note_count_nonexistent_vault() {
    let config = Config {
        vault_path: "/nonexistent/vault".to_string(),
        claude_projects_path: "/tmp/projects".to_string(),
    };
    assert_eq!(hippoclaudus::vault_note_count(&config), 0);
}

#[test]
fn file_mtime_secs_returns_positive_for_existing_file() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("test.txt");
    std::fs::write(&file, "hello").unwrap();

    let mtime = hippoclaudus::file_mtime_secs(&file);
    assert!(mtime > 0.0);
}

#[test]
fn file_mtime_secs_returns_zero_for_missing_file() {
    let missing = std::path::PathBuf::from("/nonexistent/file.txt");
    assert_eq!(hippoclaudus::file_mtime_secs(&missing), 0.0);
}

// ---------------------------------------------------------------------------
// Config — struct methods
// ---------------------------------------------------------------------------

#[test]
fn config_vault_path_expands_tilde() {
    let config = Config {
        vault_path: "~/Documents/Vault".to_string(),
        claude_projects_path: "~/.claude/projects".to_string(),
    };
    let vault = config.vault_path();
    assert!(!vault.to_str().unwrap().starts_with('~'));
    assert!(vault.to_str().unwrap().ends_with("Documents/Vault"));

    let projects = config.projects_path();
    assert!(!projects.to_str().unwrap().starts_with('~'));
    assert!(projects.to_str().unwrap().ends_with(".claude/projects"));
}

#[test]
fn config_vault_path_absolute_passthrough() {
    let config = Config {
        vault_path: "/absolute/path/vault".to_string(),
        claude_projects_path: "/absolute/projects".to_string(),
    };
    assert_eq!(
        config.vault_path().to_str().unwrap(),
        "/absolute/path/vault"
    );
    assert_eq!(
        config.projects_path().to_str().unwrap(),
        "/absolute/projects"
    );
}

// ---------------------------------------------------------------------------
// git_stats — compute_commit_ratio with a real temp git repo
// ---------------------------------------------------------------------------

#[test]
fn commit_ratio_with_real_git_repo() {
    let dir = TempDir::new().unwrap();
    let cwd = dir.path().to_str().unwrap();

    // Initialize a git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create and commit a file
    let committed_file = dir.path().join("committed.rs");
    std::fs::write(&committed_file, "fn main() {}").unwrap();
    std::process::Command::new("git")
        .args(["add", "committed.rs"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Create a file that was NOT committed
    let uncommitted_file = dir.path().join("uncommitted.rs");
    std::fs::write(&uncommitted_file, "fn other() {}").unwrap();

    // Tool calls referencing both files
    let tool_calls = vec![
        ToolCall {
            tool_name: "Write".to_string(),
            file_path: Some(committed_file.to_str().unwrap().to_string()),
            content_preview: None,
            timestamp: "2025-01-15T10:00:00Z".to_string(),
        },
        ToolCall {
            tool_name: "Write".to_string(),
            file_path: Some(uncommitted_file.to_str().unwrap().to_string()),
            content_preview: None,
            timestamp: "2025-01-15T10:01:00Z".to_string(),
        },
        // Non-Write/Edit tool calls should be ignored
        ToolCall {
            tool_name: "Bash".to_string(),
            file_path: Some("/some/path".to_string()),
            content_preview: None,
            timestamp: "2025-01-15T10:02:00Z".to_string(),
        },
    ];

    let result = compute_commit_ratio(&tool_calls, cwd, 7).expect("should compute ratio");
    assert_eq!(result.files_touched, 2);
    assert_eq!(result.files_committed, 1);
    assert!((result.ratio_pct - 50.0).abs() < f64::EPSILON);
}

#[test]
fn commit_ratio_skips_files_outside_repo() {
    let dir = TempDir::new().unwrap();
    let cwd = dir.path().to_str().unwrap();

    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Tool call with file outside the repo
    let tool_calls = vec![ToolCall {
        tool_name: "Write".to_string(),
        file_path: Some("/completely/different/path/file.rs".to_string()),
        content_preview: None,
        timestamp: "2025-01-15T10:00:00Z".to_string(),
    }];

    let result = compute_commit_ratio(&tool_calls, cwd, 7).expect("should compute ratio");
    assert_eq!(result.files_touched, 0);
}

#[test]
fn commit_ratio_relative_paths_counted() {
    let dir = TempDir::new().unwrap();
    let cwd = dir.path().to_str().unwrap();

    // Initialize repo with an initial commit so git log works
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::fs::write(dir.path().join("README.md"), "init").unwrap();
    std::process::Command::new("git")
        .args(["add", "."])
        .current_dir(dir.path())
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["commit", "-m", "init"])
        .current_dir(dir.path())
        .output()
        .unwrap();

    // Tool call with a relative path (not committed)
    let tool_calls = vec![ToolCall {
        tool_name: "Edit".to_string(),
        file_path: Some("src/lib.rs".to_string()),
        content_preview: None,
        timestamp: "2025-01-15T10:00:00Z".to_string(),
    }];

    let result = compute_commit_ratio(&tool_calls, cwd, 7).expect("should compute ratio");
    assert_eq!(result.files_touched, 1);
    assert_eq!(result.files_committed, 0);
}
