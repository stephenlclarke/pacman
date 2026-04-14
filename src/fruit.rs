use crate::{
    constants::{GREEN, PACMAN_COLLIDE_RADIUS, PACMAN_RADIUS},
    nodes::{NodeGroup, NodeId},
    pacman::Direction,
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
    pub fn new(node: NodeId, nodes: &NodeGroup) -> Self {
        Self::for_level(node, nodes, 0)
    }

    pub fn for_level(node: NodeId, nodes: &NodeGroup, level_index: u32) -> Self {
        let target = nodes.neighbor(node, Direction::Right).unwrap_or(node);
        let position = (nodes.position(node) + nodes.position(target)) * 0.5;
        let sprite_index = (level_index as usize) % 6;

        Self {
            position,
            radius: PACMAN_RADIUS,
            collide_radius: PACMAN_COLLIDE_RADIUS,
            color: GREEN,
            lifespan: 5.0,
            timer: 0.0,
            destroy: false,
            points: 100 + level_index * 20,
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

#[cfg(test)]
mod tests {
    use super::Fruit;
    use crate::nodes::NodeGroup;

    #[test]
    fn fruit_spawns_between_the_expected_nodes() {
        let nodes = NodeGroup::pacman_maze();
        let node = nodes
            .get_node_from_tiles(9.0, 20.0)
            .expect("fruit spawn node should exist");
        let fruit = Fruit::new(node, &nodes);

        assert_eq!(fruit.position().as_tuple(), (216.0, 320.0));
        assert_eq!(fruit.points(), 100);
        assert_eq!(fruit.sprite_index(), 0);
    }

    #[test]
    fn fruit_marks_itself_for_removal_after_its_lifespan() {
        let nodes = NodeGroup::pacman_maze();
        let node = nodes
            .get_node_from_tiles(9.0, 20.0)
            .expect("fruit spawn node should exist");
        let mut fruit = Fruit::new(node, &nodes);

        fruit.update(5.1);

        assert!(fruit.destroyed());
    }

    #[test]
    fn fruit_level_controls_points_and_sprite_cycle() {
        let nodes = NodeGroup::pacman_maze();
        let node = nodes
            .get_node_from_tiles(9.0, 20.0)
            .expect("fruit spawn node should exist");
        let fruit = Fruit::for_level(node, &nodes, 7);

        assert_eq!(fruit.points(), 240);
        assert_eq!(fruit.sprite_index(), 1);
    }
}
