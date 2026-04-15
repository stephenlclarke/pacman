use crate::{
    actors::{EntityKind, GhostKind},
    arcade::{MovePatternState, ORIGINAL_FRAME_TIME, level_spec, move_patterns},
    constants::{
        ARCADE_PIXEL_STEP, NCOLS, NROWS, ORANGE, PACMAN_COLLIDE_RADIUS, PACMAN_RADIUS,
        PACMAN_SPEED, PINK, RED, TEAL, TILE_HEIGHT, TILE_WIDTH,
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
    frame_accumulator: f32,
    speed: f32,
    radius: f32,
    collide_radius: f32,
    color: [u8; 4],
    node: NodeId,
    target: NodeId,
    start_node: NodeId,
    start_position_override: Option<Vector2>,
    goal: Vector2,
    direction_method: DirectionMethod,
    reverse_pending: bool,
    mode: ModeController,
    move_patterns: GhostMovePatterns,
    spawn_node: Option<NodeId>,
    visible: bool,
    points: u32,
}

#[derive(Clone, Debug)]
pub struct GhostGroup {
    ghosts: [Ghost; 4],
}

#[derive(Clone, Copy, Debug)]
pub struct GhostGroupUpdateContext {
    pub pacman_position: Vector2,
    pub pacman_direction: Direction,
    pub level: u32,
    pub dots_remaining: usize,
    pub elroy_enabled: bool,
}

#[derive(Clone, Copy, Debug)]
struct GhostUpdateContext {
    pacman_position: Vector2,
    pacman_direction: Direction,
    blinky_position: Vector2,
    level: u32,
    dots_remaining: usize,
    elroy_enabled: bool,
}

#[derive(Clone, Debug)]
struct GhostMovePatterns {
    normal: MovePatternState,
    frightened: MovePatternState,
    tunnel: MovePatternState,
    blinky_elroy_one: MovePatternState,
    blinky_elroy_two: MovePatternState,
}

impl GhostMovePatterns {
    fn for_level(level: u32) -> Self {
        let patterns = move_patterns(level);
        Self {
            normal: MovePatternState::new(patterns.ghost_normal),
            frightened: MovePatternState::new(patterns.ghost_frightened),
            tunnel: MovePatternState::new(patterns.ghost_tunnel),
            blinky_elroy_one: MovePatternState::new(patterns.blinky_elroy_one),
            blinky_elroy_two: MovePatternState::new(patterns.blinky_elroy_two),
        }
    }
}

impl Ghost {
    pub fn new(kind: GhostKind, node: NodeId, nodes: &NodeGroup, level: u32) -> Self {
        let mut ghost = Self {
            kind,
            position: Vector2::default(),
            direction: Direction::Stop,
            frame_accumulator: 0.0,
            speed: PACMAN_SPEED,
            radius: PACMAN_RADIUS,
            collide_radius: PACMAN_COLLIDE_RADIUS,
            color: color_for(kind),
            node,
            target: node,
            start_node: node,
            start_position_override: None,
            goal: Vector2::default(),
            direction_method: DirectionMethod::Goal,
            reverse_pending: false,
            mode: ModeController::new(level),
            move_patterns: GhostMovePatterns::for_level(level),
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
        self.mode.fright_remaining()
    }

    pub fn fright_total_duration(&self) -> Option<f32> {
        self.mode.fright_total_duration()
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
        self.set_start_state(node, None, nodes);
    }

    pub fn set_start_state(
        &mut self,
        node: NodeId,
        start_position: Option<Vector2>,
        nodes: &NodeGroup,
    ) {
        self.start_node = node;
        self.start_position_override = start_position;
        self.node = node;
        self.target = node;
        self.set_position(nodes);
        self.apply_start_position_override();
    }

    pub fn set_spawn_node(&mut self, node: NodeId) {
        self.spawn_node = Some(node);
    }

    pub fn reset(&mut self, nodes: &NodeGroup, level: u32) {
        self.node = self.start_node;
        self.target = self.start_node;
        self.direction = Direction::Stop;
        self.frame_accumulator = 0.0;
        self.speed = PACMAN_SPEED;
        self.visible = true;
        self.goal = Vector2::default();
        self.direction_method = DirectionMethod::Goal;
        self.reverse_pending = false;
        self.mode = ModeController::new(level);
        self.move_patterns = GhostMovePatterns::for_level(level);
        self.points = 200;
        self.set_position(nodes);
        self.apply_start_position_override();
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn start_freight(&mut self) {
        let reversed = self.mode.set_freight_mode();
        if reversed {
            self.reverse_pending = true;
        }
        if self.mode.current() == GhostMode::Freight {
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

    fn update(&mut self, dt: f32, nodes: &NodeGroup, context: GhostUpdateContext) -> bool {
        let at_spawn_node = self.spawn_node.is_some_and(|spawn| self.node == spawn);
        let transition = self.mode.update(dt, at_spawn_node);
        if transition.returned_to_normal {
            self.normal_mode();
        }
        if transition.reversed {
            self.reverse_pending = true;
        }
        let spec = level_spec(context.level);
        let elroy_active = self.kind == GhostKind::Blinky
            && context.elroy_enabled
            && context.dots_remaining <= spec.elroy_one_dots_left;

        match self.mode.current() {
            GhostMode::Scatter => {
                self.goal = if elroy_active {
                    context.pacman_position
                } else {
                    self.scatter_goal()
                }
            }
            GhostMode::Chase => {
                self.goal = self.chase_goal(
                    context.pacman_position,
                    context.pacman_direction,
                    context.blinky_position,
                )
            }
            GhostMode::Freight | GhostMode::Spawn => {}
        }

        self.update_speed(
            nodes,
            context.level,
            context.dots_remaining,
            context.elroy_enabled,
        );

        self.prime_direction(nodes);
        self.frame_accumulator += dt;
        let frames = (self.frame_accumulator / ORIGINAL_FRAME_TIME) as usize;
        self.frame_accumulator -= frames as f32 * ORIGINAL_FRAME_TIME;
        for _ in 0..frames {
            let steps = self.frame_steps(nodes, context);
            for _ in 0..steps {
                self.advance_position_step(nodes);
            }
        }

        transition.returned_to_normal
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
            GhostKind::Pinky => pacman_position + arcade_offset(pacman_direction, 4, true, tile),
            GhostKind::Inky => {
                let vec1 = pacman_position + arcade_offset(pacman_direction, 2, true, tile);
                blinky_position + (vec1 - blinky_position) * 2.0
            }
            GhostKind::Clyde => {
                let distance = pacman_position - self.position;
                if distance.magnitude_squared() <= (tile * 8.0) * (tile * 8.0) {
                    self.scatter_goal()
                } else {
                    pacman_position
                }
            }
        }
    }

    fn normal_mode(&mut self) {
        self.direction_method = DirectionMethod::Goal;
    }

    fn update_speed(
        &mut self,
        nodes: &NodeGroup,
        level: u32,
        dots_remaining: usize,
        elroy_enabled: bool,
    ) {
        let spec = level_spec(level);
        let percent = match self.mode.current() {
            GhostMode::Spawn => 2.0,
            _ if self.in_tunnel(nodes) => spec.ghost_tunnel_speed,
            GhostMode::Freight => spec.frightened_ghost_speed.unwrap_or(spec.ghost_speed),
            GhostMode::Scatter | GhostMode::Chase => {
                self.normal_mode_speed(spec, dots_remaining, elroy_enabled)
            }
        };
        self.speed = PACMAN_SPEED * percent;
    }

    fn normal_mode_speed(
        &self,
        spec: crate::arcade::ArcadeLevelSpec,
        dots_remaining: usize,
        elroy_enabled: bool,
    ) -> f32 {
        if self.kind != GhostKind::Blinky || !elroy_enabled {
            return spec.ghost_speed;
        }

        if dots_remaining <= spec.elroy_two_dots_left {
            spec.elroy_two_speed
        } else if dots_remaining <= spec.elroy_one_dots_left {
            spec.elroy_one_speed
        } else {
            spec.ghost_speed
        }
    }

    fn frame_steps(&mut self, nodes: &NodeGroup, context: GhostUpdateContext) -> usize {
        if self.mode.current() == GhostMode::Spawn {
            return 2;
        }

        let spec = level_spec(context.level);
        let elroy_state = if self.kind == GhostKind::Blinky && context.elroy_enabled {
            if context.dots_remaining <= spec.elroy_two_dots_left {
                Some(2)
            } else if context.dots_remaining <= spec.elroy_one_dots_left {
                Some(1)
            } else {
                None
            }
        } else {
            None
        };

        let move_now = if self.in_tunnel(nodes) {
            self.move_patterns.tunnel.advance()
        } else {
            match self.mode.current() {
                GhostMode::Freight => self.move_patterns.frightened.advance(),
                GhostMode::Scatter | GhostMode::Chase => match elroy_state {
                    Some(2) => self.move_patterns.blinky_elroy_two.advance(),
                    Some(1) => self.move_patterns.blinky_elroy_one.advance(),
                    _ => self.move_patterns.normal.advance(),
                },
                GhostMode::Spawn => unreachable!(),
            }
        };

        usize::from(move_now)
    }

    fn advance_position_step(&mut self, nodes: &NodeGroup) {
        self.position += self.direction.vector() * ARCADE_PIXEL_STEP;
        if self.overshot_target(nodes) {
            self.node = self.target;
            self.position = nodes.position(self.node);
            if self.consume_pending_reverse(nodes) {
                return;
            }
            let directions = self.valid_directions(nodes);
            let next_direction = match self.direction_method {
                DirectionMethod::Goal => self.goal_direction(&directions, nodes),
                DirectionMethod::Random => self.random_direction(&directions),
            };

            if let Some(portal) = nodes.portal(self.node) {
                self.node = portal;
                self.position = nodes.position(self.node);
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

    fn prime_direction(&mut self, nodes: &NodeGroup) {
        if self.target == self.node && self.consume_pending_reverse(nodes) {
            return;
        }

        if self.target != self.node {
            return;
        }

        let directions = self.valid_directions(nodes);
        let next_direction = match self.direction_method {
            DirectionMethod::Goal => self.goal_direction(&directions, nodes),
            DirectionMethod::Random => self.random_direction(&directions),
        };
        self.target = self.get_new_target(next_direction, nodes);
        if self.target != self.node {
            self.direction = next_direction;
        }
    }

    fn set_position(&mut self, nodes: &NodeGroup) {
        self.position = nodes.position(self.node);
    }

    fn apply_start_position_override(&mut self) {
        if let Some(position) = self.start_position_override {
            self.position = position;
        }
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
        let order = [
            Direction::Up,
            Direction::Right,
            Direction::Down,
            Direction::Left,
        ];
        let start = fastrand::usize(..order.len());
        for offset in 0..order.len() {
            let direction = order[(start + offset) % order.len()];
            if directions.contains(&direction) {
                return direction;
            }
        }

        directions[0]
    }

    fn goal_direction(&self, directions: &[Direction], nodes: &NodeGroup) -> Direction {
        let mut best = directions[0];
        let mut best_distance = f32::INFINITY;

        for direction in [
            Direction::Up,
            Direction::Left,
            Direction::Down,
            Direction::Right,
        ] {
            if !directions.contains(&direction) {
                continue;
            }
            let next_position =
                nodes.position(self.node) + direction.vector() * TILE_WIDTH as f32 - self.goal;
            let distance = next_position.magnitude_squared();
            if distance < best_distance {
                best = direction;
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

    fn consume_pending_reverse(&mut self, nodes: &NodeGroup) -> bool {
        if !self.reverse_pending || self.direction == Direction::Stop {
            return false;
        }

        self.reverse_pending = false;
        self.direction = self.direction.opposite();
        self.target = self.get_new_target(self.direction, nodes);
        true
    }

    fn in_tunnel(&self, nodes: &NodeGroup) -> bool {
        let current = nodes.position(self.node);
        let target = nodes.position(self.target);
        let tunnel_row = 17.0 * TILE_HEIGHT as f32;
        current.y == tunnel_row
            && target.y == tunnel_row
            && (current.x <= 6.0 * TILE_WIDTH as f32
                || target.x <= 6.0 * TILE_WIDTH as f32
                || current.x >= 21.0 * TILE_WIDTH as f32
                || target.x >= 21.0 * TILE_WIDTH as f32)
    }
}

impl GhostGroup {
    pub fn new(node: NodeId, nodes: &NodeGroup, level: u32) -> Self {
        Self {
            ghosts: [
                Ghost::new(GhostKind::Blinky, node, nodes, level),
                Ghost::new(GhostKind::Pinky, node, nodes, level),
                Ghost::new(GhostKind::Inky, node, nodes, level),
                Ghost::new(GhostKind::Clyde, node, nodes, level),
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
        context: GhostGroupUpdateContext,
    ) -> Vec<EntityKind> {
        let mut returned_to_normal = Vec::new();

        for kind in GhostKind::ALL {
            let ghost_context = GhostUpdateContext {
                pacman_position: context.pacman_position,
                pacman_direction: context.pacman_direction,
                blinky_position: self.ghost(GhostKind::Blinky).position(),
                level: context.level,
                dots_remaining: context.dots_remaining,
                elroy_enabled: context.elroy_enabled,
            };
            let ghost = self.ghost_mut(kind);
            if ghost.update(dt, nodes, ghost_context) {
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

    pub fn reset(&mut self, nodes: &NodeGroup, level: u32) {
        for ghost in &mut self.ghosts {
            ghost.reset(nodes, level);
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

fn arcade_offset(direction: Direction, tiles: i32, overflow_bug: bool, tile_size: f32) -> Vector2 {
    if overflow_bug && direction == Direction::Up {
        return Vector2::new(-(tiles as f32) * tile_size, -(tiles as f32) * tile_size);
    }

    direction.vector() * tile_size * tiles as f32
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
    use super::{Ghost, GhostGroup, GhostUpdateContext};
    use crate::{
        actors::{EntityKind, GhostKind},
        arcade::{ORIGINAL_FRAME_TIME, move_patterns},
        constants::ARCADE_PIXEL_STEP,
        modes::GhostMode,
        nodes::NodeGroup,
        pacman::Direction,
        vector::Vector2,
    };

    #[test]
    fn blinky_starts_in_scatter_mode() {
        let nodes = NodeGroup::pacman_maze();
        let ghost = Ghost::new(GhostKind::Blinky, nodes.start_node(), &nodes, 1);

        assert_eq!(ghost.mode(), GhostMode::Scatter);
        assert_eq!(ghost.position(), Vector2::new(16.0, 64.0));
    }

    #[test]
    fn freight_mode_changes_the_ghost_mode() {
        let nodes = NodeGroup::pacman_maze();
        let mut ghost = Ghost::new(GhostKind::Blinky, nodes.start_node(), &nodes, 1);

        ghost.start_freight();

        assert_eq!(ghost.mode(), GhostMode::Freight);
        ghost.update(
            0.0,
            &nodes,
            GhostUpdateContext {
                pacman_position: Vector2::new(0.0, 0.0),
                pacman_direction: Direction::Stop,
                blinky_position: Vector2::default(),
                level: 1,
                dots_remaining: 244,
                elroy_enabled: true,
            },
        );
    }

    #[test]
    fn spawn_mode_targets_the_spawn_node() {
        let mut nodes = NodeGroup::pacman_maze();
        nodes.create_home_nodes(11.5, 14.0);
        let spawn_node = nodes
            .get_node_from_tiles(13.5, 17.0)
            .expect("spawn node should exist");
        let mut ghost = Ghost::new(GhostKind::Blinky, nodes.start_node(), &nodes, 1);
        ghost.set_spawn_node(spawn_node);
        ghost.start_freight();
        ghost.start_spawn(&nodes);

        assert_eq!(ghost.mode(), GhostMode::Spawn);
    }

    #[test]
    fn ghost_group_contains_all_four_ghosts() {
        let nodes = NodeGroup::pacman_maze();
        let ghosts = GhostGroup::new(nodes.start_node(), &nodes, 1);
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
        let mut ghost = Ghost::new(GhostKind::Clyde, nodes.start_node(), &nodes, 1);

        ghost.update(
            0.0,
            &nodes,
            GhostUpdateContext {
                pacman_position: ghost.position(),
                pacman_direction: Direction::Right,
                blinky_position: Vector2::default(),
                level: 1,
                dots_remaining: 244,
                elroy_enabled: true,
            },
        );

        assert_eq!(ghost.mode(), GhostMode::Scatter);
    }

    #[test]
    fn ghosts_use_portals_in_arcade_rules() {
        let mut nodes = NodeGroup::from_pacman_layout(
            "
            + . + X X X +
            ",
        );
        nodes.set_portal_pair((0.0, 0.0), (6.0, 0.0));
        let left_entry = nodes
            .get_node_from_tiles(2.0, 0.0)
            .expect("left-side entry node should exist");
        let left_portal = nodes
            .get_node_from_tiles(0.0, 0.0)
            .expect("left portal should exist");
        let right_portal = nodes
            .get_node_from_tiles(6.0, 0.0)
            .expect("right portal should exist");
        let mut ghost = Ghost::new(GhostKind::Blinky, left_entry, &nodes, 1);

        ghost.update(
            0.0,
            &nodes,
            GhostUpdateContext {
                pacman_position: Vector2::default(),
                pacman_direction: Direction::Stop,
                blinky_position: Vector2::default(),
                level: 1,
                dots_remaining: 244,
                elroy_enabled: true,
            },
        );
        for _ in 0..64 {
            ghost.update(
                ORIGINAL_FRAME_TIME,
                &nodes,
                GhostUpdateContext {
                    pacman_position: Vector2::default(),
                    pacman_direction: Direction::Stop,
                    blinky_position: Vector2::default(),
                    level: 1,
                    dots_remaining: 244,
                    elroy_enabled: true,
                },
            );
            if ghost.current_node() == right_portal {
                break;
            }
        }

        assert_ne!(ghost.position(), nodes.position(left_portal));
        assert_eq!(ghost.current_node(), right_portal);
    }

    #[test]
    fn ghost_move_pattern_advances_the_expected_distance_per_cycle() {
        let nodes = NodeGroup::from_pacman_layout("+ . + . + . + . +");
        let start = nodes
            .get_node_from_tiles(0.0, 0.0)
            .expect("start node should exist");
        let mut ghost = Ghost::new(GhostKind::Pinky, start, &nodes, 1);

        ghost.update(
            0.0,
            &nodes,
            GhostUpdateContext {
                pacman_position: Vector2::new(128.0, 0.0),
                pacman_direction: Direction::Right,
                blinky_position: Vector2::default(),
                level: 1,
                dots_remaining: 244,
                elroy_enabled: true,
            },
        );
        for _ in 0..32 {
            ghost.update(
                ORIGINAL_FRAME_TIME,
                &nodes,
                GhostUpdateContext {
                    pacman_position: Vector2::new(128.0, 0.0),
                    pacman_direction: Direction::Right,
                    blinky_position: Vector2::default(),
                    level: 1,
                    dots_remaining: 244,
                    elroy_enabled: true,
                },
            );
        }

        let expected = move_patterns(1).ghost_normal.count_ones() as f32 * ARCADE_PIXEL_STEP;
        assert_eq!(ghost.position(), Vector2::new(expected, 0.0));
    }

    #[test]
    fn freight_reversal_waits_until_the_next_tile_center() {
        let nodes = NodeGroup::from_pacman_layout("+ . + . +");
        let start = nodes
            .get_node_from_tiles(2.0, 0.0)
            .expect("start node should exist");
        let left = nodes
            .neighbor(start, Direction::Left)
            .expect("left neighbor should exist");
        let mut ghost = Ghost::new(GhostKind::Blinky, start, &nodes, 1);
        ghost.direction = Direction::Left;
        ghost.target = left;
        ghost.position = (nodes.position(start) + nodes.position(left)) * 0.5;

        ghost.start_freight();

        assert_eq!(ghost.direction, Direction::Left);
        assert!(ghost.reverse_pending);

        for _ in 0..16 {
            ghost.advance_position_step(&nodes);
            if ghost.current_node() == left {
                break;
            }
        }

        assert_eq!(ghost.current_node(), left);
        assert_eq!(ghost.direction, Direction::Right);
        assert_eq!(ghost.target, start);
        assert!(!ghost.reverse_pending);
    }
}
