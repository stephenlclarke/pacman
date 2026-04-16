//! Builds pellet layouts and handles pellet flashing, lookup, and consumption.

use crate::{
    constants::{
        PELLET_RADIUS, POWER_PELLET_FLASH_TIME, POWER_PELLET_RADIUS, TILE_HEIGHT, TILE_WIDTH, WHITE,
    },
    render::{Circle, FrameData},
    vector::Vector2,
};

#[cfg(test)]
const ARCADE_MAZE_LAYOUT: &str = include_str!("../assets/arcade/maze-logic.txt");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PelletKind {
    Pellet,
    PowerPellet,
}

#[derive(Clone, Debug)]
pub struct Pellet {
    position: Vector2,
    radius: f32,
    points: u32,
    visible: bool,
    kind: PelletKind,
    flash_time: f32,
    timer: f32,
}

#[derive(Clone, Debug, Default)]
pub struct PelletGroup {
    pellets: Vec<Pellet>,
    num_eaten: usize,
}

impl Pellet {
    /// Creates new.
    fn new(row: usize, column: usize, kind: PelletKind) -> Self {
        let position = Vector2::new(
            (column as u32 * TILE_WIDTH) as f32,
            (row as u32 * TILE_HEIGHT) as f32,
        );
        let (radius, points) = match kind {
            PelletKind::Pellet => (PELLET_RADIUS, 10),
            PelletKind::PowerPellet => (POWER_PELLET_RADIUS, 50),
        };

        Self {
            position,
            radius,
            points,
            visible: true,
            kind,
            flash_time: POWER_PELLET_FLASH_TIME,
            timer: 0.0,
        }
    }

    /// Handles position.
    pub fn position(&self) -> Vector2 {
        self.position
    }

    /// Handles points.
    pub fn points(&self) -> u32 {
        self.points
    }

    /// Handles kind.
    pub fn kind(&self) -> PelletKind {
        self.kind
    }

    /// Updates update.
    fn update(&mut self, dt: f32) {
        // Branch based on the current runtime condition.
        if self.kind != PelletKind::PowerPellet {
            return;
        }

        self.timer += dt;
        // Branch based on the current runtime condition.
        if self.timer >= self.flash_time {
            self.visible = !self.visible;
            self.timer = 0.0;
        }
    }
}

impl PelletGroup {
    /// Handles layout.
    pub fn from_layout(text: &str) -> Self {
        Self::from_text(text)
    }

    /// Handles len.
    pub fn len(&self) -> usize {
        self.pellets.len()
    }

    /// Handles eaten.
    pub fn num_eaten(&self) -> usize {
        self.num_eaten
    }

    /// Handles empty.
    pub fn is_empty(&self) -> bool {
        self.pellets.is_empty()
    }

    /// Computes pellet count.
    pub fn power_pellet_count(&self) -> usize {
        self.pellets
            .iter()
            .filter(|pellet| pellet.kind == PelletKind::PowerPellet)
            .count()
    }

    /// Updates update.
    pub fn update(&mut self, dt: f32) {
        // Iterate through each item in the current collection or range.
        for pellet in &mut self.pellets {
            pellet.update(dt);
        }
    }

    /// Handles iter.
    pub fn iter(&self) -> impl Iterator<Item = &Pellet> {
        self.pellets.iter()
    }

    /// Handles eat.
    pub fn try_eat(&mut self, position: Vector2, collide_radius: f32) -> Option<Pellet> {
        let index = self.pellets.iter().position(|pellet| {
            let distance = position - pellet.position;
            let collision_radius = pellet.radius + collide_radius;
            distance.magnitude_squared() <= collision_radius * collision_radius
        })?;

        self.num_eaten += 1;
        Some(self.pellets.remove(index))
    }

    /// Appends renderables.
    pub fn append_renderables(&self, frame: &mut FrameData) {
        let offset = Vector2::new(TILE_WIDTH as f32 * 0.5, TILE_HEIGHT as f32 * 0.5);
        // Iterate through each item in the current collection or range.
        for pellet in &self.pellets {
            // Branch based on the current runtime condition.
            if pellet.visible {
                frame.circles.push(Circle {
                    center: pellet.position + offset,
                    radius: pellet.radius,
                    color: WHITE,
                });
            }
        }
    }

    /// Handles text.
    fn from_text(text: &str) -> Self {
        let mut pellets = Vec::new();

        // Iterate through each item in the current collection or range.
        for (row, line) in text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .enumerate()
        {
            // Iterate through each item in the current collection or range.
            for (column, symbol) in line.split_whitespace().enumerate() {
                // Select the next behavior based on the current state.
                match symbol {
                    "." | "+" => pellets.push(Pellet::new(row, column, PelletKind::Pellet)),
                    "P" | "p" => pellets.push(Pellet::new(row, column, PelletKind::PowerPellet)),
                    _ => {}
                }
            }
        }

        Self {
            pellets,
            num_eaten: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ARCADE_MAZE_LAYOUT, PelletGroup, PelletKind};
    use crate::vector::Vector2;

    #[test]
    /// Handles one has the expected pellet counts.
    fn maze_one_has_the_expected_pellet_counts() {
        let pellets = PelletGroup::from_layout(ARCADE_MAZE_LAYOUT);

        assert_eq!(pellets.len(), 244);
        assert_eq!(pellets.power_pellet_count(), 4);
        assert!(!pellets.is_empty());
    }

    #[test]
    /// Computes pellets flash after their timer expires.
    fn power_pellets_flash_after_their_timer_expires() {
        let mut pellets = PelletGroup::from_layout(ARCADE_MAZE_LAYOUT);
        let initial_visible = pellets
            .pellets
            .iter()
            .find(|pellet| pellet.kind == PelletKind::PowerPellet)
            .expect("maze should include a power pellet")
            .visible;

        pellets.update(0.2);

        let current_visible = pellets
            .pellets
            .iter()
            .find(|pellet| pellet.kind == PelletKind::PowerPellet)
            .expect("maze should still include a power pellet")
            .visible;
        assert_ne!(initial_visible, current_visible);
    }

    #[test]
    /// Handles can be eaten at pacmans position.
    fn pellets_can_be_eaten_at_pacmans_position() {
        let mut pellets = PelletGroup::from_layout(ARCADE_MAZE_LAYOUT);

        let pellet = pellets
            .try_eat(Vector2::new(16.0, 64.0), 5.0)
            .expect("pacman should start on a pellet");

        assert_eq!(pellet.position, Vector2::new(16.0, 64.0));
        assert_eq!(pellet.points(), 10);
        assert_eq!(pellets.len(), 243);
        assert_eq!(pellets.num_eaten(), 1);
    }

    #[test]
    /// Handles layouts can build their own pellet groups.
    fn custom_layouts_can_build_their_own_pellet_groups() {
        let pellets = PelletGroup::from_layout(
            "
            . x P
            x + p
            ",
        );

        assert_eq!(pellets.len(), 4);
        assert_eq!(pellets.power_pellet_count(), 2);
    }
}
