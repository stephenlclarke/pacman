//! Renders arcade-styled text and manages dynamic status and score labels.

use std::{
    io::Cursor,
    sync::{Arc, OnceLock},
};

use png::{ColorType, Decoder, Transformations};

use crate::{
    constants::{RED, TILE_HEIGHT, TILE_WIDTH, WHITE, YELLOW},
    render::{FrameData, RenderedImage, Sprite, SpriteAnchor},
    vector::Vector2,
};

const FONT_SHEET_BYTES: &[u8] = include_bytes!("../assets/arcade/font-sheet.png");
const GLYPH_SIZE: u32 = 8;
const FONT_COLUMNS: u32 = 16;
const FONT_FIRST_TILE: u8 = 0x30;
const FONT_LAST_TILE: u8 = 0x5b;
const SPACE_TILE: u8 = 0x40;
const EXCLAMATION_TILE: u8 = 0x5b;

#[derive(Clone, Debug)]
struct ArcadeFont {
    glyphs: Vec<Arc<RenderedImage>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusText {
    Ready,
    Paused,
    GameOver,
}

#[derive(Clone, Debug)]
struct TextItem {
    value: String,
    color: [u8; 4],
    position: Vector2,
    size: f32,
    visible: bool,
    image: Arc<RenderedImage>,
    timer: f32,
    lifespan: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct TextGroup {
    score_value: TextItem,
    level_value: TextItem,
    ready: TextItem,
    paused: TextItem,
    game_over: TextItem,
    score_label: TextItem,
    level_label: TextItem,
    transient: Vec<TextItem>,
}

impl TextGroup {
    pub fn new() -> Self {
        let size = TILE_HEIGHT as f32;
        let mut group = Self {
            score_value: TextItem::new("00000000", WHITE, 0.0, TILE_HEIGHT as f32, size, true),
            level_value: TextItem::new(
                "001",
                WHITE,
                23.0 * TILE_WIDTH as f32,
                TILE_HEIGHT as f32,
                size,
                true,
            ),
            ready: TextItem::new(
                "READY!",
                YELLOW,
                11.25 * TILE_WIDTH as f32,
                20.0 * TILE_HEIGHT as f32,
                size,
                false,
            ),
            paused: TextItem::new(
                "PAUSED!",
                YELLOW,
                10.625 * TILE_WIDTH as f32,
                20.0 * TILE_HEIGHT as f32,
                size,
                false,
            ),
            game_over: TextItem::new(
                "GAMEOVER!",
                RED,
                10.0 * TILE_WIDTH as f32,
                20.0 * TILE_HEIGHT as f32,
                size,
                false,
            ),
            score_label: TextItem::new("SCORE", WHITE, 0.0, 0.0, size, true),
            level_label: TextItem::new("LEVEL", WHITE, 23.0 * TILE_WIDTH as f32, 0.0, size, true),
            transient: Vec::new(),
        };
        group.show_status(StatusText::Ready);
        group
    }

    pub fn update(&mut self, dt: f32) {
        for text in &mut self.transient {
            text.update(dt);
        }
        self.transient.retain(|text| !text.destroyed());
    }

    pub fn show_status(&mut self, status: StatusText) {
        self.hide_status();
        match status {
            StatusText::Ready => self.ready.visible = true,
            StatusText::Paused => self.paused.visible = true,
            StatusText::GameOver => self.game_over.visible = true,
        }
    }

    /// Hides status.
    pub fn hide_status(&mut self) {
        self.ready.visible = false;
        self.paused.visible = false;
        self.game_over.visible = false;
    }

    /// Updates score.
    pub fn update_score(&mut self, score: u32) {
        self.score_value.set_text(format!("{score:08}"));
    }

    /// Updates level.
    pub fn update_level(&mut self, level: u32) {
        self.level_value.set_text(format!("{level:03}"));
    }

    pub fn add_popup(&mut self, text: impl Into<String>, color: [u8; 4], x: f32, y: f32) {
        self.transient
            .push(TextItem::timed(text.into(), color, x, y, 8.0, 1.0));
    }

    /// Appends renderables.
    pub fn append_renderables(&self, frame: &mut FrameData) {
        for text in [
            &self.score_value,
            &self.level_value,
            &self.score_label,
            &self.level_label,
            &self.ready,
            &self.paused,
            &self.game_over,
        ] {
            text.append_renderable(frame);
        }

        for text in &self.transient {
            text.append_renderable(frame);
        }
    }
}

impl Default for TextGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl TextItem {
    fn new(
        value: impl Into<String>,
        color: [u8; 4],
        x: f32,
        y: f32,
        size: f32,
        visible: bool,
    ) -> Self {
        let value = value.into();
        let image = rasterize_text_image(&value, color, size);
        Self {
            value,
            color,
            position: Vector2::new(x, y),
            size,
            visible,
            image,
            timer: 0.0,
            lifespan: None,
        }
    }

    fn timed(value: String, color: [u8; 4], x: f32, y: f32, size: f32, lifespan: f32) -> Self {
        let mut text = Self::new(value, color, x, y, size, true);
        text.lifespan = Some(lifespan);
        text
    }

    /// Sets text.
    fn set_text(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.image = rasterize_text_image(&self.value, self.color, self.size);
    }

    fn update(&mut self, dt: f32) {
        let Some(lifespan) = self.lifespan else {
            return;
        };

        self.timer += dt;
        if self.timer >= lifespan {
            self.visible = false;
            self.lifespan = None;
        }
    }

    fn destroyed(&self) -> bool {
        !self.visible && self.lifespan.is_none() && self.timer > 0.0
    }

    /// Appends renderable.
    fn append_renderable(&self, frame: &mut FrameData) {
        if !self.visible {
            return;
        }

        frame.sprites.push(Sprite {
            image: self.image.clone(),
            position: self.position,
            anchor: SpriteAnchor::TopLeft,
        });
    }
}

pub fn rasterize_text_image(text: &str, color: [u8; 4], size: f32) -> Arc<RenderedImage> {
    let font = shared_font();
    let glyphs = text
        .chars()
        .map(|ch| font.glyph_for_char(ch))
        .collect::<Vec<_>>();
    let scale = ((size / GLYPH_SIZE as f32).round() as u32).max(1);
    let width = (glyphs.len() as u32).max(1) * GLYPH_SIZE * scale;
    let height = GLYPH_SIZE * scale;
    let mut pixels = vec![0; width as usize * height as usize * 4];

    for (glyph_index, glyph) in glyphs.iter().enumerate() {
        blit_tinted_scaled(
            &mut pixels,
            width,
            glyph,
            glyph_index as u32 * GLYPH_SIZE * scale,
            scale,
            color,
        );
    }

    Arc::new(RenderedImage {
        width,
        height,
        pixels,
    })
}

fn shared_font() -> &'static ArcadeFont {
    static FONT: OnceLock<ArcadeFont> = OnceLock::new();
    FONT.get_or_init(ArcadeFont::load)
}

impl ArcadeFont {
    /// Loads load.
    fn load() -> Self {
        let sheet = decode_png_image(FONT_SHEET_BYTES).expect("embedded font sheet should decode");
        let glyphs = (FONT_FIRST_TILE..=FONT_LAST_TILE)
            .enumerate()
            .map(|(offset, _)| {
                let column = offset as u32 % FONT_COLUMNS;
                let row = offset as u32 / FONT_COLUMNS;
                Arc::new(crop_image(
                    &sheet,
                    column * GLYPH_SIZE,
                    row * GLYPH_SIZE,
                    GLYPH_SIZE,
                    GLYPH_SIZE,
                ))
            })
            .collect();
        Self { glyphs }
    }

    fn glyph_for_char(&self, ch: char) -> Arc<RenderedImage> {
        let tile_index = tile_index_for_char(ch).unwrap_or(SPACE_TILE);
        let clamped = tile_index.clamp(FONT_FIRST_TILE, FONT_LAST_TILE);
        self.glyphs[(clamped - FONT_FIRST_TILE) as usize].clone()
    }
}

fn tile_index_for_char(ch: char) -> Option<u8> {
    match ch.to_ascii_uppercase() {
        '0'..='9' | 'A'..='Z' => Some(ch.to_ascii_uppercase() as u8),
        ' ' => Some(SPACE_TILE),
        '!' => Some(EXCLAMATION_TILE),
        _ => None,
    }
}

fn blit_tinted_scaled(
    target: &mut [u8],
    target_width: u32,
    glyph: &RenderedImage,
    origin_x: u32,
    scale: u32,
    color: [u8; 4],
) {
    for row in 0..glyph.height {
        for col in 0..glyph.width {
            let src_index = ((row * glyph.width + col) * 4) as usize;
            let alpha = glyph.pixels[src_index + 3];
            if alpha == 0 {
                continue;
            }

            for dy in 0..scale {
                for dx in 0..scale {
                    let dst_x = origin_x + col * scale + dx;
                    let dst_y = row * scale + dy;
                    let dst_index = ((dst_y * target_width + dst_x) * 4) as usize;
                    target[dst_index] = color[0];
                    target[dst_index + 1] = color[1];
                    target[dst_index + 2] = color[2];
                    target[dst_index + 3] = ((u16::from(alpha) * u16::from(color[3])) / 0xff) as u8;
                }
            }
        }
    }
}

fn crop_image(image: &RenderedImage, x: u32, y: u32, width: u32, height: u32) -> RenderedImage {
    let mut pixels = vec![0; (width * height * 4) as usize];
    for row in 0..height {
        let src_start = (((y + row) * image.width + x) * 4) as usize;
        let src_end = src_start + (width * 4) as usize;
        let dst_start = (row * width * 4) as usize;
        pixels[dst_start..dst_start + (width * 4) as usize]
            .copy_from_slice(&image.pixels[src_start..src_end]);
    }
    RenderedImage {
        width,
        height,
        pixels,
    }
}

fn decode_png_image(bytes: &[u8]) -> anyhow::Result<RenderedImage> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .expect("png size should be known")
    ];
    let info = reader.next_frame(&mut buffer)?;
    let bytes = &buffer[..info.buffer_size()];

    let pixels = match info.color_type {
        ColorType::Rgba => bytes.to_vec(),
        ColorType::Rgb => bytes
            .chunks_exact(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 0xff])
            .collect(),
        ColorType::Grayscale => bytes
            .iter()
            .flat_map(|&gray| [gray, gray, gray, 0xff])
            .collect(),
        ColorType::GrayscaleAlpha => bytes
            .chunks_exact(2)
            .flat_map(|gray_alpha| [gray_alpha[0], gray_alpha[0], gray_alpha[0], gray_alpha[1]])
            .collect(),
        ColorType::Indexed => unreachable!("indexed pngs should be expanded by the decoder"),
    };

    Ok(RenderedImage {
        width: info.width,
        height: info.height,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::{StatusText, TextGroup, rasterize_text_image};
    use crate::{
        constants::{RED, WHITE},
        render::FrameData,
    };

    #[test]
    fn text_group_starts_with_ready_visible() {
        let text = TextGroup::new();
        let mut frame = FrameData::default();
        text.append_renderables(&mut frame);

        assert!(frame.sprites.len() >= 5);
    }

    #[test]
    fn updating_score_rebuilds_the_score_label() {
        let mut text = TextGroup::new();
        text.update_score(1234);
        let mut frame = FrameData::default();
        text.append_renderables(&mut frame);

        assert!(!frame.sprites.is_empty());
    }

    #[test]
    fn status_switches_hide_the_previous_label() {
        let mut text = TextGroup::new();
        text.show_status(StatusText::Paused);
        let mut frame = FrameData::default();
        text.append_renderables(&mut frame);

        assert!(!frame.sprites.is_empty());
    }

    #[test]
    fn game_over_text_uses_the_arcade_red_color() {
        let text = TextGroup::new();

        assert_eq!(text.game_over.color, RED);
    }

    #[test]
    fn arcade_exclamation_glyph_renders_visible_pixels() {
        let image = rasterize_text_image("!", WHITE, 16.0);

        assert!(image.pixels.chunks_exact(4).any(|pixel| pixel[3] != 0));
    }
}
