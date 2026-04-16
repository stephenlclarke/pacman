//! Loads embedded sprite assets and exposes animation helpers for the arcade visuals.

use std::{
    io::Cursor,
    sync::{Arc, OnceLock},
};

use anyhow::Context;
use png::{ColorType, Decoder, Transformations};

use crate::{
    actors::GhostKind, animation::Animator, arcade, modes::GhostMode, pacman::Direction,
    render::RenderedImage,
};

#[derive(Clone, Debug)]
struct ArcadeActorAssets {
    pacman_left: Arc<RenderedImage>,
    pacman_right: Arc<RenderedImage>,
    pacman_up: Arc<RenderedImage>,
    pacman_down: Arc<RenderedImage>,
    pacman_closed: Arc<RenderedImage>,
    pacman_death: [Arc<RenderedImage>; 11],
    ghost_left: [Arc<RenderedImage>; 4],
    ghost_down: [Arc<RenderedImage>; 4],
    ghost_right: [Arc<RenderedImage>; 4],
    ghost_up: [Arc<RenderedImage>; 4],
    ghost_eyes_up: Arc<RenderedImage>,
    ghost_eyes_down: Arc<RenderedImage>,
    ghost_eyes_left: Arc<RenderedImage>,
    ghost_eyes_right: Arc<RenderedImage>,
    ghost_freight: [Arc<RenderedImage>; 2],
    ghost_freight_flash: [Arc<RenderedImage>; 2],
    fruit_items: [Arc<RenderedImage>; 8],
    fruit_icons: [Arc<RenderedImage>; 8],
}

#[derive(Clone, Debug)]
pub struct PacmanSprites {
    left: Arc<RenderedImage>,
    right: Arc<RenderedImage>,
    up: Arc<RenderedImage>,
    down: Arc<RenderedImage>,
    closed: Arc<RenderedImage>,
    death: Animator<Arc<RenderedImage>>,
    stop_left: Arc<RenderedImage>,
    stop_right: Arc<RenderedImage>,
    stop_up: Arc<RenderedImage>,
    stop_down: Arc<RenderedImage>,
    stop_image: Arc<RenderedImage>,
    current: Arc<RenderedImage>,
    chomp_dt: f32,
    chomp_open: bool,
}

#[derive(Clone, Debug)]
pub struct GhostSprites {
    start: [Arc<RenderedImage>; 4],
    up: [Arc<RenderedImage>; 4],
    down: [Arc<RenderedImage>; 4],
    left: [Arc<RenderedImage>; 4],
    right: [Arc<RenderedImage>; 4],
    spawn_up: Arc<RenderedImage>,
    spawn_down: Arc<RenderedImage>,
    spawn_left: Arc<RenderedImage>,
    spawn_right: Arc<RenderedImage>,
    freight: [Arc<RenderedImage>; 2],
    freight_flash: [Arc<RenderedImage>; 2],
}

#[derive(Clone, Debug)]
pub struct FruitSprites {
    items: [Arc<RenderedImage>; 8],
    icons: [Arc<RenderedImage>; 8],
}

#[derive(Clone, Debug)]
pub struct LifeSprites {
    image: Arc<RenderedImage>,
    lives: usize,
}

#[derive(Clone, Debug)]
pub struct MazeSprites {
    background: Arc<RenderedImage>,
    flash_background: Arc<RenderedImage>,
}

impl PacmanSprites {
    pub fn new() -> Self {
        let assets = shared_arcade_assets();
        let stop_left = assets.pacman_left.clone();
        let stop_right = assets.pacman_right.clone();
        let stop_up = assets.pacman_up.clone();
        let stop_down = assets.pacman_down.clone();
        let closed = assets.pacman_closed.clone();

        Self {
            left: stop_left.clone(),
            right: stop_right.clone(),
            up: stop_up.clone(),
            down: stop_down.clone(),
            closed: closed.clone(),
            death: Animator::new(assets.pacman_death.to_vec(), 6.0, false),
            stop_left: stop_left.clone(),
            stop_right,
            stop_up,
            stop_down,
            stop_image: stop_left.clone(),
            current: stop_left,
            chomp_dt: 0.0,
            chomp_open: false,
        }
    }

    /// Resets reset.
    pub fn reset(&mut self) {
        self.death.reset();
        self.stop_image = self.stop_left.clone();
        self.current = self.stop_left.clone();
        self.chomp_dt = 0.0;
        self.chomp_open = false;
    }

    pub fn update(&mut self, dt: f32, direction: Direction) -> Arc<RenderedImage> {
        self.update_for_state(dt, direction, true)
    }

    /// Updates for state.
    pub fn update_for_state(
        &mut self,
        dt: f32,
        direction: Direction,
        alive: bool,
    ) -> Arc<RenderedImage> {
        self.current = if alive {
            if direction == Direction::Stop {
                self.chomp_dt = 0.0;
                self.chomp_open = false;
                self.stop_image.clone()
            } else {
                self.chomp_dt += dt;
                if self.chomp_dt >= 1.0 / 20.0 {
                    self.chomp_open = !self.chomp_open;
                    self.chomp_dt = 0.0;
                }

                let directional_image = match direction {
                    Direction::Left => {
                        self.stop_image = self.stop_left.clone();
                        self.left.clone()
                    }
                    Direction::Right => {
                        self.stop_image = self.stop_right.clone();
                        self.right.clone()
                    }
                    Direction::Up => {
                        self.stop_image = self.stop_up.clone();
                        self.up.clone()
                    }
                    Direction::Down => {
                        self.stop_image = self.stop_down.clone();
                        self.down.clone()
                    }
                    Direction::Stop => unreachable!("stop handled above"),
                };

                if self.chomp_open {
                    directional_image
                } else {
                    self.closed.clone()
                }
            }
        } else {
            self.death.update(dt)
        };

        self.current.clone()
    }

    pub fn current(&self) -> Arc<RenderedImage> {
        self.current.clone()
    }
}

impl Default for PacmanSprites {
    fn default() -> Self {
        Self::new()
    }
}

impl GhostSprites {
    pub fn new() -> Self {
        let assets = shared_arcade_assets();
        let left = assets.ghost_left.clone();
        let down = assets.ghost_down.clone();
        let right = assets.ghost_right.clone();
        let up = assets.ghost_up.clone();
        let start = up.clone();

        Self {
            start,
            up,
            down,
            left,
            right,
            spawn_up: assets.ghost_eyes_up.clone(),
            spawn_down: assets.ghost_eyes_down.clone(),
            spawn_left: assets.ghost_eyes_left.clone(),
            spawn_right: assets.ghost_eyes_right.clone(),
            freight: assets.ghost_freight.clone(),
            freight_flash: assets.ghost_freight_flash.clone(),
        }
    }

    pub fn image(
        &self,
        kind: GhostKind,
        mode: GhostMode,
        direction: Direction,
        freight_remaining: Option<f32>,
        fright_total_duration: Option<f32>,
    ) -> Arc<RenderedImage> {
        let index = kind.index();

        match mode {
            GhostMode::Scatter | GhostMode::Chase => match direction {
                Direction::Up => self.up[index].clone(),
                Direction::Down => self.down[index].clone(),
                Direction::Left => self.left[index].clone(),
                Direction::Right => self.right[index].clone(),
                Direction::Stop => self.start[index].clone(),
            },
            GhostMode::Freight => self.freight_image(freight_remaining, fright_total_duration),
            GhostMode::Spawn => match direction {
                Direction::Up => self.spawn_up.clone(),
                Direction::Down => self.spawn_down.clone(),
                Direction::Left => self.spawn_left.clone(),
                Direction::Right => self.spawn_right.clone(),
                Direction::Stop => self.spawn_up.clone(),
            },
        }
    }

    fn freight_image(
        &self,
        freight_remaining: Option<f32>,
        fright_total_duration: Option<f32>,
    ) -> Arc<RenderedImage> {
        let elapsed = freight_remaining
            .zip(fright_total_duration)
            .map(|(remaining, total)| (total - remaining).max(0.0))
            .unwrap_or(0.0);
        let frame = ((elapsed / arcade::fright_flash_half_period_seconds()).floor() as usize) % 2;

        let flashing = freight_remaining
            .zip(fright_total_duration)
            .is_some_and(|(remaining, total)| remaining <= arcade::fright_flash_duration(total));

        if flashing {
            self.freight_flash[frame].clone()
        } else {
            self.freight[frame].clone()
        }
    }
}

impl Default for GhostSprites {
    fn default() -> Self {
        Self::new()
    }
}

impl FruitSprites {
    pub fn new() -> Self {
        let assets = shared_arcade_assets();
        Self {
            items: assets.fruit_items.clone(),
            icons: assets.fruit_icons.clone(),
        }
    }

    pub fn item_image(&self, index: usize) -> Arc<RenderedImage> {
        self.items[index % self.items.len()].clone()
    }

    pub fn icon_image(&self, index: usize) -> Arc<RenderedImage> {
        self.icons[index % self.icons.len()].clone()
    }

    pub fn image(&self, index: usize) -> Arc<RenderedImage> {
        self.item_image(index)
    }

    pub fn image_for_level(&self, level_index: u32) -> Arc<RenderedImage> {
        self.item_image(level_index as usize)
    }
}

impl Default for FruitSprites {
    fn default() -> Self {
        Self::new()
    }
}

impl LifeSprites {
    pub fn new(num_lives: u32) -> Self {
        Self {
            image: shared_arcade_assets().pacman_left.clone(),
            lives: num_lives as usize,
        }
    }

    pub fn remove_image(&mut self) {
        self.lives = self.lives.saturating_sub(1);
    }

    /// Resets lives.
    pub fn reset_lives(&mut self, num_lives: u32) {
        self.lives = num_lives as usize;
    }

    pub fn lives(&self) -> usize {
        self.lives
    }

    pub fn image(&self) -> Arc<RenderedImage> {
        self.image.clone()
    }
}

impl MazeSprites {
    pub fn new() -> Self {
        Self {
            background: load_embedded_png(include_bytes!("../assets/arcade/maze-blue.png")),
            flash_background: load_embedded_png(include_bytes!("../assets/arcade/maze-flash.png")),
        }
    }

    pub fn from_layout(_layout: &str) -> Self {
        Self::new()
    }

    pub fn construct_background(&self, _level: u32) -> Arc<RenderedImage> {
        self.background.clone()
    }

    pub fn construct_flash_background(&self) -> Arc<RenderedImage> {
        self.flash_background.clone()
    }
}

impl Default for MazeSprites {
    fn default() -> Self {
        Self::new()
    }
}

fn shared_arcade_assets() -> &'static ArcadeActorAssets {
    static ASSETS: OnceLock<ArcadeActorAssets> = OnceLock::new();
    ASSETS.get_or_init(|| ArcadeActorAssets {
        pacman_left: load_embedded_png(include_bytes!("../assets/arcade/pacman-left.png")),
        pacman_right: load_embedded_png(include_bytes!("../assets/arcade/pacman-right.png")),
        pacman_up: load_embedded_png(include_bytes!("../assets/arcade/pacman-up.png")),
        pacman_down: load_embedded_png(include_bytes!("../assets/arcade/pacman-down.png")),
        pacman_closed: load_embedded_png(include_bytes!("../assets/arcade/pacman-closed.png")),
        pacman_death: [
            load_embedded_png(include_bytes!("../assets/arcade/pacman-death-0.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pacman-death-1.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pacman-death-2.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pacman-death-3.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pacman-death-4.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pacman-death-5.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pacman-death-6.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pacman-death-7.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pacman-death-8.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pacman-death-9.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pacman-death-10.png")),
        ],
        ghost_left: [
            load_embedded_png(include_bytes!("../assets/arcade/blinky-left.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pinky-left.png")),
            load_embedded_png(include_bytes!("../assets/arcade/inky-left.png")),
            load_embedded_png(include_bytes!("../assets/arcade/clyde-left.png")),
        ],
        ghost_down: [
            load_embedded_png(include_bytes!("../assets/arcade/blinky-down.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pinky-down.png")),
            load_embedded_png(include_bytes!("../assets/arcade/inky-down.png")),
            load_embedded_png(include_bytes!("../assets/arcade/clyde-down.png")),
        ],
        ghost_right: [
            load_embedded_png(include_bytes!("../assets/arcade/blinky-right.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pinky-right.png")),
            load_embedded_png(include_bytes!("../assets/arcade/inky-right.png")),
            load_embedded_png(include_bytes!("../assets/arcade/clyde-right.png")),
        ],
        ghost_up: [
            load_embedded_png(include_bytes!("../assets/arcade/blinky-up.png")),
            load_embedded_png(include_bytes!("../assets/arcade/pinky-up.png")),
            load_embedded_png(include_bytes!("../assets/arcade/inky-up.png")),
            load_embedded_png(include_bytes!("../assets/arcade/clyde-up.png")),
        ],
        ghost_eyes_up: load_embedded_png(include_bytes!("../assets/arcade/ghost-eyes-up.png")),
        ghost_eyes_down: load_embedded_png(include_bytes!("../assets/arcade/ghost-eyes-down.png")),
        ghost_eyes_left: load_embedded_png(include_bytes!("../assets/arcade/ghost-eyes-left.png")),
        ghost_eyes_right: load_embedded_png(include_bytes!(
            "../assets/arcade/ghost-eyes-right.png"
        )),
        ghost_freight: [
            load_embedded_png(include_bytes!("../assets/arcade/ghost-freight-0.png")),
            load_embedded_png(include_bytes!("../assets/arcade/ghost-freight-1.png")),
        ],
        ghost_freight_flash: [
            load_embedded_png(include_bytes!("../assets/arcade/ghost-freight-flash-0.png")),
            load_embedded_png(include_bytes!("../assets/arcade/ghost-freight-flash-1.png")),
        ],
        fruit_items: [
            load_embedded_png(include_bytes!("../assets/arcade/fruit-item-cherry.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-item-strawberry.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-item-peach.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-item-apple.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-item-grape.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-item-galaxian.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-item-bell.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-item-key.png")),
        ],
        fruit_icons: [
            load_embedded_png(include_bytes!("../assets/arcade/fruit-icon-cherry.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-icon-strawberry.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-icon-peach.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-icon-apple.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-icon-grape.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-icon-galaxian.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-icon-bell.png")),
            load_embedded_png(include_bytes!("../assets/arcade/fruit-icon-key.png")),
        ],
    })
}

/// Loads embedded png.
fn load_embedded_png(bytes: &'static [u8]) -> Arc<RenderedImage> {
    Arc::new(decode_png_image(bytes).expect("embedded png should decode correctly"))
}

fn decode_png_image(bytes: &[u8]) -> anyhow::Result<RenderedImage> {
    let mut decoder = Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
    let mut reader = decoder.read_info().context("read png metadata")?;
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .expect("png decoder should know output buffer size")
    ];
    let info = reader
        .next_frame(&mut buffer)
        .context("decode png pixels")?;
    let raw = &buffer[..info.buffer_size()];
    let mut rgba = match info.color_type {
        ColorType::Rgba => raw.to_vec(),
        ColorType::Rgb => rgb_to_rgba(raw),
        ColorType::Grayscale => grayscale_to_rgba(raw),
        ColorType::GrayscaleAlpha => grayscale_alpha_to_rgba(raw),
        ColorType::Indexed => unreachable!("expanded indexed PNG should not remain indexed"),
    };

    let transparent = rgba[..4].to_vec();
    for chunk in rgba.chunks_exact_mut(4) {
        if chunk[..3] == transparent[..3] {
            chunk[3] = 0;
        }
    }

    Ok(RenderedImage {
        width: info.width,
        height: info.height,
        pixels: rgba,
    })
}

fn rgb_to_rgba(raw: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(raw.len() / 3 * 4);
    for chunk in raw.chunks_exact(3) {
        rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
    }
    rgba
}

fn grayscale_to_rgba(raw: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(raw.len() * 4);
    for value in raw {
        rgba.extend_from_slice(&[*value, *value, *value, 255]);
    }
    rgba
}

fn grayscale_alpha_to_rgba(raw: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(raw.len() / 2 * 4);
    for chunk in raw.chunks_exact(2) {
        rgba.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
    }
    rgba
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{LifeSprites, MazeSprites, PacmanSprites};
    use crate::{pacman::Direction, render::RenderedImage};

    fn mirror(image: &RenderedImage) -> RenderedImage {
        let mut pixels = vec![0; image.pixels.len()];
        let width = image.width as usize;
        let height = image.height as usize;
        let stride = width * 4;
        for y in 0..height {
            let row_start = y * stride;
            for x in 0..width {
                let src = row_start + x * 4;
                let dst = row_start + (width - 1 - x) * 4;
                pixels[dst..dst + 4].copy_from_slice(&image.pixels[src..src + 4]);
            }
        }

        RenderedImage {
            width: image.width,
            height: image.height,
            pixels,
        }
    }

    fn flip(image: &RenderedImage) -> RenderedImage {
        let mut pixels = vec![0; image.pixels.len()];
        let width = image.width as usize;
        let height = image.height as usize;
        let stride = width * 4;
        for y in 0..height {
            let src_row_start = y * stride;
            let dst_row_start = (height - 1 - y) * stride;
            pixels[dst_row_start..dst_row_start + stride]
                .copy_from_slice(&image.pixels[src_row_start..src_row_start + stride]);
        }

        RenderedImage {
            width: image.width,
            height: image.height,
            pixels,
        }
    }

    #[test]
    fn pacman_sprites_use_arcade_sprite_dimensions() {
        let sprites = PacmanSprites::new();
        let image = sprites.current();

        assert_eq!(image.width, 32);
        assert_eq!(image.height, 32);
    }

    #[test]
    fn maze_background_matches_screen_size() {
        let maze = MazeSprites::new();
        let image = maze.construct_background(1);

        assert_eq!(image.width, crate::constants::SCREEN_WIDTH);
        assert_eq!(image.height, crate::constants::SCREEN_HEIGHT);
    }

    #[test]
    fn pacman_death_animation_advances_to_a_different_frame() {
        let mut sprites = PacmanSprites::new();

        let before = sprites.current();
        let after = sprites.update_for_state(0.2, crate::pacman::Direction::Stop, false);

        assert_ne!(before.pixels, after.pixels);
    }

    #[test]
    fn pacman_directional_frames_match_right_and_down_mirrors() {
        let right = Arc::unwrap_or_clone(PacmanSprites::new().update_for_state(
            0.1,
            Direction::Right,
            true,
        ));
        let left =
            Arc::unwrap_or_clone(PacmanSprites::new().update_for_state(0.1, Direction::Left, true));
        let down =
            Arc::unwrap_or_clone(PacmanSprites::new().update_for_state(0.1, Direction::Down, true));
        let up =
            Arc::unwrap_or_clone(PacmanSprites::new().update_for_state(0.1, Direction::Up, true));

        assert_ne!(left.pixels, right.pixels);
        assert_ne!(up.pixels, down.pixels);
        assert_eq!(left.pixels, mirror(&right).pixels);
        assert_eq!(up.pixels, flip(&down).pixels);
    }

    #[test]
    fn fruit_sprites_cycle_by_level() {
        let sprites = super::FruitSprites::new();

        assert_eq!(
            sprites.image_for_level(0).pixels,
            sprites.item_image(0).pixels
        );
        assert_eq!(
            sprites.image_for_level(7).pixels,
            sprites.item_image(7).pixels
        );
        assert_eq!(sprites.icon_image(7).width, 32);
        assert_eq!(sprites.icon_image(7).height, 32);
    }

    #[test]
    fn life_sprites_track_remaining_lives() {
        let mut lives = LifeSprites::new(5);
        lives.remove_image();

        assert_eq!(lives.lives(), 4);
    }
}
