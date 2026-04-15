use std::io::{IsTerminal, Stdout, Write};

use anyhow::{Result, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use crossterm::{
    cursor::MoveTo,
    queue,
    terminal::{Clear, ClearType},
};
use png::{BitDepth, ColorType, Compression, Encoder};

use crate::render::RenderedImage;

const IMAGE_ID: u32 = 7;
const CHUNK_SIZE: usize = 4_096;
const ESCAPE_BEGIN: &str = "\x1b_G";
const ESCAPE_END: &str = "\x1b\\";

pub struct KittyGraphics {
    placement_cols: u16,
    placement_rows: u16,
    png_buffer: Vec<u8>,
    base64_buffer: String,
}

impl KittyGraphics {
    pub fn new(placement_cols: u16, placement_rows: u16) -> Self {
        Self {
            placement_cols,
            placement_rows,
            png_buffer: Vec::new(),
            base64_buffer: String::new(),
        }
    }

    pub fn ensure_supported() -> Result<()> {
        let term = std::env::var("TERM").unwrap_or_default();
        let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
        let force = std::env::var("PACMAN_FORCE_KITTY").unwrap_or_default();
        let kitty_window = std::env::var_os("KITTY_WINDOW_ID").is_some();

        if force == "1" || kitty_window || is_known_kitty_graphics_terminal(&term, &term_program) {
            return Ok(());
        }

        validate_environment(&term, std::io::stdout().is_terminal())
    }

    pub fn resize(&mut self, placement_cols: u16, placement_rows: u16) {
        self.placement_cols = placement_cols;
        self.placement_rows = placement_rows;
    }

    pub fn draw_frame(&mut self, stdout: &mut Stdout, image: &RenderedImage) -> Result<()> {
        encode_png_into(image, &mut self.png_buffer)?;
        self.base64_buffer.clear();
        STANDARD.encode_string(&self.png_buffer, &mut self.base64_buffer);
        let chunk_count = self.base64_buffer.len().div_ceil(CHUNK_SIZE);

        queue!(stdout, MoveTo(0, 0), Clear(ClearType::All))?;

        for (index, chunk) in self.base64_buffer.as_bytes().chunks(CHUNK_SIZE).enumerate() {
            let more = if index + 1 == chunk_count { 0 } else { 1 };
            if index == 0 {
                write!(
                    stdout,
                    "{ESCAPE_BEGIN}a=T,f=100,i={IMAGE_ID},q=2,C=1,c={},r={},z=-1,m={more};",
                    self.placement_cols, self.placement_rows
                )?;
            } else {
                write!(stdout, "{ESCAPE_BEGIN}m={more};")?;
            }

            stdout.write_all(chunk)?;
            write!(stdout, "{ESCAPE_END}")?;
        }

        Ok(())
    }

    pub fn clear(&self, stdout: &mut Stdout) -> Result<()> {
        write!(stdout, "{ESCAPE_BEGIN}a=d,d=I,i={IMAGE_ID},q=2{ESCAPE_END}")?;
        Ok(())
    }
}

fn is_known_kitty_graphics_terminal(term: &str, term_program: &str) -> bool {
    term == "xterm-kitty"
        || term == "xterm-ghostty"
        || term_program == "ghostty"
        || term_program == "kitty"
        || term_program == "WarpTerminal"
}

fn validate_environment(term: &str, is_terminal: bool) -> Result<()> {
    if !is_terminal {
        bail!(
            "Kitty graphics output requires an interactive terminal on stdout. \
             Run inside kitty or another compatible terminal."
        );
    }

    if term.is_empty() || term == "dumb" {
        bail!(
            "TERM={term:?} does not expose the interactive terminal capabilities needed for \
             Kitty graphics. Run inside kitty or set PACMAN_FORCE_KITTY=1 to bypass this \
             basic check."
        );
    }

    Ok(())
}

#[cfg(test)]
fn encode_png(image: &RenderedImage) -> Result<Vec<u8>> {
    let mut encoded = Vec::new();
    encode_png_into(image, &mut encoded)?;
    Ok(encoded)
}

fn encode_png_into(image: &RenderedImage, encoded: &mut Vec<u8>) -> Result<()> {
    encoded.clear();
    let mut encoder = Encoder::new(encoded, image.width, image.height);
    encoder.set_color(ColorType::Rgba);
    encoder.set_depth(BitDepth::Eight);
    encoder.set_compression(Compression::Fast);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&image.pixels)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        sync::{Mutex, OnceLock},
    };

    use super::{
        CHUNK_SIZE, KittyGraphics, encode_png, is_known_kitty_graphics_terminal,
        validate_environment,
    };
    use crate::render::RenderedImage;

    fn env_lock() -> &'static Mutex<()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_env_vars<T>(vars: &[(&str, Option<&str>)], f: impl FnOnce() -> T) -> T {
        let _guard = env_lock().lock().expect("env lock should not be poisoned");
        let previous = vars
            .iter()
            .map(|(key, _)| ((*key).to_string(), env::var_os(key)))
            .collect::<Vec<_>>();
        for (key, value) in vars {
            match value {
                Some(value) => unsafe { env::set_var(key, value) },
                None => unsafe { env::remove_var(key) },
            }
        }
        let result = f();
        for (key, value) in previous {
            match value {
                Some(value) => unsafe { env::set_var(&key, value) },
                None => unsafe { env::remove_var(&key) },
            }
        }
        result
    }

    #[test]
    fn png_encoder_writes_signature() {
        let image = RenderedImage {
            width: 2,
            height: 2,
            pixels: vec![
                0, 0, 0, 255, 255, 255, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255,
            ],
        };

        let png = encode_png(&image).expect("png encoding should succeed");
        assert!(png.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));
    }

    #[test]
    fn chunk_size_matches_protocol_limit() {
        assert_eq!(CHUNK_SIZE, 4_096);
    }

    #[test]
    fn environment_check_allows_interactive_terminals() {
        validate_environment("xterm-256color", true).expect("interactive terminals should pass");
    }

    #[test]
    fn environment_check_rejects_dumb_terminals() {
        assert!(validate_environment("dumb", true).is_err());
    }

    #[test]
    fn known_terminals_include_kitty() {
        assert!(is_known_kitty_graphics_terminal("xterm-kitty", ""));
        assert!(is_known_kitty_graphics_terminal("xterm-ghostty", "ghostty"));
        assert!(is_known_kitty_graphics_terminal("", "WarpTerminal"));
    }

    #[test]
    fn force_flag_bypasses_terminal_detection() {
        with_env_vars(
            &[
                ("PACMAN_FORCE_KITTY", Some("1")),
                ("TERM", Some("dumb")),
                ("TERM_PROGRAM", None),
            ],
            || {
                assert!(KittyGraphics::ensure_supported().is_ok());
            },
        );
    }

    #[test]
    fn kitty_window_id_bypasses_terminal_detection() {
        with_env_vars(
            &[
                ("PACMAN_FORCE_KITTY", None),
                ("KITTY_WINDOW_ID", Some("123")),
                ("TERM", Some("dumb")),
                ("TERM_PROGRAM", None),
            ],
            || {
                assert!(KittyGraphics::ensure_supported().is_ok());
            },
        );
    }

    #[test]
    fn resize_updates_placement_dimensions() {
        let mut graphics = KittyGraphics::new(10, 20);
        graphics.resize(30, 40);

        assert_eq!(graphics.placement_cols, 30);
        assert_eq!(graphics.placement_rows, 40);
    }
}
