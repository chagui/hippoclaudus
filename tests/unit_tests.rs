use rusqlite::Connection;
use tempfile::TempDir;

use claude_pulse::config::Config;
use claude_pulse::state::SyncState;

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
    let sessions = claude_pulse::discover_sessions(&config, Some(7));
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

    let active = claude_pulse::discover_active_sessions(&config);
    assert_eq!(active.len(), 1);
    assert!(active[0].to_str().unwrap().contains("session1.jsonl"));

    let completed = claude_pulse::discover_sessions(&config, Some(7));
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
    let sessions = claude_pulse::discover_sessions(&config, Some(7));
    assert!(sessions.is_empty());
}

#[test]
fn discover_active_sessions_nonexistent_dir() {
    let config = Config {
        vault_path: "/tmp/vault".to_string(),
        claude_projects_path: "/nonexistent/path".to_string(),
    };
    let sessions = claude_pulse::discover_active_sessions(&config);
    assert!(sessions.is_empty());
}
