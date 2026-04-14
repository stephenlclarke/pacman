use crate::{actors::GhostKind, pacman::Direction};

type TilePosition = (f32, f32);
type PortalPair = (TilePosition, TilePosition);

const MAZE1: &str = include_str!("../assets/maze1.txt");
const MAZE1_ROTATION: &str = include_str!("../assets/maze1_rotation.txt");
const MAZE2: &str = include_str!("../assets/maze2.txt");
const MAZE2_ROTATION: &str = include_str!("../assets/maze2_rotation.txt");

#[derive(Clone, Copy, Debug)]
pub struct MazeSpec {
    pub name: &'static str,
    pub layout: &'static str,
    pub rotation: &'static str,
    pub portal_pairs: &'static [PortalPair],
    pub home_offset: TilePosition,
    pub home_connect_left: TilePosition,
    pub home_connect_right: TilePosition,
    pub pacman_start: TilePosition,
    pub fruit_start: TilePosition,
    pub ghost_deny_up: &'static [TilePosition],
}

impl MazeSpec {
    pub fn for_level(level: u32, allow_multiple_mazes: bool) -> Self {
        if allow_multiple_mazes && level.is_multiple_of(2) {
            maze2()
        } else {
            maze1()
        }
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

    pub fn deny_ghost_access_positions(self) -> [(Direction, (f32, f32)); 2] {
        [
            (Direction::Left, self.add_offset(2.0, 3.0)),
            (Direction::Right, self.add_offset(2.0, 3.0)),
        ]
    }

    pub fn inky_start_restriction(self) -> (Direction, (f32, f32), GhostKind) {
        (Direction::Right, self.inky_start(), GhostKind::Inky)
    }

    pub fn clyde_start_restriction(self) -> (Direction, (f32, f32), GhostKind) {
        (Direction::Left, self.clyde_start(), GhostKind::Clyde)
    }
}

fn maze1() -> MazeSpec {
    MazeSpec {
        name: "maze1",
        layout: MAZE1,
        rotation: MAZE1_ROTATION,
        portal_pairs: &[((0.0, 17.0), (27.0, 17.0))],
        home_offset: (11.5, 14.0),
        home_connect_left: (12.0, 14.0),
        home_connect_right: (15.0, 14.0),
        pacman_start: (15.0, 26.0),
        fruit_start: (9.0, 20.0),
        ghost_deny_up: &[(12.0, 14.0), (15.0, 14.0), (12.0, 26.0), (15.0, 26.0)],
    }
}

fn maze2() -> MazeSpec {
    MazeSpec {
        name: "maze2",
        layout: MAZE2,
        rotation: MAZE2_ROTATION,
        portal_pairs: &[((0.0, 4.0), (27.0, 4.0)), ((0.0, 26.0), (27.0, 26.0))],
        home_offset: (11.5, 14.0),
        home_connect_left: (9.0, 14.0),
        home_connect_right: (18.0, 14.0),
        pacman_start: (16.0, 26.0),
        fruit_start: (11.0, 20.0),
        ghost_deny_up: &[(9.0, 14.0), (18.0, 14.0), (11.0, 23.0), (16.0, 23.0)],
    }
}

#[cfg(test)]
mod tests {
    use super::MazeSpec;

    #[test]
    fn maze_data_cycles_between_the_two_layouts() {
        assert_eq!(MazeSpec::for_level(1, true).name, "maze1");
        assert_eq!(MazeSpec::for_level(2, true).name, "maze2");
        assert_eq!(MazeSpec::for_level(3, true).name, "maze1");
    }

    #[test]
    fn single_maze_mode_stays_on_maze_one() {
        assert_eq!(MazeSpec::for_level(2, false).name, "maze1");
    }
}
