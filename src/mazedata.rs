//! Loads the embedded arcade maze metadata and exposes the runtime maze specification.

use crate::{actors::GhostKind, pacman::Direction};

type TilePosition = (f32, f32);
type PortalPair = (TilePosition, TilePosition);

const ARCADE_MAZE_LAYOUT: &str = include_str!("../assets/arcade/maze-logic.txt");
const ARCADE_MAZE_METADATA: &str = include_str!("../assets/arcade/maze-metadata.txt");

#[derive(Clone, Copy, Debug)]
pub struct MazeSpec {
    pub layout: &'static str,
    pub portal_pairs: [PortalPair; 1],
    pub home_offset: TilePosition,
    pub home_connect_left: TilePosition,
    pub home_connect_right: TilePosition,
    pub blinky_start_pixels: TilePosition,
    pub pinky_start_pixels: TilePosition,
    pub inky_start_pixels: TilePosition,
    pub clyde_start_pixels: TilePosition,
    pub pacman_start: TilePosition,
    pub fruit_start: TilePosition,
    pub pacman_start_pixels: TilePosition,
    pub fruit_start_pixels: TilePosition,
    pub ghost_deny_up: [TilePosition; 4],
}

impl MazeSpec {
    pub fn arcade() -> Self {
        arcade_maze()
    }

    pub fn add_offset(self, x: f32, y: f32) -> (f32, f32) {
        (x + self.home_offset.0, y + self.home_offset.1)
    }

    pub fn blinky_start(self) -> (f32, f32) {
        self.add_offset(2.0, 0.0)
    }

    pub fn pinky_start(self) -> (f32, f32) {
        self.add_offset(2.0, 3.0)
    }

    pub fn inky_start(self) -> (f32, f32) {
        self.add_offset(0.0, 3.0)
    }

    pub fn clyde_start(self) -> (f32, f32) {
        self.add_offset(4.0, 3.0)
    }

    pub fn spawn_node(self) -> (f32, f32) {
        self.add_offset(2.0, 3.0)
    }

    /// Denies ghost access positions.
    pub fn deny_ghost_access_positions(self) -> [(Direction, (f32, f32)); 2] {
        [
            (Direction::Left, self.add_offset(2.0, 3.0)),
            (Direction::Right, self.add_offset(2.0, 3.0)),
        ]
    }

    pub fn inky_start_restriction(self) -> (Direction, (f32, f32), GhostKind) {
        (Direction::Right, self.inky_start(), GhostKind::Inky)
    }

    pub fn pinky_start_restriction(self) -> (Direction, (f32, f32), GhostKind) {
        (Direction::Up, self.pinky_start(), GhostKind::Pinky)
    }

    pub fn clyde_start_restriction(self) -> (Direction, (f32, f32), GhostKind) {
        (Direction::Left, self.clyde_start(), GhostKind::Clyde)
    }
}

fn arcade_maze() -> MazeSpec {
    let metadata = arcade_maze_metadata();
    MazeSpec {
        layout: ARCADE_MAZE_LAYOUT,
        portal_pairs: [metadata.portal_pair],
        home_offset: metadata.home_offset,
        home_connect_left: metadata.home_connect_left,
        home_connect_right: metadata.home_connect_right,
        blinky_start_pixels: metadata.blinky_start_pixels,
        pinky_start_pixels: metadata.pinky_start_pixels,
        inky_start_pixels: metadata.inky_start_pixels,
        clyde_start_pixels: metadata.clyde_start_pixels,
        pacman_start: metadata.pacman_start,
        fruit_start: metadata.fruit_start,
        pacman_start_pixels: metadata.pacman_start_pixels,
        fruit_start_pixels: metadata.fruit_start_pixels,
        ghost_deny_up: metadata.ghost_deny_up,
    }
}

#[derive(Clone, Copy, Debug)]
struct MazeMetadata {
    portal_pair: PortalPair,
    home_offset: TilePosition,
    home_connect_left: TilePosition,
    home_connect_right: TilePosition,
    blinky_start_pixels: TilePosition,
    pinky_start_pixels: TilePosition,
    inky_start_pixels: TilePosition,
    clyde_start_pixels: TilePosition,
    pacman_start: TilePosition,
    fruit_start: TilePosition,
    pacman_start_pixels: TilePosition,
    fruit_start_pixels: TilePosition,
    ghost_deny_up: [TilePosition; 4],
}

fn arcade_maze_metadata() -> MazeMetadata {
    parse_maze_metadata(ARCADE_MAZE_METADATA)
}

/// Parses maze metadata.
fn parse_maze_metadata(text: &str) -> MazeMetadata {
    let mut portal_pair = None;
    let mut home_offset = None;
    let mut home_connect_left = None;
    let mut home_connect_right = None;
    let mut blinky_start_pixels = None;
    let mut pinky_start_pixels = None;
    let mut inky_start_pixels = None;
    let mut clyde_start_pixels = None;
    let mut pacman_start = None;
    let mut fruit_start = None;
    let mut pacman_start_pixels = None;
    let mut fruit_start_pixels = None;
    let mut ghost_deny_up = None;

    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let (key, value) = line
            .split_once('=')
            .expect("maze metadata lines should use key=value");
        match key {
            "portal_pair" => portal_pair = Some(parse_portal_pair(value)),
            "home_offset" => home_offset = Some(parse_position(value)),
            "home_connect_left" => home_connect_left = Some(parse_position(value)),
            "home_connect_right" => home_connect_right = Some(parse_position(value)),
            "blinky_start_pixels" => blinky_start_pixels = Some(parse_position(value)),
            "pinky_start_pixels" => pinky_start_pixels = Some(parse_position(value)),
            "inky_start_pixels" => inky_start_pixels = Some(parse_position(value)),
            "clyde_start_pixels" => clyde_start_pixels = Some(parse_position(value)),
            "pacman_start" => pacman_start = Some(parse_position(value)),
            "fruit_start" => fruit_start = Some(parse_position(value)),
            "pacman_start_pixels" => pacman_start_pixels = Some(parse_position(value)),
            "fruit_start_pixels" => fruit_start_pixels = Some(parse_position(value)),
            "ghost_deny_up" => ghost_deny_up = Some(parse_position_list(value)),
            _ => {}
        }
    }

    MazeMetadata {
        portal_pair: portal_pair.expect("maze metadata should define portal_pair"),
        home_offset: home_offset.expect("maze metadata should define home_offset"),
        home_connect_left: home_connect_left
            .expect("maze metadata should define home_connect_left"),
        home_connect_right: home_connect_right
            .expect("maze metadata should define home_connect_right"),
        blinky_start_pixels: blinky_start_pixels
            .expect("maze metadata should define blinky_start_pixels"),
        pinky_start_pixels: pinky_start_pixels
            .expect("maze metadata should define pinky_start_pixels"),
        inky_start_pixels: inky_start_pixels
            .expect("maze metadata should define inky_start_pixels"),
        clyde_start_pixels: clyde_start_pixels
            .expect("maze metadata should define clyde_start_pixels"),
        pacman_start: pacman_start.expect("maze metadata should define pacman_start"),
        fruit_start: fruit_start.expect("maze metadata should define fruit_start"),
        pacman_start_pixels: pacman_start_pixels
            .expect("maze metadata should define pacman_start_pixels"),
        fruit_start_pixels: fruit_start_pixels
            .expect("maze metadata should define fruit_start_pixels"),
        ghost_deny_up: ghost_deny_up.expect("maze metadata should define ghost_deny_up"),
    }
}

/// Parses portal pair.
fn parse_portal_pair(value: &str) -> PortalPair {
    let (left, right) = value
        .split_once('|')
        .expect("portal pairs should contain exactly two positions");
    (parse_position(left), parse_position(right))
}

/// Parses position list.
fn parse_position_list(value: &str) -> [TilePosition; 4] {
    value
        .split(';')
        .map(parse_position)
        .collect::<Vec<_>>()
        .try_into()
        .expect("maze metadata should provide four ghost deny-up positions")
}

/// Parses position.
fn parse_position(value: &str) -> TilePosition {
    let (x, y) = value
        .split_once(',')
        .expect("positions should be encoded as x,y");
    (
        x.parse::<f32>().expect("position x should parse"),
        y.parse::<f32>().expect("position y should parse"),
    )
}

#[cfg(test)]
mod tests {
    use super::{MazeSpec, arcade_maze_metadata};

    #[test]
    fn arcade_maze_spec_uses_the_embedded_layout() {
        let maze = MazeSpec::arcade();

        assert!(maze.layout.contains("P"));
        assert_eq!(maze.portal_pairs, [((0.0, 17.0), (27.0, 17.0))]);
    }

    #[test]
    fn extracted_arcade_metadata_matches_expected_positions() {
        let metadata = arcade_maze_metadata();

        assert_eq!(metadata.portal_pair, ((0.0, 17.0), (27.0, 17.0)));
        assert_eq!(metadata.home_offset, (11.5, 14.0));
        assert_eq!(metadata.home_connect_left, (12.0, 14.0));
        assert_eq!(metadata.home_connect_right, (15.0, 14.0));
        assert_eq!(metadata.blinky_start_pixels, (216.0, 224.0));
        assert_eq!(metadata.pinky_start_pixels, (216.0, 272.0));
        assert_eq!(metadata.inky_start_pixels, (184.0, 272.0));
        assert_eq!(metadata.clyde_start_pixels, (248.0, 272.0));
        assert_eq!(metadata.pacman_start, (15.0, 26.0));
        assert_eq!(metadata.fruit_start, (15.0, 23.0));
        assert_eq!(metadata.pacman_start_pixels, (216.0, 416.0));
        assert_eq!(metadata.fruit_start_pixels, (216.0, 320.0));
        assert_eq!(
            metadata.ghost_deny_up,
            [(12.0, 14.0), (15.0, 14.0), (12.0, 26.0), (15.0, 26.0)]
        );
    }
}
