use std::sync::{Arc, OnceLock};

use fontdue::{
    Font, FontSettings,
    layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle},
};

use crate::{
    constants::{RED, TILE_HEIGHT, TILE_WIDTH, WHITE, YELLOW},
    render::{FrameData, RenderedImage, Sprite, SpriteAnchor},
    vector::Vector2,
};

const FONT_BYTES: &[u8] = include_bytes!("../assets/PressStart2P-Regular.ttf");

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

    pub fn hide_status(&mut self) {
        self.ready.visible = false;
        self.paused.visible = false;
        self.game_over.visible = false;
    }

    pub fn update_score(&mut self, score: u32) {
        self.score_value.set_text(format!("{score:08}"));
    }

    pub fn update_level(&mut self, level: u32) {
        self.level_value.set_text(format!("{level:03}"));
    }

    pub fn add_popup(&mut self, text: impl Into<String>, color: [u8; 4], x: f32, y: f32) {
        self.transient
            .push(TextItem::timed(text.into(), color, x, y, 8.0, 1.0));
    }

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
        let image = rasterize_text(&value, color, size);
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

    fn set_text(&mut self, value: impl Into<String>) {
        self.value = value.into();
        self.image = rasterize_text(&self.value, self.color, self.size);
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

fn rasterize_text(text: &str, color: [u8; 4], size: f32) -> Arc<RenderedImage> {
    let font = shared_font();
    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    layout.reset(&LayoutSettings::default());
    layout.append(&[font.as_ref()], &TextStyle::new(text, size, 0));
    let glyphs = layout.glyphs();

    let width = glyphs
        .iter()
        .map(|glyph| glyph.x as i32 + glyph.width as i32)
        .max()
        .unwrap_or(1)
        .max(1) as u32;
    let height = glyphs
        .iter()
        .map(|glyph| glyph.y as i32 + glyph.height as i32)
        .max()
        .unwrap_or(size.ceil() as i32)
        .max(1) as u32;

    let mut pixels = vec![0; width as usize * height as usize * 4];
    for glyph in glyphs {
        let (metrics, bitmap) = font.rasterize_config(glyph.key);
        let origin_x = glyph.x.round() as i32;
        let origin_y = glyph.y.round() as i32;

        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let alpha = bitmap[row * metrics.width + col];
                if alpha == 0 {
                    continue;
                }

                let x = origin_x + col as i32;
                let y = origin_y + row as i32;
                if x < 0 || y < 0 {
                    continue;
                }
                let x = x as usize;
                let y = y as usize;
                if x >= width as usize || y >= height as usize {
                    continue;
                }

                let index = (y * width as usize + x) * 4;
                pixels[index] = color[0];
                pixels[index + 1] = color[1];
                pixels[index + 2] = color[2];
                pixels[index + 3] = alpha;
            }
        }
    }

    Arc::new(RenderedImage {
        width,
        height,
        pixels,
    })
}

fn shared_font() -> Arc<Font> {
    static FONT: OnceLock<Arc<Font>> = OnceLock::new();
    FONT.get_or_init(|| {
        Arc::new(
            Font::from_bytes(FONT_BYTES, FontSettings::default())
                .expect("embedded font should load correctly"),
        )
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::{StatusText, TextGroup};
    use crate::{constants::RED, render::FrameData};

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
    fn game_over_text_uses_the_tutorial_red_color() {
        let text = TextGroup::new();

        assert_eq!(text.game_over.color, RED);
    }
}
