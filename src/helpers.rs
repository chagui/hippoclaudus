use claude_pulse::config::Config;
use std::path::{Path, PathBuf};

pub fn session_id_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

pub fn file_mtime(path: &Path) -> f64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64()
        })
        .unwrap_or(0.0)
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        // Find a valid char boundary at or before max
        let mut end = max;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &s[..end])
    }
}

pub fn vault_size_kb(config: &Config) -> u64 {
    let vault_dir = config.vault_path();
    let mut total: u64 = 0;

    fn walk(dir: &PathBuf, total: &mut u64) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !name.starts_with('.') && name != "Templates" {
                        walk(&path, total);
                    }
                } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    *total += std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                }
            }
        }
    }

    walk(&vault_dir, &mut total);
    total / 1024
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    // ---- truncate proptest properties ----

    proptest! {
        #[test]
        fn truncate_output_length(s in "\\PC{0,500}", max in 0usize..200) {
            let result = truncate(&s, max);
            prop_assert!(result.len() <= max + 3,
                "result.len()={} > max+3={} for input len={}",
                result.len(), max + 3, s.len());
        }

        #[test]
        fn truncate_noop_for_short_strings(s in "\\PC{0,50}", extra in 0usize..50) {
            let max = s.chars().count() + extra;
            let result = truncate(&s, max);
            if s.len() <= max {
                prop_assert_eq!(&result, &s);
            }
        }

        #[test]
        fn truncate_valid_utf8(s in "\\PC{0,500}", max in 0usize..200) {
            let result = truncate(&s, max);
            for (i, _) in result.char_indices() {
                prop_assert!(result.is_char_boundary(i));
            }
        }

        #[test]
        fn truncate_ellipsis_iff_shortened(s in "\\PC{0,300}", max in 0usize..200) {
            let result = truncate(&s, max);
            if s.len() <= max {
                prop_assert_eq!(&result, &s, "short string should be unchanged");
            } else {
                prop_assert!(result.ends_with("..."),
                    "truncated string should end with '...': '{}'", result);
            }
        }
    }

    // ---- truncate unit tests ----

    #[test]
    fn truncate_multibyte_utf8() {
        let emoji = "🎉🎊🎈🎁";
        let result = truncate(emoji, 5);
        assert!(result.len() <= 8);
        assert!(result.ends_with("...") || result == emoji);

        let cjk = "你好世界";
        let result = truncate(cjk, 4);
        assert!(result.ends_with("...") || result == cjk);

        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_edge_cases() {
        assert_eq!(truncate("", 0), "");
        assert_eq!(truncate("", 10), "");
        assert_eq!(truncate("a", 0), "...");
        assert_eq!(truncate("abc", 2), "ab...");
    }

    // ---- session_id_from_path ----

    #[test]
    fn session_id_from_various_paths() {
        assert_eq!(
            session_id_from_path(Path::new("/foo/bar/abc123.jsonl")),
            "abc123"
        );
        assert_eq!(
            session_id_from_path(Path::new("/a/b/my-session.jsonl")),
            "my-session"
        );
        assert_eq!(session_id_from_path(Path::new("simple.jsonl")), "simple");
        assert_eq!(
            session_id_from_path(Path::new("/no-extension")),
            "no-extension"
        );
    }

    // ---- file_mtime ----

    #[test]
    fn file_mtime_nonexistent_returns_zero() {
        let result = file_mtime(Path::new("/nonexistent/path/file.txt"));
        assert_eq!(result, 0.0);
    }

    #[test]
    fn file_mtime_real_file_returns_positive() {
        let mut tmp = NamedTempFile::new().unwrap();
        write!(tmp, "hello").unwrap();
        let mtime = file_mtime(tmp.path());
        assert!(mtime > 0.0, "expected positive mtime, got {}", mtime);
    }

    // ---- vault_size_kb ----

    #[test]
    fn vault_size_kb_empty_dir() {
        let dir = TempDir::new().unwrap();
        let config = Config {
            vault_path: dir.path().to_str().unwrap().to_string(),
            claude_projects_path: "/tmp/unused".to_string(),
        };
        assert_eq!(vault_size_kb(&config), 0);
    }

    #[test]
    fn vault_size_kb_with_md_files() {
        let dir = TempDir::new().unwrap();
        let md_path = dir.path().join("note.md");
        std::fs::write(&md_path, vec![b'a'; 2048]).unwrap();

        let config = Config {
            vault_path: dir.path().to_str().unwrap().to_string(),
            claude_projects_path: "/tmp/unused".to_string(),
        };
        assert_eq!(vault_size_kb(&config), 2);
    }

    #[test]
    fn vault_size_kb_nonexistent_dir() {
        let config = Config {
            vault_path: "/nonexistent/vault/path".to_string(),
            claude_projects_path: "/tmp/unused".to_string(),
        };
        assert_eq!(vault_size_kb(&config), 0);
    }
}
