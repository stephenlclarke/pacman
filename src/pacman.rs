use crate::{
    constants::{
        PACMAN_COLLIDE_RADIUS, PACMAN_RADIUS, PACMAN_SPEED, PACMAN_START_X, PACMAN_START_Y, YELLOW,
    },
    nodes::{NodeGroup, NodeId},
    render::Circle,
    vector::Vector2,
};

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
    pub const fn cardinals() -> [Self; 4] {
        [Self::Up, Self::Down, Self::Left, Self::Right]
    }

    pub fn vector(self) -> Vector2 {
        match self {
            Self::Stop => Vector2::default(),
            Self::Up => Vector2::new(0.0, -1.0),
            Self::Down => Vector2::new(0.0, 1.0),
            Self::Left => Vector2::new(-1.0, 0.0),
            Self::Right => Vector2::new(1.0, 0.0),
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            Self::Stop => Self::Stop,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    pub fn neighbor_index(self) -> Option<usize> {
        match self {
            Self::Up => Some(0),
            Self::Down => Some(1),
            Self::Left => Some(2),
            Self::Right => Some(3),
            Self::Stop => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BasicPacman {
    position: Vector2,
    direction: Direction,
    speed: f32,
    radius: f32,
    color: [u8; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeMovementMode {
    Teleport,
    OvershootStop,
    Reversible,
}

#[derive(Clone, Debug)]
pub struct NodePacman {
    position: Vector2,
    direction: Direction,
    speed: f32,
    radius: f32,
    color: [u8; 4],
    node: NodeId,
    target: NodeId,
    mode: NodeMovementMode,
    collide_radius: f32,
}

impl BasicPacman {
    pub fn new() -> Self {
        Self {
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

    pub fn renderable(&self) -> Circle {
        Circle {
            center: self.position,
            radius: self.radius,
            color: self.color,
        }
    }
}

impl Default for BasicPacman {
    fn default() -> Self {
        Self::new()
    }
}

impl NodePacman {
    pub fn new(start_node: NodeId, nodes: &NodeGroup, mode: NodeMovementMode) -> Self {
        let mut pacman = Self {
            position: nodes.position(start_node),
            direction: Direction::Stop,
            speed: PACMAN_SPEED,
            radius: PACMAN_RADIUS,
            color: YELLOW,
            node: start_node,
            target: start_node,
            mode,
            collide_radius: PACMAN_COLLIDE_RADIUS,
        };
        pacman.set_position(nodes);
        pacman
    }

    pub fn update(&mut self, dt: f32, requested_direction: Direction, nodes: &NodeGroup) {
        match self.mode {
            NodeMovementMode::Teleport => self.update_teleport(requested_direction, nodes),
            NodeMovementMode::OvershootStop => {
                self.update_overshoot(dt, requested_direction, nodes)
            }
            NodeMovementMode::Reversible => self.update_reversible(dt, requested_direction, nodes),
        }
    }

    pub fn position(&self) -> Vector2 {
        self.position
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn current_node(&self) -> NodeId {
        self.node
    }

    pub fn target(&self) -> NodeId {
        self.target
    }

    pub fn collide_radius(&self) -> f32 {
        self.collide_radius
    }

    pub fn renderable(&self) -> Circle {
        Circle {
            center: self.position,
            radius: self.radius,
            color: self.color,
        }
    }

    fn update_teleport(&mut self, requested_direction: Direction, nodes: &NodeGroup) {
        self.direction = requested_direction;
        self.node = self.get_new_target(requested_direction, nodes);
        self.target = self.node;
        self.set_position(nodes);
    }

    fn update_overshoot(&mut self, dt: f32, requested_direction: Direction, nodes: &NodeGroup) {
        self.position += self.direction.vector() * self.speed * dt;

        if self.overshot_target(nodes) {
            self.enter_target_node(nodes);
            self.target = self.get_new_target(requested_direction, nodes);
            if self.target != self.node {
                self.direction = requested_direction;
            } else {
                self.direction = Direction::Stop;
            }
            self.set_position(nodes);
        }
    }

    fn update_reversible(&mut self, dt: f32, requested_direction: Direction, nodes: &NodeGroup) {
        self.position += self.direction.vector() * self.speed * dt;

        if self.overshot_target(nodes) {
            self.enter_target_node(nodes);
            self.target = self.get_new_target(requested_direction, nodes);
            if self.target != self.node {
                self.direction = requested_direction;
            } else {
                self.target = self.get_new_target(self.direction, nodes);
            }
            if self.target == self.node {
                self.direction = Direction::Stop;
            }
            self.set_position(nodes);
        } else if requested_direction != Direction::Stop
            && requested_direction == self.direction.opposite()
        {
            self.reverse_direction();
        }
    }

    fn enter_target_node(&mut self, nodes: &NodeGroup) {
        self.node = self.target;
        if let Some(portal) = nodes.portal(self.node) {
            self.node = portal;
        }
    }

    fn set_position(&mut self, nodes: &NodeGroup) {
        self.position = nodes.position(self.node);
    }

    fn valid_direction(&self, direction: Direction, nodes: &NodeGroup) -> bool {
        direction != Direction::Stop && nodes.neighbor(self.node, direction).is_some()
    }

    fn get_new_target(&self, direction: Direction, nodes: &NodeGroup) -> NodeId {
        if self.valid_direction(direction, nodes) {
            nodes.neighbor(self.node, direction).unwrap_or(self.node)
        } else {
            self.node
        }
    }

    fn overshot_target(&self, nodes: &NodeGroup) -> bool {
        let node_to_target = nodes.position(self.target) - nodes.position(self.node);
        let node_to_self = self.position - nodes.position(self.node);
        node_to_self.magnitude_squared() >= node_to_target.magnitude_squared()
    }

    fn reverse_direction(&mut self) {
        self.direction = self.direction.opposite();
        std::mem::swap(&mut self.node, &mut self.target);
    }
}

#[cfg(test)]
mod tests {
    use super::{BasicPacman, Direction, NodeMovementMode, NodePacman};
    use crate::{nodes::NodeGroup, vector::Vector2};

    #[test]
    fn pacman_starts_with_tutorial_defaults() {
        let pacman = BasicPacman::new();

        assert_eq!(pacman.position(), Vector2::new(200.0, 400.0));
        assert_eq!(pacman.direction(), Direction::Stop);
        assert_eq!(pacman.radius(), 10.0);
        assert_eq!(pacman.color(), [255, 255, 0, 255]);
    }

    #[test]
    fn pacman_moves_using_the_previous_frame_direction() {
        let mut pacman = BasicPacman::new();

        pacman.update(0.5, Direction::Right);
        assert_eq!(pacman.position(), Vector2::new(200.0, 400.0));

        pacman.update(0.5, Direction::Right);
        assert_eq!(pacman.position(), Vector2::new(250.0, 400.0));
    }

    #[test]
    fn teleport_mode_snaps_between_connected_nodes() {
        let nodes = NodeGroup::setup_test_nodes();
        let mut pacman = NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Teleport);

        pacman.update(0.0, Direction::Right, &nodes);

        assert_eq!(pacman.current_node(), 1);
        assert_eq!(pacman.position(), Vector2::new(160.0, 80.0));
    }

    #[test]
    fn overshoot_mode_advances_when_the_target_is_reached() {
        let nodes = NodeGroup::setup_test_nodes();
        let mut pacman =
            NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::OvershootStop);

        pacman.update(0.0, Direction::Right, &nodes);
        pacman.update(0.8, Direction::Right, &nodes);

        assert_eq!(pacman.current_node(), 1);
        assert_eq!(pacman.target(), 1);
        assert_eq!(pacman.direction(), Direction::Stop);
        assert_eq!(pacman.position(), Vector2::new(160.0, 80.0));
    }

    #[test]
    fn reversible_mode_swaps_node_and_target_mid_segment() {
        let nodes = NodeGroup::setup_test_nodes();
        let mut pacman = NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);

        pacman.update(0.0, Direction::Right, &nodes);
        pacman.update(0.2, Direction::Right, &nodes);
        pacman.update(0.0, Direction::Left, &nodes);

        assert_eq!(pacman.direction(), Direction::Left);
        assert_eq!(pacman.current_node(), 1);
        assert_eq!(pacman.target(), 0);
    }

    #[test]
    fn portal_nodes_teleport_to_their_pair_when_reached() {
        let mut nodes = NodeGroup::pacman_maze();
        nodes.set_portal_pair((0, 17), (27, 17));

        let left = nodes
            .get_node_from_tiles(0, 17)
            .expect("left portal should exist");
        let right = nodes
            .get_node_from_tiles(27, 17)
            .expect("right portal should exist");
        let mut pacman = NodePacman::new(left, &nodes, NodeMovementMode::Reversible);
        pacman.direction = Direction::Left;
        pacman.target = left;

        pacman.update(0.0, Direction::Stop, &nodes);

        assert_eq!(pacman.current_node(), right);
        assert_eq!(pacman.position(), nodes.position(right));
        assert_eq!(pacman.collide_radius(), 5.0);
    }
}
