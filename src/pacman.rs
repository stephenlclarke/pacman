use crate::{
    constants::{PACMAN_RADIUS, PACMAN_SPEED, PACMAN_START_X, PACMAN_START_Y, YELLOW},
    vector::Vector2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityKind {
    Pacman,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    #[default]
    Stop,
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn vector(self) -> Vector2 {
        match self {
            Self::Stop => Vector2::default(),
            Self::Up => Vector2::new(0.0, -1.0),
            Self::Down => Vector2::new(0.0, 1.0),
            Self::Left => Vector2::new(-1.0, 0.0),
            Self::Right => Vector2::new(1.0, 0.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pacman {
    pub kind: EntityKind,
    position: Vector2,
    direction: Direction,
    speed: f32,
    radius: f32,
    color: [u8; 4],
}

impl Pacman {
    pub fn new() -> Self {
        Self {
            kind: EntityKind::Pacman,
            position: Vector2::new(PACMAN_START_X, PACMAN_START_Y),
            direction: Direction::Stop,
            speed: PACMAN_SPEED,
            radius: PACMAN_RADIUS,
            color: YELLOW,
        }
    }

    pub fn update(&mut self, dt: f32, next_direction: Direction) {
        self.position += self.direction.vector() * self.speed * dt;
        self.direction = next_direction;
    }

    pub fn position(&self) -> Vector2 {
        self.position
    }

    pub fn radius(&self) -> f32 {
        self.radius
    }

    pub fn color(&self) -> [u8; 4] {
        self.color
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }
}

impl Default for Pacman {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, EntityKind, Pacman};
    use crate::vector::Vector2;

    #[test]
    fn pacman_starts_with_tutorial_defaults() {
        let pacman = Pacman::new();

        assert_eq!(pacman.kind, EntityKind::Pacman);
        assert_eq!(pacman.position(), Vector2::new(200.0, 400.0));
        assert_eq!(pacman.direction(), Direction::Stop);
        assert_eq!(pacman.radius(), 10.0);
        assert_eq!(pacman.color(), [255, 255, 0, 255]);
    }

    #[test]
    fn pacman_moves_using_the_previous_frame_direction() {
        let mut pacman = Pacman::new();

        pacman.update(0.5, Direction::Right);
        assert_eq!(pacman.position(), Vector2::new(200.0, 400.0));

        pacman.update(0.5, Direction::Right);
        assert_eq!(pacman.position(), Vector2::new(250.0, 400.0));
    }
}
