use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub vault_path: String,
    pub claude_projects_path: String,
}

impl Config {
    /// Load config from `~/Library/Application Support/com.chagui.claude-pulse/config.json`.
    /// Falls back to defaults if the file doesn't exist.
    /// Creates the config file with defaults on first run.
    pub fn load() -> Self {
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

    /// Application Support directory for claude-pulse.
    pub fn app_support_dir(&self) -> PathBuf {
        let home = home_dir();
        let dir = home.join("Library/Application Support/com.chagui.claude-pulse");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// Resolved vault path (tilde-expanded).
    pub fn vault_path(&self) -> PathBuf {
        expand_tilde(&self.vault_path)
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
        home.join("Library/Application Support/com.chagui.claude-pulse/config.json")
    }

    fn defaults() -> Self {
        Self {
            vault_path: "~/Documents/Obsidian/Vaults/Claude".to_string(),
            claude_projects_path: "~/.claude/projects".to_string(),
        }
    }
}

fn home_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("HOME").expect("$HOME environment variable is not set"),
    )
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
}
