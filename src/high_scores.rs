use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HighScoreTable {
    top_score: u32,
}

impl HighScoreTable {
    pub fn load_default() -> Self {
        Self::load(&default_storage_path()).unwrap_or_default()
    }

    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(text) => Ok(Self::parse(&text).unwrap_or_default()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    pub fn save_default(&self) -> io::Result<()> {
        self.save(&default_storage_path())
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, self.serialize())
    }

    pub fn top_score(&self) -> u32 {
        self.top_score
    }

    pub fn record(&mut self, score: u32) -> bool {
        if score > self.top_score {
            self.top_score = score;
            true
        } else {
            false
        }
    }

    fn parse(text: &str) -> Option<Self> {
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            return trimmed
                .parse::<u32>()
                .ok()
                .map(|top_score| Self { top_score });
        }

        None
    }

    fn serialize(&self) -> String {
        format!("{}\n", self.top_score)
    }
}

pub fn default_storage_path() -> PathBuf {
    if let Some(path) = env::var_os("PACMAN_DATA_DIR") {
        return PathBuf::from(path).join("high_scores.txt");
    }

    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".xyzzy")
            .join("pacman")
            .join("high_scores.txt");
    }

    PathBuf::from(".xyzzy")
        .join("pacman")
        .join("high_scores.txt")
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{HighScoreTable, default_storage_path};

    static NEXT_DIR_ID: AtomicUsize = AtomicUsize::new(0);

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "pacman-high-score-test-{}-{}",
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
    fn record_keeps_the_highest_score() {
        let mut table = HighScoreTable::default();

        assert!(table.record(12_340));
        assert_eq!(table.top_score(), 12_340);
        assert!(!table.record(8_000));
        assert_eq!(table.top_score(), 12_340);
    }

    #[test]
    fn save_and_load_round_trip_the_top_score() {
        let dir = TempDir::new();
        let path = dir.path().join("high_scores.txt");
        let mut table = HighScoreTable::default();
        table.record(23_450);

        table.save(&path).expect("save high score");
        let loaded = HighScoreTable::load(&path).expect("load high score");

        assert_eq!(loaded.top_score(), 23_450);
    }

    #[test]
    fn load_defaults_when_file_is_missing() {
        let dir = TempDir::new();
        let path = dir.path().join("missing.txt");

        let loaded = HighScoreTable::load(&path).expect("missing file should default");

        assert_eq!(loaded, HighScoreTable::default());
    }

    #[test]
    fn default_storage_path_uses_override_or_home() {
        let original_override = std::env::var_os("PACMAN_DATA_DIR");
        let original_home = std::env::var_os("HOME");

        let override_path = std::env::temp_dir().join("pacman-score-override");
        // SAFETY: This test sets process environment variables and restores them before exit.
        unsafe {
            std::env::set_var("PACMAN_DATA_DIR", &override_path);
        }
        let path = default_storage_path();
        assert_eq!(path, override_path.join("high_scores.txt"));

        // SAFETY: This test sets process environment variables and restores them before exit.
        unsafe {
            std::env::remove_var("PACMAN_DATA_DIR");
            std::env::set_var("HOME", "/tmp/pacman-home");
        }
        let path = default_storage_path();
        assert_eq!(
            path,
            PathBuf::from("/tmp/pacman-home/.xyzzy/pacman/high_scores.txt")
        );

        // SAFETY: This test restores process environment variables modified above.
        unsafe {
            if let Some(value) = original_override {
                std::env::set_var("PACMAN_DATA_DIR", value);
            } else {
                std::env::remove_var("PACMAN_DATA_DIR");
            }
            if let Some(value) = original_home {
                std::env::set_var("HOME", value);
            } else {
                std::env::remove_var("HOME");
            }
        }
    }
}
