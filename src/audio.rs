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
            GameEvent::ButtonClicked => self.play_effect("button_click.ogg"),
            GameEvent::GameStarted => {
                self.stop_title_music();
                self.stop_freight_sound();
                self.play_effect("level_start.wav");
            }
            GameEvent::SmallPelletEaten => self.play_effect("small_pellet2.wav"),
            GameEvent::PowerPelletEaten => {}
            GameEvent::FreightModeStarted => self.play_freight_sound(),
            GameEvent::FreightModeEnded => self.stop_freight_sound(),
            GameEvent::GhostEaten => self.play_effect("ghost_eat.wav"),
            GameEvent::FruitEaten => self.play_effect("fruit_eat.wav"),
            GameEvent::PacmanDied => {
                self.stop_freight_sound();
                self.play_effect("death.wav");
            }
            GameEvent::LevelCompleted => {
                self.stop_freight_sound();
                self.play_effect("level_complete.wav");
            }
        }
    }

    fn play_title_music(&mut self) {
        self.title_music.play(sound_path("music.ogg"));
    }

    fn stop_title_music(&mut self) {
        self.title_music.stop();
    }

    fn play_freight_sound(&mut self) {
        self.freight_sound.play(sound_path("fright_mode_short.wav"));
    }

    fn stop_freight_sound(&mut self) {
        self.freight_sound.stop();
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
        self.stop_freight_sound();
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
