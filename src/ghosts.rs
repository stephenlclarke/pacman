use crate::{
    constants::{PACMAN_COLLIDE_RADIUS, PACMAN_RADIUS, TILE_WIDTH, WHITE},
    modes::{GhostMode, ModeController},
    nodes::{NodeGroup, NodeId},
    pacman::Direction,
    render::Circle,
    vector::Vector2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DirectionMethod {
    Goal,
    Random,
}

#[derive(Clone, Debug)]
pub struct Ghost {
    position: Vector2,
    direction: Direction,
    speed: f32,
    radius: f32,
    collide_radius: f32,
    color: [u8; 4],
    node: NodeId,
    target: NodeId,
    disable_portal: bool,
    goal: Vector2,
    direction_method: DirectionMethod,
    mode: ModeController,
    spawn_node: Option<NodeId>,
}

impl Ghost {
    pub fn new(node: NodeId) -> Self {
        Self {
            position: Vector2::default(),
            direction: Direction::Stop,
            speed: 100.0 * TILE_WIDTH as f32 / 16.0,
            radius: PACMAN_RADIUS,
            collide_radius: PACMAN_COLLIDE_RADIUS,
            color: WHITE,
            node,
            target: node,
            disable_portal: false,
            goal: Vector2::default(),
            direction_method: DirectionMethod::Goal,
            mode: ModeController::new(),
            spawn_node: None,
        }
    }

    pub fn initialize_position(&mut self, nodes: &NodeGroup) {
        self.position = nodes.position(self.node);
    }

    pub fn update(&mut self, dt: f32, nodes: &NodeGroup, pacman_position: Vector2) {
        let at_spawn_node = self.spawn_node.is_some_and(|spawn| self.node == spawn);
        if self.mode.update(dt, at_spawn_node) {
            self.normal_mode();
        }

        match self.mode.current() {
            GhostMode::Scatter => self.scatter(),
            GhostMode::Chase => self.chase(pacman_position),
            GhostMode::Freight | GhostMode::Spawn => {}
        }

        self.position += self.direction.vector() * self.speed * dt;
        if self.overshot_target(nodes) {
            self.node = self.target;
            let directions = self.valid_directions(nodes);
            let next_direction = match self.direction_method {
                DirectionMethod::Goal => self.goal_direction(&directions, nodes),
                DirectionMethod::Random => self.random_direction(&directions),
            };

            if !self.disable_portal
                && let Some(portal) = nodes.portal(self.node)
            {
                self.node = portal;
            }

            self.target = self.get_new_target(next_direction, nodes);
            if self.target != self.node {
                self.direction = next_direction;
            } else {
                self.target = self.get_new_target(self.direction, nodes);
            }

            self.position = nodes.position(self.node);
        }
    }

    pub fn position(&self) -> Vector2 {
        self.position
    }

    pub fn collide_radius(&self) -> f32 {
        self.collide_radius
    }

    pub fn mode(&self) -> GhostMode {
        self.mode.current()
    }

    pub fn renderable(&self) -> Circle {
        Circle {
            center: self.position,
            radius: self.radius,
            color: self.color,
        }
    }

    pub fn start_freight(&mut self) {
        self.mode.set_freight_mode();
        if self.mode.current() == GhostMode::Freight {
            self.set_speed(50.0);
            self.direction_method = DirectionMethod::Random;
        }
    }

    pub fn set_spawn_node(&mut self, node: NodeId) {
        self.spawn_node = Some(node);
    }

    pub fn start_spawn(&mut self, nodes: &NodeGroup) {
        self.mode.set_spawn_mode();
        if self.mode.current() == GhostMode::Spawn {
            self.set_speed(150.0);
            self.direction_method = DirectionMethod::Goal;
            if let Some(spawn_node) = self.spawn_node {
                self.goal = nodes.position(spawn_node);
            }
        }
    }

    fn scatter(&mut self) {
        self.goal = Vector2::default();
    }

    fn chase(&mut self, pacman_position: Vector2) {
        self.goal = pacman_position;
    }

    fn normal_mode(&mut self) {
        self.set_speed(100.0);
        self.direction_method = DirectionMethod::Goal;
    }

    fn set_speed(&mut self, speed: f32) {
        self.speed = speed * TILE_WIDTH as f32 / 16.0;
    }

    fn valid_direction(&self, direction: Direction, nodes: &NodeGroup) -> bool {
        direction != Direction::Stop && nodes.neighbor(self.node, direction).is_some()
    }

    fn valid_directions(&self, nodes: &NodeGroup) -> Vec<Direction> {
        let mut directions = Vec::new();
        for direction in Direction::cardinals() {
            if self.valid_direction(direction, nodes) && direction != self.direction.opposite() {
                directions.push(direction);
            }
        }

        if directions.is_empty() {
            directions.push(self.direction.opposite());
        }

        directions
    }

    fn random_direction(&self, directions: &[Direction]) -> Direction {
        directions[fastrand::usize(..directions.len())]
    }

    fn goal_direction(&self, directions: &[Direction], nodes: &NodeGroup) -> Direction {
        let mut best = directions[0];
        let mut best_distance = f32::INFINITY;

        for direction in directions {
            let next_position =
                nodes.position(self.node) + direction.vector() * TILE_WIDTH as f32 - self.goal;
            let distance = next_position.magnitude_squared();
            if distance < best_distance {
                best = *direction;
                best_distance = distance;
            }
        }

        best
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
}

#[cfg(test)]
mod tests {
    use super::Ghost;
    use crate::{modes::GhostMode, nodes::NodeGroup, vector::Vector2};

    #[test]
    fn ghost_starts_in_scatter_mode() {
        let nodes = NodeGroup::pacman_maze();
        let mut ghost = Ghost::new(nodes.start_node());
        ghost.initialize_position(&nodes);

        assert_eq!(ghost.mode(), GhostMode::Scatter);
        assert_eq!(ghost.position(), Vector2::new(16.0, 64.0));
    }

    #[test]
    fn freight_mode_changes_the_ghost_speed() {
        let nodes = NodeGroup::pacman_maze();
        let mut ghost = Ghost::new(nodes.start_node());
        ghost.initialize_position(&nodes);

        ghost.start_freight();

        assert_eq!(ghost.mode(), GhostMode::Freight);
        ghost.update(0.0, &nodes, Vector2::new(0.0, 0.0));
    }

    #[test]
    fn spawn_mode_targets_the_spawn_node() {
        let mut nodes = NodeGroup::pacman_maze();
        nodes.create_home_nodes(11.5, 14.0);
        let spawn_node = nodes
            .get_node_from_tiles(13.5, 17.0)
            .expect("spawn node should exist");
        let mut ghost = Ghost::new(nodes.start_node());
        ghost.initialize_position(&nodes);
        ghost.set_spawn_node(spawn_node);
        ghost.start_freight();
        ghost.start_spawn(&nodes);

        assert_eq!(ghost.mode(), GhostMode::Spawn);
    }
}
