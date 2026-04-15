use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use crate::game::GameEvent;

#[derive(Clone, Copy, Debug)]
enum SoundAsset {
    ButtonClick,
    Death,
    FreightMode,
    FruitEat,
    GhostEat,
    LevelComplete,
    LevelStart,
    Music,
    SmallPellet,
}

#[derive(Debug, Default)]
struct LoopingAudio {
    stop: Option<Arc<AtomicBool>>,
    child: Arc<Mutex<Option<Child>>>,
    thread: Option<JoinHandle<()>>,
}

#[derive(Debug, Default)]
pub struct AudioManager {
    enabled: bool,
    title_music: LoopingAudio,
    freight_sound: LoopingAudio,
}

impl SoundAsset {
    const fn file_name(self) -> &'static str {
        match self {
            Self::ButtonClick => "button_click.ogg",
            Self::Death => "death.wav",
            Self::FreightMode => "fright_mode_short.wav",
            Self::FruitEat => "fruit_eat.wav",
            Self::GhostEat => "ghost_eat.wav",
            Self::LevelComplete => "level_complete.wav",
            Self::LevelStart => "level_start.wav",
            Self::Music => "music.ogg",
            Self::SmallPellet => "small_pellet2.wav",
        }
    }

    const fn bytes(self) -> &'static [u8] {
        match self {
            Self::ButtonClick => include_bytes!("../assets/Sounds/button_click.ogg"),
            Self::Death => include_bytes!("../assets/Sounds/death.wav"),
            Self::FreightMode => include_bytes!("../assets/Sounds/fright_mode_short.wav"),
            Self::FruitEat => include_bytes!("../assets/Sounds/fruit_eat.wav"),
            Self::GhostEat => include_bytes!("../assets/Sounds/ghost_eat.wav"),
            Self::LevelComplete => include_bytes!("../assets/Sounds/level_complete.wav"),
            Self::LevelStart => include_bytes!("../assets/Sounds/level_start.wav"),
            Self::Music => include_bytes!("../assets/Sounds/music.ogg"),
            Self::SmallPellet => include_bytes!("../assets/Sounds/small_pellet2.wav"),
        }
    }
}

impl LoopingAudio {
    fn play(&mut self, path: PathBuf) {
        self.stop();

        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let child_slot = self.child.clone();
        self.thread = Some(thread::spawn(move || {
            while !stop_flag.load(Ordering::SeqCst) {
                let child = Command::new("/usr/bin/afplay")
                    .arg(&path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
                let Ok(child) = child else {
                    break;
                };

                {
                    let mut slot = child_slot
                        .lock()
                        .expect("audio child slot should not be poisoned");
                    *slot = Some(child);
                }

                let wait_result = {
                    let mut slot = child_slot
                        .lock()
                        .expect("audio child slot should not be poisoned");
                    slot.as_mut().expect("audio child should be present").wait()
                };

                {
                    let mut slot = child_slot
                        .lock()
                        .expect("audio child slot should not be poisoned");
                    *slot = None;
                }

                if wait_result.is_err() {
                    break;
                }
            }
        }));
        self.stop = Some(stop);
    }

    fn stop(&mut self) {
        if let Some(stop) = &self.stop {
            stop.store(true, Ordering::SeqCst);
        }

        if let Some(child) = self
            .child
            .lock()
            .expect("audio child slot should not be poisoned")
            .as_mut()
        {
            let _ = child.kill();
            let _ = child.wait();
        }

        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }

        self.stop = None;
    }
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            enabled: Path::new("/usr/bin/afplay").exists(),
            title_music: LoopingAudio::default(),
            freight_sound: LoopingAudio::default(),
        }
    }

    pub fn handle_event(&mut self, event: GameEvent) {
        if !self.enabled {
            return;
        }

        match event {
            GameEvent::TitleScreenEntered => {
                self.stop_freight_sound();
                self.play_title_music();
            }
            GameEvent::ButtonClicked => self.play_effect(SoundAsset::ButtonClick),
            GameEvent::GameStarted => {
                self.stop_title_music();
                self.stop_freight_sound();
                self.play_effect(SoundAsset::LevelStart);
            }
            GameEvent::SmallPelletEaten => self.play_effect(SoundAsset::SmallPellet),
            GameEvent::PowerPelletEaten => {}
            GameEvent::FreightModeStarted => self.play_freight_sound(),
            GameEvent::FreightModeEnded => self.stop_freight_sound(),
            GameEvent::GhostEaten => self.play_effect(SoundAsset::GhostEat),
            GameEvent::FruitEaten => self.play_effect(SoundAsset::FruitEat),
            GameEvent::PacmanDied => {
                self.stop_freight_sound();
                self.play_effect(SoundAsset::Death);
            }
            GameEvent::LevelCompleted => {
                self.stop_freight_sound();
                self.play_effect(SoundAsset::LevelComplete);
            }
        }
    }

    fn play_title_music(&mut self) {
        if let Some(path) = sound_path(SoundAsset::Music) {
            self.title_music.play(path);
        }
    }

    fn stop_title_music(&mut self) {
        self.title_music.stop();
    }

    fn play_freight_sound(&mut self) {
        if let Some(path) = sound_path(SoundAsset::FreightMode) {
            self.freight_sound.play(path);
        }
    }

    fn stop_freight_sound(&mut self) {
        self.freight_sound.stop();
    }

    fn play_effect(&self, sound: SoundAsset) {
        let Some(path) = sound_path(sound) else {
            return;
        };

        let _ = Command::new("/usr/bin/afplay")
            .arg(path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        self.stop_title_music();
        self.stop_freight_sound();
    }
}

fn sound_path(sound: SoundAsset) -> Option<PathBuf> {
    let path = cached_sound_path(sound);
    if ensure_embedded_sound(sound, &path).is_ok() {
        Some(path)
    } else {
        None
    }
}

fn cached_sound_path(sound: SoundAsset) -> PathBuf {
    env::temp_dir()
        .join("pacman")
        .join(env!("CARGO_PKG_VERSION"))
        .join("sounds")
        .join(sound.file_name())
}

fn ensure_embedded_sound(sound: SoundAsset, path: &Path) -> std::io::Result<()> {
    let bytes = sound.bytes();

    if path
        .metadata()
        .map(|meta| meta.len() == bytes.len() as u64)
        .unwrap_or(false)
    {
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{LoopingAudio, SoundAsset, cached_sound_path, ensure_embedded_sound, sound_path};

    fn all_assets() -> [SoundAsset; 9] {
        [
            SoundAsset::ButtonClick,
            SoundAsset::Death,
            SoundAsset::FreightMode,
            SoundAsset::FruitEat,
            SoundAsset::GhostEat,
            SoundAsset::LevelComplete,
            SoundAsset::LevelStart,
            SoundAsset::Music,
            SoundAsset::SmallPellet,
        ]
    }

    #[test]
    fn embedded_sound_cache_uses_the_temp_directory() {
        let path = cached_sound_path(SoundAsset::Music);

        assert!(path.ends_with("pacman/0.1.0/sounds/music.ogg"));
    }

    #[test]
    fn embedded_sound_files_can_be_materialized_for_playback() {
        let path = sound_path(SoundAsset::Music).expect("embedded music should materialize");

        assert!(path.exists());
        assert!(
            path.metadata()
                .expect("embedded sound path should be readable")
                .len()
                > 0
        );
    }

    #[test]
    fn materialized_sound_matches_embedded_length() {
        let path = cached_sound_path(SoundAsset::ButtonClick);
        ensure_embedded_sound(SoundAsset::ButtonClick, &path)
            .expect("embedded click sound should be written");

        assert_eq!(
            path.metadata()
                .expect("materialized sound should exist")
                .len(),
            SoundAsset::ButtonClick.bytes().len() as u64
        );
    }

    #[test]
    fn every_embedded_sound_asset_has_bytes_and_a_cache_path() {
        for sound in all_assets() {
            let path = cached_sound_path(sound);
            assert!(path.starts_with(std::env::temp_dir()));
            assert_eq!(
                path.file_name(),
                Some(std::ffi::OsStr::new(sound.file_name()))
            );
            assert!(
                !sound.bytes().is_empty(),
                "asset {} should not be empty",
                sound.file_name()
            );
        }
    }

    #[test]
    fn every_embedded_sound_asset_can_be_materialized() {
        for sound in all_assets() {
            let path = sound_path(sound).unwrap_or_else(|| PathBuf::from(sound.file_name()));
            assert!(
                path.exists(),
                "{} should be written to disk",
                sound.file_name()
            );
            assert_eq!(
                path.metadata()
                    .expect("materialized sound should exist")
                    .len(),
                sound.bytes().len() as u64
            );
        }
    }

    #[test]
    fn looping_audio_stop_is_safe_without_an_active_child() {
        let mut looping = LoopingAudio::default();
        looping.stop();
        assert!(looping.stop.is_none());
    }
}
