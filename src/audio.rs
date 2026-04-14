use std::{
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use crate::game::GameEvent;

#[derive(Debug, Default)]
pub struct AudioManager {
    enabled: bool,
    title_music_stop: Option<Arc<AtomicBool>>,
    title_music_child: Arc<Mutex<Option<Child>>>,
    title_music_thread: Option<JoinHandle<()>>,
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            enabled: Path::new("/usr/bin/afplay").exists(),
            title_music_stop: None,
            title_music_child: Arc::new(Mutex::new(None)),
            title_music_thread: None,
        }
    }

    pub fn handle_event(&mut self, event: GameEvent) {
        if !self.enabled {
            return;
        }

        match event {
            GameEvent::TitleScreenEntered => self.play_title_music(),
            GameEvent::ButtonClicked => self.play_effect("button_click.ogg"),
            GameEvent::GameStarted => {
                self.stop_title_music();
                self.play_effect("level_start.wav");
            }
            GameEvent::SmallPelletEaten => self.play_effect("small_pellet2.wav"),
            GameEvent::PowerPelletEaten => self.play_effect("fright_mode_short.wav"),
            GameEvent::GhostEaten => self.play_effect("ghost_eat.wav"),
            GameEvent::FruitEaten => self.play_effect("fruit_eat.wav"),
            GameEvent::PacmanDied => self.play_effect("death.wav"),
            GameEvent::LevelCompleted => self.play_effect("level_complete.wav"),
        }
    }

    fn play_title_music(&mut self) {
        self.stop_title_music();

        let stop = Arc::new(AtomicBool::new(false));
        let stop_flag = stop.clone();
        let child_slot = self.title_music_child.clone();
        let music_path = sound_path("music.ogg");

        self.title_music_thread = Some(thread::spawn(move || {
            while !stop_flag.load(Ordering::SeqCst) {
                let child = Command::new("/usr/bin/afplay")
                    .arg(&music_path)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn();
                let Ok(child) = child else {
                    break;
                };

                {
                    let mut slot = child_slot
                        .lock()
                        .expect("title music child slot should not be poisoned");
                    *slot = Some(child);
                }

                let wait_result = {
                    let mut slot = child_slot
                        .lock()
                        .expect("title music child slot should not be poisoned");
                    slot.as_mut()
                        .expect("title music child should be present")
                        .wait()
                };

                {
                    let mut slot = child_slot
                        .lock()
                        .expect("title music child slot should not be poisoned");
                    *slot = None;
                }

                if wait_result.is_err() {
                    break;
                }
            }
        }));

        self.title_music_stop = Some(stop);
    }

    fn stop_title_music(&mut self) {
        if let Some(stop) = &self.title_music_stop {
            stop.store(true, Ordering::SeqCst);
        }

        if let Some(child) = self
            .title_music_child
            .lock()
            .expect("title music child slot should not be poisoned")
            .as_mut()
        {
            let _ = child.kill();
            let _ = child.wait();
        }

        if let Some(handle) = self.title_music_thread.take() {
            let _ = handle.join();
        }

        self.title_music_stop = None;
    }

    fn play_effect(&self, filename: &str) {
        let _ = Command::new("/usr/bin/afplay")
            .arg(sound_path(filename))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        self.stop_title_music();
    }
}

fn sound_path(filename: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("Sounds")
        .join(filename)
}

#[cfg(test)]
mod tests {
    use super::sound_path;

    #[test]
    fn sound_paths_point_into_the_repo_assets_directory() {
        let path = sound_path("music.ogg");

        assert!(path.ends_with("assets/Sounds/music.ogg"));
    }
}
