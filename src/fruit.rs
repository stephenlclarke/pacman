//! Models bonus fruit spawning, timing, scoring, and per-level sprite selection.

use crate::{
    arcade::{fruit_lifespan_seconds, level_spec},
    constants::{GREEN, PACMAN_COLLIDE_RADIUS, PACMAN_RADIUS},
    render::Circle,
    vector::Vector2,
};

#[derive(Clone, Debug)]
pub struct Fruit {
    position: Vector2,
    radius: f32,
    collide_radius: f32,
    color: [u8; 4],
    lifespan: f32,
    timer: f32,
    destroy: bool,
    points: u32,
    sprite_index: usize,
}

impl Fruit {
    pub fn new(position: Vector2) -> Self {
        Self::for_level(position, 1)
    }

    pub fn for_level(position: Vector2, level: u32) -> Self {
        let sprite_index = sprite_slot(level);
        let spec = level_spec(level);

        Self {
            position,
            radius: PACMAN_RADIUS,
            collide_radius: PACMAN_COLLIDE_RADIUS,
            color: GREEN,
            lifespan: fruit_lifespan_seconds(),
            timer: 0.0,
            destroy: false,
            points: spec.fruit_points,
            sprite_index,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.timer += dt;
        if self.timer >= self.lifespan {
            self.destroy = true;
        }
    }

    pub fn position(&self) -> Vector2 {
        self.position
    }

    pub fn collide_radius(&self) -> f32 {
        self.collide_radius
    }

    pub fn destroyed(&self) -> bool {
        self.destroy
    }

    pub fn remaining_life(&self) -> f32 {
        (self.lifespan - self.timer).max(0.0)
    }

    pub fn points(&self) -> u32 {
        self.points
    }

    pub fn sprite_index(&self) -> usize {
        self.sprite_index
    }

    pub fn renderable(&self) -> Circle {
        Circle {
            center: self.position,
            radius: self.radius,
            color: self.color,
        }
    }
}

fn sprite_slot(level: u32) -> usize {
    match level {
        1 => 0,
        2 => 1,
        3 | 4 => 2,
        5 | 6 => 3,
        7 | 8 => 4,
        9 | 10 => 5,
        11 | 12 => 6,
        _ => 7,
    }
}

#[cfg(test)]
mod tests {
    use super::Fruit;
    use crate::vector::Vector2;

    #[test]
    fn fruit_spawns_at_the_arcade_position() {
        let fruit = Fruit::new(Vector2::new(216.0, 320.0));

        assert_eq!(fruit.position().as_tuple(), (216.0, 320.0));
        assert_eq!(fruit.points(), 100);
        assert_eq!(fruit.sprite_index(), 0);
    }

    #[test]
    fn fruit_marks_itself_for_removal_after_its_lifespan() {
        let mut fruit = Fruit::new(Vector2::new(216.0, 320.0));

        fruit.update(10.0);

        assert!(fruit.destroyed());
    }

    #[test]
    fn fruit_level_controls_points_and_sprite_cycle() {
        let fruit = Fruit::for_level(Vector2::new(216.0, 320.0), 7);

        assert_eq!(fruit.points(), 1000);
        assert_eq!(fruit.sprite_index(), 4);
    }

    #[test]
    fn late_levels_use_the_key_bonus_sprite() {
        let fruit = Fruit::for_level(Vector2::new(216.0, 320.0), 13);

        assert_eq!(fruit.points(), 5000);
        assert_eq!(fruit.sprite_index(), 7);
    }
}
