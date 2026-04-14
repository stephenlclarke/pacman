pub const TILE_WIDTH: u32 = 16;
pub const TILE_HEIGHT: u32 = 16;
pub const NROWS: u32 = 36;
pub const NCOLS: u32 = 28;
pub const SCREEN_WIDTH: u32 = NCOLS * TILE_WIDTH;
pub const SCREEN_HEIGHT: u32 = NROWS * TILE_HEIGHT;

pub const PACMAN_START_X: f32 = 200.0;
pub const PACMAN_START_Y: f32 = 400.0;
pub const PACMAN_RADIUS: f32 = 10.0;
pub const PACMAN_COLLIDE_RADIUS: f32 = 5.0;
pub const PACMAN_SPEED: f32 = 100.0 * TILE_WIDTH as f32 / 16.0;
pub const PELLET_RADIUS: f32 = 4.0 * TILE_WIDTH as f32 / 16.0;
pub const POWER_PELLET_RADIUS: f32 = 8.0 * TILE_WIDTH as f32 / 16.0;
pub const POWER_PELLET_FLASH_TIME: f32 = 0.2;

pub const BLACK: [u8; 4] = [0, 0, 0, 255];
pub const YELLOW: [u8; 4] = [255, 255, 0, 255];
pub const WHITE: [u8; 4] = [255, 255, 255, 255];
pub const RED: [u8; 4] = [255, 0, 0, 255];
pub const PINK: [u8; 4] = [255, 100, 150, 255];
pub const TEAL: [u8; 4] = [100, 255, 255, 255];
pub const ORANGE: [u8; 4] = [230, 190, 40, 255];
pub const GREEN: [u8; 4] = [0, 255, 0, 255];
