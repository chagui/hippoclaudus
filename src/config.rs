use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::time::Duration;

const DEFAULT_ACTIVE_WINDOW_MINUTES: u32 = 5;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub vault_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vault_name: Option<String>,
    pub claude_projects_path: String,
    /// Minutes after which a session is considered inactive. Sessions modified
    /// within this window appear in `active_sessions`; older ones in
    /// `inactive_sessions`. Defaults to 5 when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_window_minutes: Option<u32>,
}

impl Config {
    /// Load config from `~/Library/Application Support/com.chagui.hippoclaudus/config.json`.
    /// Falls back to defaults if the file doesn't exist.
    /// Creates the config file with defaults on first run.
    /// Migrates from the old `com.chagui.hippoclaudus` location on first run.
    pub fn load() -> Self {
        migrate_from_old_bundle_id();
        let path = Self::config_file_path();
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                log::warn!("Failed to parse config file: {}", e);
                Self::defaults()
            }),
            Err(_) => {
                let config = Self::defaults();
                config.save();
                config
            }
        }
    }

    /// Save config to disk.
    fn save(&self) {
        let path = Self::config_file_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, &json);
            // Restrict config to owner-only (0600) since it may contain paths
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        }
    }

    /// Application Support directory for hippoclaudus.
    pub fn app_support_dir(&self) -> PathBuf {
        let home = home_dir();
        let dir = home.join("Library/Application Support/com.chagui.hippoclaudus");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// Resolved vault path (tilde-expanded).
    pub fn vault_path(&self) -> PathBuf {
        expand_tilde(&self.vault_path)
    }

    /// Duration a session is considered "active" after its last modification.
    /// Reads `active_window_minutes` from config, defaulting to 5 minutes.
    pub fn active_window(&self) -> Duration {
        let minutes = u64::from(
            self.active_window_minutes
                .unwrap_or(DEFAULT_ACTIVE_WINDOW_MINUTES),
        );
        Duration::from_secs(minutes * 60)
    }

    /// Obsidian vault name used in `obsidian://open?vault=...` URLs.
    /// Falls back to the last path component of `vault_path` when unset.
    pub fn vault_name(&self) -> String {
        if let Some(name) = self.vault_name.as_ref().filter(|n| !n.is_empty()) {
            return name.clone();
        }
        self.vault_path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string()
    }

    /// Resolved claude projects path (tilde-expanded).
    pub fn projects_path(&self) -> PathBuf {
        expand_tilde(&self.claude_projects_path)
    }

    /// Path to the SQLite state database.
    pub fn db_path(&self) -> PathBuf {
        self.app_support_dir().join("state.db")
    }

    fn config_file_path() -> PathBuf {
        let home = home_dir();
        home.join("Library/Application Support/com.chagui.hippoclaudus/config.json")
    }

    fn defaults() -> Self {
        Self {
            vault_path: "~/Documents/Obsidian/Vaults/Claude".to_string(),
            vault_name: None,
            claude_projects_path: "~/.claude/projects".to_string(),
            active_window_minutes: None,
        }
    }
}

/// Migrate Application Support and Logs from the old `com.chagui.claude-pulse` bundle ID
/// to the new `com.chagui.hippoclaudus` location. No-op if the old directory doesn't exist
/// or the new directory already exists.
fn migrate_from_old_bundle_id() {
    let home = home_dir();

    let migrations = [
        (
            home.join("Library/Application Support/com.chagui.claude-pulse"),
            home.join("Library/Application Support/com.chagui.hippoclaudus"),
        ),
        (
            home.join("Library/Logs/com.chagui.claude-pulse"),
            home.join("Library/Logs/com.chagui.hippoclaudus"),
        ),
    ];

    for (old, new) in &migrations {
        if old.exists() && !new.exists() {
            match fs::rename(old, new) {
                Ok(()) => log::info!("Migrated {} → {}", old.display(), new.display()),
                Err(e) => log::warn!(
                    "Failed to migrate {} → {}: {}",
                    old.display(),
                    new.display(),
                    e
                ),
            }
        }
    }
}

fn home_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("$HOME environment variable is not set"))
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        home_dir().join(rest)
    } else {
        PathBuf::from(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn expand_tilde_with_home(rest in "[a-zA-Z0-9._-]{1,30}") {
            let input = format!("~/{}", rest);
            let result = expand_tilde(&input);
            let result_str = result.to_str().unwrap();
            prop_assert!(!result_str.starts_with("~/"),
                "tilde not expanded: {}", result_str);
            let home = std::env::var("HOME").unwrap();
            prop_assert!(result_str.starts_with(&home),
                "result '{}' doesn't start with HOME '{}'", result_str, home);
            prop_assert!(result_str.ends_with(&rest),
                "result '{}' doesn't end with rest '{}'", result_str, rest);
        }

        #[test]
        fn expand_tilde_passthrough(path in "/[a-zA-Z0-9/._-]{0,50}") {
            let result = expand_tilde(&path);
            prop_assert_eq!(result.to_str().unwrap(), &path);
        }
    }

    #[test]
    fn vault_name_uses_configured_value() {
        let config = Config {
            vault_path: "/tmp/some/dir".to_string(),
            vault_name: Some("MyKnowledge".to_string()),
            claude_projects_path: "/tmp/projects".to_string(),
            active_window_minutes: None,
        };
        assert_eq!(config.vault_name(), "MyKnowledge");
    }

    #[test]
    fn vault_name_falls_back_to_path_component() {
        let config = Config {
            vault_path: "/tmp/some/dir/Claude".to_string(),
            vault_name: None,
            claude_projects_path: "/tmp/projects".to_string(),
            active_window_minutes: None,
        };
        assert_eq!(config.vault_name(), "Claude");
    }

    #[test]
    fn vault_name_empty_configured_falls_back() {
        let config = Config {
            vault_path: "/tmp/vaults/Work".to_string(),
            vault_name: Some(String::new()),
            claude_projects_path: "/tmp/projects".to_string(),
            active_window_minutes: None,
        };
        assert_eq!(config.vault_name(), "Work");
    }

    #[test]
    fn vault_name_missing_field_deserializes() {
        let json = r#"{"vault_path":"/tmp/v/Claude","claude_projects_path":"/tmp/p"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.vault_name.is_none());
        assert_eq!(config.vault_name(), "Claude");
    }

    #[test]
    fn active_window_defaults_to_five_minutes() {
        let config = Config {
            vault_path: "/tmp/v".to_string(),
            vault_name: None,
            claude_projects_path: "/tmp/p".to_string(),
            active_window_minutes: None,
        };
        assert_eq!(config.active_window(), Duration::from_secs(5 * 60));
    }

    #[test]
    fn active_window_uses_configured_value() {
        let config = Config {
            vault_path: "/tmp/v".to_string(),
            vault_name: None,
            claude_projects_path: "/tmp/p".to_string(),
            active_window_minutes: Some(30),
        };
        assert_eq!(config.active_window(), Duration::from_secs(30 * 60));
    }

    #[test]
    fn active_window_missing_field_deserializes() {
        let json = r#"{"vault_path":"/tmp/v","claude_projects_path":"/tmp/p"}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.active_window_minutes.is_none());
        assert_eq!(config.active_window(), Duration::from_secs(5 * 60));
    }
}
