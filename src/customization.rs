//! Loads optional local customization files that override embedded arcade data.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

const DATA_DIR_ENV: &str = "PACMAN_DATA_DIR";
const REPO_NAME: &str = "pacman";

pub fn load_arcade_text(file_name: &str, default_text: &str) -> String {
    load_arcade_text_from_dir(&data_dir(), file_name, default_text)
}

fn data_dir() -> PathBuf {
    if let Some(path) = env::var_os(DATA_DIR_ENV) {
        return PathBuf::from(path);
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".xyzzy").join(REPO_NAME);
    }

    PathBuf::from(".xyzzy").join(REPO_NAME)
}

fn load_arcade_text_from_dir(dir: &Path, file_name: &str, default_text: &str) -> String {
    let override_path = dir.join(file_name);
    let override_text = match fs::read_to_string(&override_path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return default_text.to_string(),
        Err(error) => panic!(
            "failed to read override file {}: {error}",
            override_path.display()
        ),
    };

    if is_key_value_text(default_text) {
        merge_key_value_text(default_text, &override_text)
    } else {
        override_text
    }
}

fn is_key_value_text(text: &str) -> bool {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .all(|line| line.contains('='))
}

fn merge_key_value_text(default_text: &str, override_text: &str) -> String {
    let mut overrides = parse_key_value_lines(override_text);
    let mut merged_lines = Vec::new();

    for line in default_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            merged_lines.push(line.to_string());
            continue;
        }

        let (key, _) = parse_key_value_line(trimmed);
        if let Some(value) = overrides.remove(key) {
            merged_lines.push(format!("{key}={value}"));
        } else {
            merged_lines.push(trimmed.to_string());
        }
    }

    for (key, value) in overrides {
        merged_lines.push(format!("{key}={value}"));
    }

    format!("{}\n", merged_lines.join("\n"))
}

fn parse_key_value_lines(text: &str) -> BTreeMap<String, String> {
    let mut values = BTreeMap::new();

    for line in text.lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (key, value) = parse_key_value_line(line);
        values.insert(key.to_string(), value.to_string());
    }

    values
}

fn parse_key_value_line(line: &str) -> (&str, &str) {
    let (key, value) = line
        .split_once('=')
        .expect("customization lines should use key=value");
    (key.trim(), value.trim())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{load_arcade_text_from_dir, merge_key_value_text};

    static NEXT_DIR_ID: AtomicUsize = AtomicUsize::new(0);

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "pacman-customization-test-{}-{}",
                std::process::id(),
                NEXT_DIR_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).expect("create temp dir");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn key_value_override_replaces_only_listed_keys() {
        let merged = merge_key_value_text(
            "fruit_reset_timer=138\nfruit_release_dots=70,170\nglobal_release_dots=7,17,32\n",
            "fruit_release_dots=64,176\n",
        );

        assert!(merged.contains("fruit_reset_timer=138"));
        assert!(merged.contains("fruit_release_dots=64,176"));
        assert!(merged.contains("global_release_dots=7,17,32"));
    }

    #[test]
    fn layout_override_replaces_whole_text() {
        let temp_dir = TempDir::new();
        fs::write(temp_dir.path().join("maze-logic.txt"), "CUSTOM\nLAYOUT\n")
            .expect("write layout override");

        let loaded = load_arcade_text_from_dir(temp_dir.path(), "maze-logic.txt", "DEFAULT\n");

        assert_eq!(loaded, "CUSTOM\nLAYOUT\n");
    }
}
