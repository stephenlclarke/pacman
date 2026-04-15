use crate::{
    actors::EntityKind,
    arcade::{MovePatternState, ORIGINAL_FRAME_TIME, move_patterns},
    constants::{ARCADE_PIXEL_STEP, PACMAN_COLLIDE_RADIUS, PACMAN_RADIUS, YELLOW},
    nodes::{NodeGroup, NodeId},
    render::Circle,
    vector::Vector2,
};

const PRETURN_DISTANCE: f32 = 4.0;

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
pub struct NodePacman {
    position: Vector2,
    direction: Direction,
    radius: f32,
    color: [u8; 4],
    node: NodeId,
    target: NodeId,
    collide_radius: f32,
    start_node: NodeId,
    start_direction: Direction,
    start_between: Option<Direction>,
    start_position_override: Option<Vector2>,
    frame_accumulator: f32,
    frightened: bool,
    normal_move_pattern: MovePatternState,
    frightened_move_pattern: MovePatternState,
    visible: bool,
    alive: bool,
}

impl NodePacman {
    pub fn new(start_node: NodeId, nodes: &NodeGroup) -> Self {
        let mut pacman = Self {
            position: nodes.position(start_node),
            direction: Direction::Stop,
            radius: PACMAN_RADIUS,
            color: YELLOW,
            node: start_node,
            target: start_node,
            collide_radius: PACMAN_COLLIDE_RADIUS,
            start_node,
            start_direction: Direction::Stop,
            start_between: None,
            start_position_override: None,
            frame_accumulator: 0.0,
            frightened: false,
            normal_move_pattern: MovePatternState::new(move_patterns(1).pacman_normal),
            frightened_move_pattern: MovePatternState::new(move_patterns(1).pacman_frightened),
            visible: true,
            alive: true,
        };
        pacman.set_position(nodes);
        pacman
    }

    pub fn configure_start(
        &mut self,
        start_node: NodeId,
        direction: Direction,
        between: Option<Direction>,
        start_position: Option<Vector2>,
        nodes: &NodeGroup,
    ) {
        self.start_node = start_node;
        self.start_direction = direction;
        self.start_between = between;
        self.start_position_override = start_position;
        self.reset(nodes);
    }

    pub fn reset(&mut self, nodes: &NodeGroup) {
        self.node = self.start_node;
        self.target = self.start_node;
        self.direction = self.start_direction;
        self.frame_accumulator = 0.0;
        self.frightened = false;
        self.normal_move_pattern.reset();
        self.frightened_move_pattern.reset();
        self.visible = true;
        self.alive = true;
        self.set_position(nodes);

        if let Some(direction) = self.start_between {
            self.set_between_nodes(direction, nodes);
            self.direction = direction;
        }

        if let Some(position) = self.start_position_override {
            self.position = position;
        }
    }

    pub fn die(&mut self) {
        self.alive = false;
        self.direction = Direction::Stop;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn alive(&self) -> bool {
        self.alive
    }

    pub fn update(&mut self, dt: f32, requested_direction: Direction, nodes: &NodeGroup) {
        if !self.alive {
            return;
        }

        self.prime_direction(requested_direction, nodes);
        self.frame_accumulator += dt;
        while self.frame_accumulator >= ORIGINAL_FRAME_TIME {
            self.frame_accumulator -= ORIGINAL_FRAME_TIME;
            if self.advance_move_pattern() {
                self.update_reversible(requested_direction, nodes);
            }
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

    pub fn configure_level(&mut self, level: u32) {
        let patterns = move_patterns(level);
        self.normal_move_pattern = MovePatternState::new(patterns.pacman_normal);
        self.frightened_move_pattern = MovePatternState::new(patterns.pacman_frightened);
    }

    pub fn set_frightened(&mut self, frightened: bool) {
        self.frightened = frightened;
    }

    pub fn collide_check(&self, other_position: Vector2, other_collide_radius: f32) -> bool {
        let distance = self.position - other_position;
        let collide_radius = self.collide_radius + other_collide_radius;
        distance.magnitude_squared() <= collide_radius * collide_radius
    }

    pub fn renderable(&self) -> Circle {
        Circle {
            center: self.position,
            radius: self.radius,
            color: self.color,
        }
    }

    pub fn teleport_to_node(&mut self, node: NodeId, nodes: &NodeGroup) {
        self.node = node;
        self.target = node;
        self.direction = Direction::Stop;
        self.set_position(nodes);
    }

    fn update_reversible(&mut self, requested_direction: Direction, nodes: &NodeGroup) {
        self.position += self.direction.vector() * ARCADE_PIXEL_STEP;

        if self.try_preturn(requested_direction, nodes) {
            return;
        }

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

    fn try_preturn(&mut self, requested_direction: Direction, nodes: &NodeGroup) -> bool {
        if self.direction == Direction::Stop
            || requested_direction == Direction::Stop
            || requested_direction == self.direction.opposite()
            || !nodes.can_travel(self.target, requested_direction, EntityKind::Pacman)
        {
            return false;
        }

        let target_position = nodes.position(self.target);
        if (self.position - target_position).magnitude() > PRETURN_DISTANCE {
            return false;
        }

        self.node = self.target;
        self.position = target_position;
        self.target = self.get_new_target(requested_direction, nodes);
        self.direction = requested_direction;
        true
    }

    fn enter_target_node(&mut self, nodes: &NodeGroup) {
        self.node = self.target;
        if let Some(portal) = nodes.portal(self.node) {
            self.node = portal;
        }
    }

    fn set_between_nodes(&mut self, direction: Direction, nodes: &NodeGroup) {
        self.target = self.get_new_target(direction, nodes);
        if self.target != self.node {
            self.position = (nodes.position(self.node) + nodes.position(self.target)) * 0.5;
        } else {
            self.set_position(nodes);
        }
    }

    fn set_position(&mut self, nodes: &NodeGroup) {
        self.position = nodes.position(self.node);
    }

    fn valid_direction(&self, direction: Direction, nodes: &NodeGroup) -> bool {
        direction != Direction::Stop && nodes.can_travel(self.node, direction, EntityKind::Pacman)
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

    fn advance_move_pattern(&mut self) -> bool {
        if self.frightened {
            self.frightened_move_pattern.advance()
        } else {
            self.normal_move_pattern.advance()
        }
    }

    fn prime_direction(&mut self, requested_direction: Direction, nodes: &NodeGroup) {
        if self.target != self.node {
            return;
        }

        let direction = if requested_direction != Direction::Stop {
            requested_direction
        } else {
            self.direction
        };
        let target = self.get_new_target(direction, nodes);
        if target != self.node {
            self.target = target;
            self.direction = direction;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Direction, NodePacman};
    use crate::{
        actors::EntityKind, arcade::ORIGINAL_FRAME_TIME, nodes::NodeGroup, vector::Vector2,
    };

    #[test]
    fn portal_nodes_teleport_to_their_pair_when_reached() {
        let mut nodes = NodeGroup::pacman_maze();
        nodes.set_portal_pair((0.0, 17.0), (27.0, 17.0));
        let left_portal = nodes
            .get_node_from_tiles(6.0, 17.0)
            .expect("left-side entry node should exist");
        let right_portal = nodes
            .get_node_from_tiles(27.0, 17.0)
            .expect("right portal should exist");
        let mut pacman = NodePacman::new(left_portal, &nodes);

        pacman.update(0.0, Direction::Left, &nodes);
        for _ in 0..192 {
            pacman.update(ORIGINAL_FRAME_TIME, Direction::Left, &nodes);
            if pacman.current_node() == right_portal {
                break;
            }
        }

        assert_eq!(pacman.current_node(), right_portal);
    }

    #[test]
    fn configured_reset_places_pacman_between_nodes() {
        let nodes = NodeGroup::pacman_maze();
        let start_node = nodes
            .get_node_from_tiles(15.0, 26.0)
            .expect("level 4 pacman start node should exist");
        let mut pacman = NodePacman::new(start_node, &nodes);
        pacman.configure_start(
            start_node,
            Direction::Left,
            Some(Direction::Left),
            None,
            &nodes,
        );

        assert_eq!(pacman.direction(), Direction::Left);
        assert_eq!(pacman.position(), Vector2::new(216.0, 416.0));
    }

    #[test]
    fn queued_turn_is_taken_at_the_next_intersection() {
        let nodes = NodeGroup::pacman_maze();
        let start_node = nodes
            .get_node_from_tiles(15.0, 26.0)
            .expect("level 4 pacman start node should exist");
        let intersection = nodes
            .get_node_from_tiles(12.0, 26.0)
            .expect("queued turn intersection should exist");
        let up_target = nodes
            .neighbor(intersection, Direction::Up)
            .expect("queued up turn should be available");

        let mut pacman = NodePacman::new(start_node, &nodes);
        pacman.configure_start(
            start_node,
            Direction::Left,
            Some(Direction::Left),
            None,
            &nodes,
        );
        for _ in 0..64 {
            pacman.update(ORIGINAL_FRAME_TIME, Direction::Up, &nodes);
            if pacman.current_node() == intersection {
                break;
            }
        }

        assert_eq!(pacman.current_node(), intersection);
        assert_eq!(pacman.target(), up_target);
        assert_eq!(pacman.direction(), Direction::Up);
        assert_eq!(pacman.position(), nodes.position(intersection));
    }

    #[test]
    fn latest_requested_turn_replaces_the_previous_queue() {
        let nodes = NodeGroup::pacman_maze();
        let start_node = nodes
            .get_node_from_tiles(15.0, 26.0)
            .expect("level 4 pacman start node should exist");
        let intersection = nodes
            .get_node_from_tiles(12.0, 26.0)
            .expect("queued turn intersection should exist");
        let left_target = nodes
            .neighbor(intersection, Direction::Left)
            .expect("continuing left should be available");

        let mut pacman = NodePacman::new(start_node, &nodes);
        pacman.configure_start(
            start_node,
            Direction::Left,
            Some(Direction::Left),
            None,
            &nodes,
        );
        for _ in 0..64 {
            pacman.update(ORIGINAL_FRAME_TIME, Direction::Left, &nodes);
            if pacman.current_node() == intersection {
                break;
            }
        }

        assert_eq!(pacman.current_node(), intersection);
        assert_eq!(pacman.target(), left_target);
        assert_eq!(pacman.direction(), Direction::Left);
        assert_eq!(pacman.position(), nodes.position(intersection));
    }

    #[test]
    fn access_restrictions_block_pacman_movement() {
        let mut nodes = NodeGroup::pacman_maze();
        let home = nodes.create_home_nodes(11.5, 14.0);
        nodes.connect_home_nodes(home, (12.0, 14.0), Direction::Left);
        nodes.connect_home_nodes(home, (15.0, 14.0), Direction::Right);
        nodes.deny_home_access(EntityKind::Pacman);

        let mut pacman = NodePacman::new(home, &nodes);
        pacman.update(0.0, Direction::Down, &nodes);

        assert_eq!(pacman.target(), home);
        assert_eq!(pacman.direction(), Direction::Stop);
    }
}
