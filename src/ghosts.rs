use crate::{
    actors::{EntityKind, GhostKind},
    constants::{
        NCOLS, NROWS, ORANGE, PACMAN_COLLIDE_RADIUS, PACMAN_RADIUS, PINK, RED, TEAL, TILE_HEIGHT,
        TILE_WIDTH,
    },
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
    kind: GhostKind,
    position: Vector2,
    direction: Direction,
    speed: f32,
    radius: f32,
    collide_radius: f32,
    color: [u8; 4],
    node: NodeId,
    target: NodeId,
    start_node: NodeId,
    disable_portal: bool,
    goal: Vector2,
    direction_method: DirectionMethod,
    mode: ModeController,
    spawn_node: Option<NodeId>,
    visible: bool,
    points: u32,
}

#[derive(Clone, Debug)]
pub struct GhostGroup {
    ghosts: [Ghost; 4],
}

impl Ghost {
    pub fn new(kind: GhostKind, node: NodeId, nodes: &NodeGroup) -> Self {
        let mut ghost = Self {
            kind,
            position: Vector2::default(),
            direction: Direction::Stop,
            speed: 100.0 * TILE_WIDTH as f32 / 16.0,
            radius: PACMAN_RADIUS,
            collide_radius: PACMAN_COLLIDE_RADIUS,
            color: color_for(kind),
            node,
            target: node,
            start_node: node,
            disable_portal: false,
            goal: Vector2::default(),
            direction_method: DirectionMethod::Goal,
            mode: ModeController::new(),
            spawn_node: None,
            visible: true,
            points: 200,
        };
        ghost.set_position(nodes);
        ghost
    }

    pub fn kind(&self) -> GhostKind {
        self.kind
    }

    pub fn entity_kind(&self) -> EntityKind {
        self.kind.entity()
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

    pub fn target_node(&self) -> NodeId {
        self.target
    }

    pub fn speed(&self) -> f32 {
        self.speed
    }

    pub fn collide_radius(&self) -> f32 {
        self.collide_radius
    }

    pub fn mode(&self) -> GhostMode {
        self.mode.current()
    }

    pub fn freight_remaining(&self) -> Option<f32> {
        self.mode.freight_remaining()
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn points(&self) -> u32 {
        self.points
    }

    pub fn renderable(&self) -> Circle {
        Circle {
            center: self.position,
            radius: self.radius,
            color: self.color,
        }
    }

    pub fn set_start_node(&mut self, node: NodeId, nodes: &NodeGroup) {
        self.start_node = node;
        self.node = node;
        self.target = node;
        self.set_position(nodes);
    }

    pub fn set_spawn_node(&mut self, node: NodeId) {
        self.spawn_node = Some(node);
    }

    pub fn reset(&mut self, nodes: &NodeGroup) {
        self.node = self.start_node;
        self.target = self.start_node;
        self.direction = Direction::Stop;
        self.speed = 100.0 * TILE_WIDTH as f32 / 16.0;
        self.visible = true;
        self.goal = Vector2::default();
        self.direction_method = DirectionMethod::Goal;
        self.mode = ModeController::new();
        self.points = 200;
        self.set_position(nodes);
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn start_freight(&mut self) {
        self.mode.set_freight_mode();
        if self.mode.current() == GhostMode::Freight {
            self.set_speed(50.0);
            self.direction_method = DirectionMethod::Random;
        }
    }

    pub fn sustain_freight(&mut self) {
        if matches!(self.mode.current(), GhostMode::Scatter | GhostMode::Chase) {
            self.start_freight();
        }
    }

    pub fn end_freight(&mut self) {
        if self.mode.clear_freight_mode() {
            self.normal_mode();
        }
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

    pub fn double_points(&mut self) {
        self.points *= 2;
    }

    pub fn reset_points(&mut self) {
        self.points = 200;
    }

    pub fn update(
        &mut self,
        dt: f32,
        nodes: &NodeGroup,
        pacman_position: Vector2,
        pacman_direction: Direction,
        blinky_position: Vector2,
    ) -> bool {
        let at_spawn_node = self.spawn_node.is_some_and(|spawn| self.node == spawn);
        let returned_to_normal = self.mode.update(dt, at_spawn_node);
        if returned_to_normal {
            self.normal_mode();
        }

        match self.mode.current() {
            GhostMode::Scatter => self.goal = self.scatter_goal(),
            GhostMode::Chase => {
                self.goal = self.chase_goal(pacman_position, pacman_direction, blinky_position)
            }
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

        returned_to_normal
    }

    fn scatter_goal(&self) -> Vector2 {
        match self.kind {
            GhostKind::Blinky => Vector2::default(),
            GhostKind::Pinky => Vector2::new(TILE_WIDTH as f32 * NCOLS as f32, 0.0),
            GhostKind::Inky => Vector2::new(
                TILE_WIDTH as f32 * NCOLS as f32,
                TILE_HEIGHT as f32 * NROWS as f32,
            ),
            GhostKind::Clyde => Vector2::new(0.0, TILE_HEIGHT as f32 * NROWS as f32),
        }
    }

    fn chase_goal(
        &self,
        pacman_position: Vector2,
        pacman_direction: Direction,
        blinky_position: Vector2,
    ) -> Vector2 {
        let tile = TILE_WIDTH as f32;
        match self.kind {
            GhostKind::Blinky => pacman_position,
            GhostKind::Pinky => pacman_position + pacman_direction.vector() * tile * 4.0,
            GhostKind::Inky => {
                let vec1 = pacman_position + pacman_direction.vector() * tile * 2.0;
                blinky_position + (vec1 - blinky_position) * 2.0
            }
            GhostKind::Clyde => {
                let distance = pacman_position - self.position;
                if distance.magnitude_squared() <= (tile * 8.0) * (tile * 8.0) {
                    self.scatter_goal()
                } else {
                    pacman_position + pacman_direction.vector() * tile * 4.0
                }
            }
        }
    }

    fn normal_mode(&mut self) {
        self.set_speed(100.0);
        self.direction_method = DirectionMethod::Goal;
    }

    fn set_speed(&mut self, speed: f32) {
        self.speed = speed * TILE_WIDTH as f32 / 16.0;
    }

    fn set_position(&mut self, nodes: &NodeGroup) {
        self.position = nodes.position(self.node);
    }

    fn valid_direction(&self, direction: Direction, nodes: &NodeGroup) -> bool {
        direction != Direction::Stop && nodes.can_travel(self.node, direction, self.entity_kind())
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

impl GhostGroup {
    pub fn new(node: NodeId, nodes: &NodeGroup) -> Self {
        Self {
            ghosts: [
                Ghost::new(GhostKind::Blinky, node, nodes),
                Ghost::new(GhostKind::Pinky, node, nodes),
                Ghost::new(GhostKind::Inky, node, nodes),
                Ghost::new(GhostKind::Clyde, node, nodes),
            ],
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Ghost> {
        self.ghosts.iter()
    }

    pub fn ghost(&self, kind: GhostKind) -> &Ghost {
        &self.ghosts[kind.index()]
    }

    pub fn ghost_mut(&mut self, kind: GhostKind) -> &mut Ghost {
        &mut self.ghosts[kind.index()]
    }

    pub fn entity_kinds(&self) -> [EntityKind; 4] {
        GhostKind::ALL.map(GhostKind::entity)
    }

    pub fn update(
        &mut self,
        dt: f32,
        nodes: &NodeGroup,
        pacman_position: Vector2,
        pacman_direction: Direction,
    ) -> Vec<EntityKind> {
        let mut returned_to_normal = Vec::new();

        for kind in GhostKind::ALL {
            let blinky_position = self.ghost(GhostKind::Blinky).position();
            let ghost = self.ghost_mut(kind);
            if ghost.update(
                dt,
                nodes,
                pacman_position,
                pacman_direction,
                blinky_position,
            ) {
                returned_to_normal.push(ghost.entity_kind());
            }
        }

        returned_to_normal
    }

    pub fn start_freight(&mut self) {
        for ghost in &mut self.ghosts {
            ghost.start_freight();
        }
        self.reset_points();
    }

    pub fn sustain_freight(&mut self) {
        for ghost in &mut self.ghosts {
            ghost.sustain_freight();
        }
    }

    pub fn end_freight(&mut self) {
        for ghost in &mut self.ghosts {
            ghost.end_freight();
        }
    }

    pub fn has_freight_mode(&self) -> bool {
        self.ghosts
            .iter()
            .any(|ghost| ghost.mode() == GhostMode::Freight)
    }

    pub fn set_spawn_node(&mut self, node: NodeId) {
        for ghost in &mut self.ghosts {
            ghost.set_spawn_node(node);
        }
    }

    pub fn update_points(&mut self) {
        for ghost in &mut self.ghosts {
            ghost.double_points();
        }
    }

    pub fn reset_points(&mut self) {
        for ghost in &mut self.ghosts {
            ghost.reset_points();
        }
    }

    pub fn reset(&mut self, nodes: &NodeGroup) {
        for ghost in &mut self.ghosts {
            ghost.reset(nodes);
        }
    }

    pub fn hide(&mut self) {
        for ghost in &mut self.ghosts {
            ghost.hide();
        }
    }

    pub fn show(&mut self) {
        for ghost in &mut self.ghosts {
            ghost.show();
        }
    }
}

fn color_for(kind: GhostKind) -> [u8; 4] {
    match kind {
        GhostKind::Blinky => RED,
        GhostKind::Pinky => PINK,
        GhostKind::Inky => TEAL,
        GhostKind::Clyde => ORANGE,
    }
}

#[cfg(test)]
mod tests {
    use super::{Ghost, GhostGroup};
    use crate::{
        actors::{EntityKind, GhostKind},
        modes::GhostMode,
        nodes::NodeGroup,
        pacman::Direction,
        vector::Vector2,
    };

    #[test]
    fn blinky_starts_in_scatter_mode() {
        let nodes = NodeGroup::pacman_maze();
        let ghost = Ghost::new(GhostKind::Blinky, nodes.start_node(), &nodes);

        assert_eq!(ghost.mode(), GhostMode::Scatter);
        assert_eq!(ghost.position(), Vector2::new(16.0, 64.0));
    }

    #[test]
    fn freight_mode_changes_the_ghost_mode() {
        let nodes = NodeGroup::pacman_maze();
        let mut ghost = Ghost::new(GhostKind::Blinky, nodes.start_node(), &nodes);

        ghost.start_freight();

        assert_eq!(ghost.mode(), GhostMode::Freight);
        ghost.update(
            0.0,
            &nodes,
            Vector2::new(0.0, 0.0),
            Direction::Stop,
            Vector2::default(),
        );
    }

    #[test]
    fn spawn_mode_targets_the_spawn_node() {
        let mut nodes = NodeGroup::pacman_maze();
        nodes.create_home_nodes(11.5, 14.0);
        let spawn_node = nodes
            .get_node_from_tiles(13.5, 17.0)
            .expect("spawn node should exist");
        let mut ghost = Ghost::new(GhostKind::Blinky, nodes.start_node(), &nodes);
        ghost.set_spawn_node(spawn_node);
        ghost.start_freight();
        ghost.start_spawn(&nodes);

        assert_eq!(ghost.mode(), GhostMode::Spawn);
    }

    #[test]
    fn ghost_group_contains_all_four_ghosts() {
        let nodes = NodeGroup::pacman_maze();
        let ghosts = GhostGroup::new(nodes.start_node(), &nodes);
        let kinds = ghosts
            .iter()
            .map(|ghost| ghost.entity_kind())
            .collect::<Vec<_>>();

        assert_eq!(
            kinds,
            vec![
                EntityKind::Blinky,
                EntityKind::Pinky,
                EntityKind::Inky,
                EntityKind::Clyde
            ]
        );
    }

    #[test]
    fn clyde_switches_to_scatter_when_pacman_is_close() {
        let nodes = NodeGroup::pacman_maze();
        let mut ghost = Ghost::new(GhostKind::Clyde, nodes.start_node(), &nodes);

        ghost.update(
            0.0,
            &nodes,
            ghost.position(),
            Direction::Right,
            Vector2::default(),
        );

        assert_eq!(ghost.mode(), GhostMode::Scatter);
    }
}
