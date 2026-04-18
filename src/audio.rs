//! Embeds the game sound effects and coordinates audio playback for emitted gameplay events.

use std::io::Cursor;

use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source, source::Repeat};

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

struct AudioOutput {
    stream: OutputStream,
}

type SoundDecoder = Decoder<Cursor<&'static [u8]>>;
type LoopingSoundDecoder = Repeat<SoundDecoder>;

pub struct AudioManager {
    output: Option<AudioOutput>,
    title_music: Option<Sink>,
    freight_sound: Option<Sink>,
}

impl Default for AudioManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundAsset {
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

impl AudioOutput {
    fn new() -> Option<Self> {
        let mut stream = OutputStreamBuilder::open_default_stream().ok()?;
        stream.log_on_drop(false);
        Some(Self { stream })
    }

    fn new_sink(&self) -> Sink {
        Sink::connect_new(self.stream.mixer())
    }
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            output: AudioOutput::new(),
            title_music: None,
            freight_sound: None,
        }
    }

    pub fn handle_event(&mut self, event: GameEvent) {
        if self.output.is_none() {
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
        self.stop_title_music();
        self.title_music = self.new_looping_sink(SoundAsset::Music);
    }

    fn stop_title_music(&mut self) {
        stop_sink(&mut self.title_music);
    }

    fn play_freight_sound(&mut self) {
        self.stop_freight_sound();
        self.freight_sound = self.new_looping_sink(SoundAsset::FreightMode);
    }

    fn stop_freight_sound(&mut self) {
        stop_sink(&mut self.freight_sound);
    }

    fn play_effect(&self, sound: SoundAsset) {
        let Some(output) = self.output.as_ref() else {
            return;
        };
        let Some(source) = sound_decoder(sound) else {
            return;
        };
        let sink = output.new_sink();
        sink.append(source);
        sink.detach();
    }

    fn new_looping_sink(&self, sound: SoundAsset) -> Option<Sink> {
        let output = self.output.as_ref()?;
        let source = looping_sound_decoder(sound)?;
        let sink = output.new_sink();
        sink.append(source);
        Some(sink)
    }
}

impl Drop for AudioManager {
    fn drop(&mut self) {
        self.stop_title_music();
        self.stop_freight_sound();
    }
}

fn stop_sink(sink: &mut Option<Sink>) {
    if let Some(sink) = sink.take() {
        sink.stop();
    }
}

fn sound_decoder(sound: SoundAsset) -> Option<SoundDecoder> {
    Decoder::new(Cursor::new(sound.bytes())).ok()
}

fn looping_sound_decoder(sound: SoundAsset) -> Option<LoopingSoundDecoder> {
    sound_decoder(sound).map(Source::repeat_infinite)
}

#[cfg(test)]
mod tests {
    use super::{SoundAsset, looping_sound_decoder, sound_decoder};

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
    fn every_embedded_sound_asset_has_bytes() {
        for sound in all_assets() {
            assert!(!sound.bytes().is_empty());
        }
    }

    #[test]
    fn every_embedded_sound_asset_can_be_decoded() {
        for sound in all_assets() {
            let source = sound_decoder(sound);
            assert!(source.is_some(), "sound asset should decode");
        }
    }

    #[test]
    fn looping_sound_assets_can_be_created_from_embedded_bytes() {
        assert!(looping_sound_decoder(SoundAsset::Music).is_some());
        assert!(looping_sound_decoder(SoundAsset::FreightMode).is_some());
    }
}
