use std::{
    io::Cursor,
    sync::{Arc, OnceLock},
};

use anyhow::Context;
use png::{ColorType, Decoder, Transformations};

use crate::{
    actors::GhostKind,
    animation::Animator,
    constants::{BLACK, SCREEN_HEIGHT, SCREEN_WIDTH, TILE_HEIGHT, TILE_WIDTH},
    modes::GhostMode,
    pacman::Direction,
    render::RenderedImage,
};

const SPRITESHEET_BYTES: &[u8] = include_bytes!("../assets/spritesheet.png");
const MAZE_FILE: &str = include_str!("../assets/maze1.txt");
const MAZE_ROTATIONS: &str = include_str!("../assets/maze1_rotation.txt");
const FRUIT_TILE_POSITIONS: [(u32, u32); 6] =
    [(16, 8), (18, 8), (20, 8), (16, 10), (18, 10), (20, 10)];
const FLASH_BACKGROUND_ROW: u32 = 5;

#[derive(Clone, Debug)]
struct SpriteSheet {
    image: RenderedImage,
}

#[derive(Clone, Debug)]
pub struct PacmanSprites {
    left: Animator<Arc<RenderedImage>>,
    right: Animator<Arc<RenderedImage>>,
    up: Animator<Arc<RenderedImage>>,
    down: Animator<Arc<RenderedImage>>,
    death: Animator<Arc<RenderedImage>>,
    stop_left: Arc<RenderedImage>,
    stop_right: Arc<RenderedImage>,
    stop_up: Arc<RenderedImage>,
    stop_down: Arc<RenderedImage>,
    stop_image: Arc<RenderedImage>,
    current: Arc<RenderedImage>,
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
    freight: Arc<RenderedImage>,
}

#[derive(Clone, Debug)]
pub struct FruitSprites {
    images: [Arc<RenderedImage>; 6],
}

#[derive(Clone, Debug)]
pub struct LifeSprites {
    image: Arc<RenderedImage>,
    lives: usize,
}

#[derive(Clone, Debug)]
pub struct MazeSprites {
    data: Vec<Vec<char>>,
    rotations: Vec<Vec<char>>,
}

impl PacmanSprites {
    pub fn new() -> Self {
        let sheet = shared_sheet();
        let stop_left = sheet.crop(8, 0, 2, 2);
        let stop_right = sheet.crop(10, 0, 2, 2);
        let stop_up = sheet.crop(10, 2, 2, 2);
        let stop_down = sheet.crop(8, 2, 2, 2);

        Self {
            left: Animator::new(
                vec![
                    stop_left.clone(),
                    sheet.crop(0, 0, 2, 2),
                    sheet.crop(0, 2, 2, 2),
                    sheet.crop(0, 0, 2, 2),
                ],
                20.0,
                true,
            ),
            right: Animator::new(
                vec![
                    stop_right.clone(),
                    sheet.crop(2, 0, 2, 2),
                    sheet.crop(2, 2, 2, 2),
                    sheet.crop(2, 0, 2, 2),
                ],
                20.0,
                true,
            ),
            up: Animator::new(
                vec![
                    stop_up.clone(),
                    sheet.crop(6, 0, 2, 2),
                    sheet.crop(6, 2, 2, 2),
                    sheet.crop(6, 0, 2, 2),
                ],
                20.0,
                true,
            ),
            down: Animator::new(
                vec![
                    stop_down.clone(),
                    sheet.crop(4, 0, 2, 2),
                    sheet.crop(4, 2, 2, 2),
                    sheet.crop(4, 0, 2, 2),
                ],
                20.0,
                true,
            ),
            death: Animator::new(
                (0..=10)
                    .map(|index| sheet.crop(index * 2, 12, 2, 2))
                    .collect(),
                6.0,
                false,
            ),
            stop_left: stop_left.clone(),
            stop_right,
            stop_up,
            stop_down,
            stop_image: stop_left.clone(),
            current: stop_left,
        }
    }

    pub fn reset(&mut self) {
        self.left.reset();
        self.right.reset();
        self.up.reset();
        self.down.reset();
        self.death.reset();
        self.stop_image = self.stop_left.clone();
        self.current = self.stop_left.clone();
    }

    pub fn update(&mut self, dt: f32, direction: Direction) -> Arc<RenderedImage> {
        self.update_for_state(dt, direction, true)
    }

    pub fn update_for_state(
        &mut self,
        dt: f32,
        direction: Direction,
        alive: bool,
    ) -> Arc<RenderedImage> {
        self.current = if alive {
            match direction {
                Direction::Left => {
                    self.stop_image = self.stop_left.clone();
                    self.left.update(dt)
                }
                Direction::Right => {
                    self.stop_image = self.stop_right.clone();
                    self.right.update(dt)
                }
                Direction::Up => {
                    self.stop_image = self.stop_up.clone();
                    self.up.update(dt)
                }
                Direction::Down => {
                    self.stop_image = self.stop_down.clone();
                    self.down.update(dt)
                }
                Direction::Stop => self.stop_image.clone(),
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
        let sheet = shared_sheet();
        let start = std::array::from_fn(|index| sheet.crop(x_offset(index), 4, 2, 2));
        let up = std::array::from_fn(|index| sheet.crop(x_offset(index), 4, 2, 2));
        let down = std::array::from_fn(|index| sheet.crop(x_offset(index), 6, 2, 2));
        let left = std::array::from_fn(|index| sheet.crop(x_offset(index), 8, 2, 2));
        let right = std::array::from_fn(|index| sheet.crop(x_offset(index), 10, 2, 2));

        Self {
            start,
            up,
            down,
            left,
            right,
            spawn_up: sheet.crop(8, 4, 2, 2),
            spawn_down: sheet.crop(8, 6, 2, 2),
            spawn_left: sheet.crop(8, 8, 2, 2),
            spawn_right: sheet.crop(8, 10, 2, 2),
            freight: sheet.crop(10, 4, 2, 2),
        }
    }

    pub fn image(
        &self,
        kind: GhostKind,
        mode: GhostMode,
        direction: Direction,
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
            GhostMode::Freight => self.freight.clone(),
            GhostMode::Spawn => match direction {
                Direction::Up => self.spawn_up.clone(),
                Direction::Down => self.spawn_down.clone(),
                Direction::Left => self.spawn_left.clone(),
                Direction::Right => self.spawn_right.clone(),
                Direction::Stop => self.spawn_up.clone(),
            },
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
        Self {
            images: FRUIT_TILE_POSITIONS.map(|(x, y)| shared_sheet().crop(x, y, 2, 2)),
        }
    }

    pub fn image(&self, index: usize) -> Arc<RenderedImage> {
        self.images[index % self.images.len()].clone()
    }

    pub fn image_for_level(&self, level_index: u32) -> Arc<RenderedImage> {
        self.image(level_index as usize)
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
            image: shared_sheet().crop(0, 0, 2, 2),
            lives: num_lives as usize,
        }
    }

    pub fn remove_image(&mut self) {
        self.lives = self.lives.saturating_sub(1);
    }

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
        Self::from_layout(MAZE_FILE, MAZE_ROTATIONS)
    }

    pub fn from_layout(layout: &str, rotations: &str) -> Self {
        Self {
            data: parse_grid(layout),
            rotations: parse_grid(rotations),
        }
    }

    pub fn construct_background(&self, level: u32) -> Arc<RenderedImage> {
        self.construct_background_row(level.saturating_sub(1) % 5)
    }

    pub fn construct_flash_background(&self) -> Arc<RenderedImage> {
        self.construct_background_row(FLASH_BACKGROUND_ROW)
    }

    pub fn construct_background_row(&self, sprite_row: u32) -> Arc<RenderedImage> {
        let sheet = shared_sheet();
        let mut background = solid_image(SCREEN_WIDTH, SCREEN_HEIGHT, BLACK);

        for (row, tiles) in self.data.iter().enumerate() {
            for (col, tile) in tiles.iter().copied().enumerate() {
                let Some(x) = maze_tile_sprite_x(tile) else {
                    if tile == '=' {
                        let sprite = sheet.crop(10, 8, 1, 1);
                        blit_image(
                            &mut background,
                            &sprite,
                            col as u32 * TILE_WIDTH,
                            row as u32 * TILE_HEIGHT,
                        );
                    }
                    continue;
                };

                let rotation = self
                    .rotations
                    .get(row)
                    .and_then(|rotation_row| rotation_row.get(col))
                    .and_then(|value| value.to_digit(10))
                    .unwrap_or(0) as u8;
                let sprite = rotate_image(&sheet.crop(x, sprite_row, 1, 1), rotation);
                blit_image(
                    &mut background,
                    &sprite,
                    col as u32 * TILE_WIDTH,
                    row as u32 * TILE_HEIGHT,
                );
            }
        }

        Arc::new(background)
    }
}

impl Default for MazeSprites {
    fn default() -> Self {
        Self::new()
    }
}

impl SpriteSheet {
    fn load() -> anyhow::Result<Self> {
        let mut decoder = Decoder::new(Cursor::new(SPRITESHEET_BYTES));
        decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
        let mut reader = decoder.read_info().context("read spritesheet metadata")?;
        let mut buffer = vec![
            0;
            reader
                .output_buffer_size()
                .expect("png decoder should know output buffer size")
        ];
        let info = reader
            .next_frame(&mut buffer)
            .context("decode spritesheet pixels")?;
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

        Ok(Self {
            image: RenderedImage {
                width: info.width,
                height: info.height,
                pixels: rgba,
            },
        })
    }

    fn crop(&self, tile_x: u32, tile_y: u32, tiles_w: u32, tiles_h: u32) -> Arc<RenderedImage> {
        let x = tile_x * TILE_WIDTH;
        let y = tile_y * TILE_HEIGHT;
        let width = tiles_w * TILE_WIDTH;
        let height = tiles_h * TILE_HEIGHT;
        let mut pixels = vec![0; width as usize * height as usize * 4];

        for row in 0..height {
            for col in 0..width {
                let src_x = x + col;
                let src_y = y + row;
                let src_index = ((src_y * self.image.width + src_x) as usize).saturating_mul(4);
                let dst_index = ((row * width + col) as usize).saturating_mul(4);
                pixels[dst_index..dst_index + 4]
                    .copy_from_slice(&self.image.pixels[src_index..src_index + 4]);
            }
        }

        Arc::new(RenderedImage {
            width,
            height,
            pixels,
        })
    }
}

fn shared_sheet() -> Arc<SpriteSheet> {
    static SHEET: OnceLock<Arc<SpriteSheet>> = OnceLock::new();
    SHEET
        .get_or_init(|| {
            Arc::new(SpriteSheet::load().expect("embedded spritesheet should decode correctly"))
        })
        .clone()
}

fn x_offset(index: usize) -> u32 {
    [0, 2, 4, 6][index]
}

fn parse_grid(text: &str) -> Vec<Vec<char>> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split_whitespace()
                .map(|cell| {
                    cell.chars()
                        .next()
                        .expect("maze cells should contain a symbol")
                })
                .collect()
        })
        .collect()
}

fn solid_image(width: u32, height: u32, color: [u8; 4]) -> RenderedImage {
    let mut pixels = vec![0; width as usize * height as usize * 4];
    for chunk in pixels.chunks_exact_mut(4) {
        chunk.copy_from_slice(&color);
    }
    RenderedImage {
        width,
        height,
        pixels,
    }
}

fn maze_tile_sprite_x(tile: char) -> Option<u32> {
    tile.to_digit(10).map(|value| value + 12)
}

fn rotate_image(image: &Arc<RenderedImage>, quarter_turns: u8) -> RenderedImage {
    let mut rotated = image.as_ref().clone();
    for _ in 0..quarter_turns % 4 {
        rotated = rotate_once_counterclockwise(&rotated);
    }
    rotated
}

fn rotate_once_counterclockwise(image: &RenderedImage) -> RenderedImage {
    let mut rotated = vec![0; image.pixels.len()];
    let width = image.width as usize;
    let height = image.height as usize;

    for y in 0..height {
        for x in 0..width {
            let src_index = (y * width + x) * 4;
            let dest_x = y;
            let dest_y = width - 1 - x;
            let dest_index = (dest_y * height + dest_x) * 4;
            rotated[dest_index..dest_index + 4]
                .copy_from_slice(&image.pixels[src_index..src_index + 4]);
        }
    }

    RenderedImage {
        width: image.height,
        height: image.width,
        pixels: rotated,
    }
}

fn blit_image(target: &mut RenderedImage, sprite: &RenderedImage, x: u32, y: u32) {
    for row in 0..sprite.height {
        for col in 0..sprite.width {
            let dst_x = x + col;
            let dst_y = y + row;
            if dst_x >= target.width || dst_y >= target.height {
                continue;
            }

            let src_index = ((row * sprite.width + col) as usize) * 4;
            let alpha = sprite.pixels[src_index + 3];
            if alpha == 0 {
                continue;
            }

            let dst_index = ((dst_y * target.width + dst_x) as usize) * 4;
            target.pixels[dst_index..dst_index + 4]
                .copy_from_slice(&sprite.pixels[src_index..src_index + 4]);
        }
    }
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

    use super::{
        LifeSprites, MazeSprites, PacmanSprites, RenderedImage, rotate_image, shared_sheet,
    };

    #[test]
    fn spritesheet_decodes_successfully() {
        let sheet = shared_sheet();
        assert!(sheet.image.width > 0);
        assert!(sheet.image.height > 0);
    }

    #[test]
    fn pacman_sprites_use_two_tile_images() {
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
    fn fruit_sprites_cycle_by_level() {
        let sprites = super::FruitSprites::new();

        assert_eq!(sprites.image_for_level(0).pixels, sprites.image(0).pixels);
        assert_eq!(sprites.image_for_level(7).pixels, sprites.image(1).pixels);
    }

    #[test]
    fn life_sprites_track_remaining_lives() {
        let mut lives = LifeSprites::new(5);
        lives.remove_image();

        assert_eq!(lives.lives(), 4);
    }

    #[test]
    fn quarter_turns_rotate_counterclockwise() {
        let image = Arc::new(RenderedImage {
            width: 2,
            height: 3,
            pixels: vec![
                1, 0, 0, 255, 2, 0, 0, 255, 3, 0, 0, 255, 4, 0, 0, 255, 5, 0, 0, 255, 6, 0, 0, 255,
            ],
        });

        let rotated = rotate_image(&image, 1);
        let red_values: Vec<u8> = rotated
            .pixels
            .chunks_exact(4)
            .map(|pixel| pixel[0])
            .collect();

        assert_eq!(rotated.width, 3);
        assert_eq!(rotated.height, 2);
        assert_eq!(red_values, vec![2, 4, 6, 1, 3, 5]);
    }
}
