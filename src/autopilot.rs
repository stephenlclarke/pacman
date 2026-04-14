use crate::{
    constants::{PACMAN_SPEED, TILE_WIDTH},
    fruit::Fruit,
    ghosts::{Ghost, GhostGroup},
    modes::GhostMode,
    nodes::{NodeGroup, NodeId},
    pacman::{Direction, NodePacman},
    pellets::{PelletGroup, PelletKind},
    vector::Vector2,
};

const GHOST_LOOKAHEAD: f32 = 0.75;
const DANGER_BLOCK_DISTANCE: f32 = TILE_WIDTH as f32 * 2.5;
const POWER_PELLET_TRIGGER_DISTANCE: f32 = TILE_WIDTH as f32 * 8.0;
const NORMAL_PELLET_REWARD: f32 = 140.0;
const POWER_PELLET_REWARD: f32 = 90.0;
const FRUIT_REWARD: f32 = 1_800.0;
const FREIGHT_REWARD: f32 = 3_000.0;
const TRAVEL_COST_SCALE: f32 = 8.0;
const REVERSE_COST: f32 = 20.0;
const BLOCKED_ROUTE_PENALTY: f32 = 50_000.0;

#[derive(Clone, Debug, Default)]
pub struct AutoPilot {
    active: bool,
}

#[derive(Clone, Copy, Debug)]
struct StartVariant {
    start_node: NodeId,
    reverse_now: bool,
}

#[derive(Clone, Copy, Debug)]
struct RouteChoice {
    requested_direction: Direction,
    score: f32,
}

#[derive(Clone, Copy, Debug)]
struct GhostSnapshot {
    position: Vector2,
    projected: Vector2,
    node: NodeId,
    target: NodeId,
    mode: GhostMode,
    points: u32,
    freight_remaining: Option<f32>,
    visible: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct RouteInfo {
    length: f32,
    normal_pellets: usize,
    power_pellets: usize,
    fruit_hit: bool,
    danger_penalty: f32,
    blocked: bool,
}

struct Planner<'a> {
    nodes: &'a NodeGroup,
    pacman: &'a NodePacman,
    pellets: &'a PelletGroup,
    ghosts: Vec<GhostSnapshot>,
    fruit: Option<&'a Fruit>,
    node_ids: Vec<NodeId>,
}

impl AutoPilot {
    pub fn active(&self) -> bool {
        self.active
    }

    pub fn toggle(&mut self) -> bool {
        self.active = !self.active;
        self.active
    }

    pub fn disable(&mut self) {
        self.active = false;
    }

    pub fn choose_direction(
        &self,
        nodes: &NodeGroup,
        pacman: &NodePacman,
        pellets: &PelletGroup,
        ghosts: &GhostGroup,
        fruit: Option<&Fruit>,
    ) -> Direction {
        if !self.active || pellets.is_empty() {
            return Direction::Stop;
        }

        Planner::new(nodes, pacman, pellets, ghosts, fruit).choose_direction()
    }
}

impl<'a> Planner<'a> {
    fn new(
        nodes: &'a NodeGroup,
        pacman: &'a NodePacman,
        pellets: &'a PelletGroup,
        ghosts: &'a GhostGroup,
        fruit: Option<&'a Fruit>,
    ) -> Self {
        Self {
            nodes,
            pacman,
            pellets,
            ghosts: ghosts.iter().map(GhostSnapshot::from_ghost).collect(),
            fruit,
            node_ids: nodes.node_ids().collect(),
        }
    }

    fn choose_direction(&self) -> Direction {
        let variants = self.start_variants();

        if let Some(choice) = self.best_choice(&variants, false) {
            return choice.requested_direction;
        }

        if let Some(choice) = self.best_choice(&variants, true) {
            return choice.requested_direction;
        }

        self.safest_fallback_direction()
    }

    fn best_choice(
        &self,
        variants: &[StartVariant],
        allow_power_pellets: bool,
    ) -> Option<RouteChoice> {
        variants
            .iter()
            .filter_map(|variant| self.best_choice_for_variant(*variant, allow_power_pellets))
            .max_by(|lhs, rhs| lhs.score.total_cmp(&rhs.score))
    }

    fn best_choice_for_variant(
        &self,
        variant: StartVariant,
        allow_power_pellets: bool,
    ) -> Option<RouteChoice> {
        let search = self.shortest_paths(variant.start_node);

        self.node_ids
            .iter()
            .copied()
            .filter_map(|target| {
                self.evaluate_target(variant, target, &search, allow_power_pellets)
            })
            .max_by(|lhs, rhs| lhs.score.total_cmp(&rhs.score))
    }

    fn evaluate_target(
        &self,
        variant: StartVariant,
        target: NodeId,
        search: &SearchTree,
        allow_power_pellets: bool,
    ) -> Option<RouteChoice> {
        let path = search.path_to(target)?;
        let route = self.route_segments(variant, &path);
        let route_info = self.analyze_route(&route);
        if route_info.blocked {
            return None;
        }

        let travel_time = route_info.length / PACMAN_SPEED;
        let freight_bonus = self.freight_bonus(target, travel_time);
        let fruit_bonus = self.fruit_bonus(&route, travel_time);

        if route_info.normal_pellets == 0
            && route_info.power_pellets == 0
            && freight_bonus == 0.0
            && fruit_bonus == 0.0
        {
            return None;
        }

        if route_info.power_pellets > 0
            && !allow_power_pellets
            && self.remaining_normal_pellets() > 0
            && !self.power_pellet_ready(&route)
        {
            return None;
        }

        let requested_direction = if variant.reverse_now {
            self.pacman.direction().opposite()
        } else {
            self.direction_from_path(&path)
        };

        let travel_tiles = route_info.length / TILE_WIDTH as f32;
        let pellet_reward = route_info.normal_pellets as f32 * NORMAL_PELLET_REWARD
            + route_info.power_pellets as f32 * POWER_PELLET_REWARD;
        let reverse_cost = if variant.reverse_now {
            REVERSE_COST
        } else {
            0.0
        };
        let score = pellet_reward + fruit_bonus + freight_bonus
            - travel_tiles * TRAVEL_COST_SCALE
            - route_info.danger_penalty
            - reverse_cost;

        Some(RouteChoice {
            requested_direction,
            score,
        })
    }

    fn safest_fallback_direction(&self) -> Direction {
        if self.pacman.direction() != Direction::Stop {
            let current_score = self.immediate_safety(self.pacman.direction());
            let reverse = self.pacman.direction().opposite();
            if self.immediate_safety(reverse) > current_score {
                return reverse;
            }
            return self.pacman.direction();
        }

        let node = self.pacman.current_node();
        Direction::cardinals()
            .into_iter()
            .filter(|&direction| {
                self.nodes
                    .can_travel(node, direction, crate::actors::EntityKind::Pacman)
            })
            .max_by(|&lhs, &rhs| {
                self.immediate_safety(lhs)
                    .total_cmp(&self.immediate_safety(rhs))
            })
            .unwrap_or(Direction::Stop)
    }

    fn immediate_safety(&self, direction: Direction) -> f32 {
        if direction == Direction::Stop {
            return 0.0;
        }

        let lookahead = self.pacman.position() + direction.vector() * TILE_WIDTH as f32 * 2.0;
        self.closest_danger_distance(lookahead)
    }

    fn start_variants(&self) -> Vec<StartVariant> {
        let mut variants = Vec::with_capacity(2);
        let current_node = self.pacman.current_node();
        if self.pacman.direction() == Direction::Stop {
            variants.push(StartVariant {
                start_node: current_node,
                reverse_now: false,
            });
            return variants;
        }

        variants.push(StartVariant {
            start_node: self.pacman.target(),
            reverse_now: false,
        });
        variants.push(StartVariant {
            start_node: current_node,
            reverse_now: true,
        });
        variants
    }

    fn shortest_paths(&self, start: NodeId) -> SearchTree {
        let node_count = self.node_ids.len();
        let mut distances = vec![f32::INFINITY; node_count];
        let mut previous = vec![None; node_count];
        let mut visited = vec![false; node_count];
        distances[start] = 0.0;

        for _ in 0..node_count {
            let current = (0..node_count)
                .filter(|&index| !visited[index] && distances[index].is_finite())
                .min_by(|&lhs, &rhs| distances[lhs].total_cmp(&distances[rhs]));
            let Some(current) = current else {
                break;
            };

            visited[current] = true;
            for direction in Direction::cardinals() {
                if !self
                    .nodes
                    .can_travel(current, direction, crate::actors::EntityKind::Pacman)
                {
                    continue;
                }

                let Some(next) = self.nodes.neighbor(current, direction) else {
                    continue;
                };

                let edge_length =
                    (self.nodes.position(next) - self.nodes.position(current)).magnitude();
                let candidate = distances[current]
                    + edge_length
                    + self.node_danger_cost(next).min(BLOCKED_ROUTE_PENALTY);
                if candidate < distances[next] {
                    distances[next] = candidate;
                    previous[next] = Some(current);
                }
            }
        }

        SearchTree {
            start,
            distances,
            previous,
        }
    }

    fn node_danger_cost(&self, node: NodeId) -> f32 {
        let position = self.nodes.position(node);
        let distance = self.closest_danger_distance(position);
        if !distance.is_finite() {
            return 0.0;
        }
        if distance < DANGER_BLOCK_DISTANCE {
            return BLOCKED_ROUTE_PENALTY;
        }

        let tiles = distance / TILE_WIDTH as f32;
        120.0 / (tiles + 0.5)
    }

    fn closest_danger_distance(&self, position: Vector2) -> f32 {
        self.ghosts
            .iter()
            .filter(|ghost| {
                ghost.visible && matches!(ghost.mode, GhostMode::Scatter | GhostMode::Chase)
            })
            .map(|ghost| {
                let current = (position - ghost.position).magnitude();
                let projected = (position - ghost.projected).magnitude();
                current.min(projected)
            })
            .fold(f32::INFINITY, f32::min)
    }

    fn analyze_route(&self, route: &[(Vector2, Vector2)]) -> RouteInfo {
        let mut info = RouteInfo::default();
        for &(start, end) in route {
            info.length += (end - start).magnitude();
        }

        for pellet in self.pellets.iter() {
            if !self.route_contains_position(route, pellet.position()) {
                continue;
            }

            match pellet.kind() {
                PelletKind::Pellet => info.normal_pellets += 1,
                PelletKind::PowerPellet => info.power_pellets += 1,
            }
        }

        if let Some(fruit) = self.fruit {
            info.fruit_hit = self.route_contains_position(route, fruit.position());
        }

        for sample in self.route_samples(route) {
            let distance = self.closest_danger_distance(sample);
            if !distance.is_finite() {
                continue;
            }
            if distance < DANGER_BLOCK_DISTANCE {
                info.blocked = true;
                info.danger_penalty = BLOCKED_ROUTE_PENALTY;
                return info;
            }

            let tiles = distance / TILE_WIDTH as f32;
            info.danger_penalty += 40.0 / (tiles + 0.5);
        }

        info
    }

    fn route_samples(&self, route: &[(Vector2, Vector2)]) -> Vec<Vector2> {
        let mut samples = vec![self.pacman.position()];
        for &(start, end) in route {
            samples.push(start);
            samples.push((start + end) * 0.5);
            samples.push(end);
        }
        samples
    }

    fn fruit_bonus(&self, route: &[(Vector2, Vector2)], travel_time: f32) -> f32 {
        let Some(fruit) = self.fruit else {
            return 0.0;
        };
        if !self.route_contains_position(route, fruit.position()) {
            return 0.0;
        }
        if travel_time > fruit.remaining_life() + 0.2 {
            return 0.0;
        }

        FRUIT_REWARD + fruit.points() as f32
    }

    fn freight_bonus(&self, target: NodeId, travel_time: f32) -> f32 {
        self.ghosts
            .iter()
            .filter(|ghost| ghost.visible && ghost.mode == GhostMode::Freight)
            .filter(|ghost| target == ghost.node || target == ghost.target)
            .filter_map(|ghost| {
                let remaining = ghost.freight_remaining?;
                (travel_time <= remaining + 0.3)
                    .then_some(FREIGHT_REWARD + ghost.points as f32 * 4.0 - travel_time * 80.0)
            })
            .fold(0.0, f32::max)
    }

    fn remaining_normal_pellets(&self) -> usize {
        self.pellets
            .iter()
            .filter(|pellet| pellet.kind() == PelletKind::Pellet)
            .count()
    }

    fn power_pellet_ready(&self, route: &[(Vector2, Vector2)]) -> bool {
        route.iter().any(|&(start, end)| {
            self.pellets.iter().any(|pellet| {
                pellet.kind() == PelletKind::PowerPellet
                    && segment_contains_position(start, end, pellet.position())
                    && self.ghost_near_position(pellet.position())
            })
        })
    }

    fn ghost_near_position(&self, position: Vector2) -> bool {
        self.ghosts
            .iter()
            .filter(|ghost| {
                ghost.visible && matches!(ghost.mode, GhostMode::Scatter | GhostMode::Chase)
            })
            .any(|ghost| {
                let current = (position - ghost.position).magnitude();
                let projected = (position - ghost.projected).magnitude();
                current.min(projected) <= POWER_PELLET_TRIGGER_DISTANCE
            })
    }

    fn route_segments(&self, variant: StartVariant, path: &[NodeId]) -> Vec<(Vector2, Vector2)> {
        let mut segments = Vec::new();
        let start_position = self.nodes.position(variant.start_node);
        if self.pacman.position() != start_position {
            segments.push((self.pacman.position(), start_position));
        }

        for window in path.windows(2) {
            let start = self.nodes.position(window[0]);
            let end = self.nodes.position(window[1]);
            segments.push((start, end));
        }

        segments
    }

    fn route_contains_position(&self, route: &[(Vector2, Vector2)], position: Vector2) -> bool {
        route
            .iter()
            .any(|&(start, end)| segment_contains_position(start, end, position))
    }

    fn direction_from_path(&self, path: &[NodeId]) -> Direction {
        let Some((&from, &to)) = path.first().zip(path.get(1)) else {
            return Direction::Stop;
        };
        Direction::cardinals()
            .into_iter()
            .find(|&direction| self.nodes.neighbor(from, direction) == Some(to))
            .unwrap_or(Direction::Stop)
    }
}

impl GhostSnapshot {
    fn from_ghost(ghost: &Ghost) -> Self {
        Self {
            position: ghost.position(),
            projected: ghost.position()
                + ghost.direction().vector() * ghost.speed() * GHOST_LOOKAHEAD,
            node: ghost.current_node(),
            target: ghost.target_node(),
            mode: ghost.mode(),
            points: ghost.points(),
            freight_remaining: ghost.freight_remaining(),
            visible: ghost.visible(),
        }
    }
}

#[derive(Clone, Debug)]
struct SearchTree {
    start: NodeId,
    distances: Vec<f32>,
    previous: Vec<Option<NodeId>>,
}

impl SearchTree {
    fn path_to(&self, goal: NodeId) -> Option<Vec<NodeId>> {
        if !self
            .distances
            .get(goal)
            .is_some_and(|distance| distance.is_finite())
        {
            return None;
        }

        let mut path = vec![goal];
        let mut current = goal;
        while current != self.start {
            current = self.previous.get(current).copied().flatten()?;
            path.push(current);
        }
        path.reverse();
        Some(path)
    }
}

fn segment_contains_position(start: Vector2, end: Vector2, position: Vector2) -> bool {
    if start.x == end.x && position.x == start.x {
        let min_y = start.y.min(end.y);
        let max_y = start.y.max(end.y);
        return position.y >= min_y && position.y <= max_y;
    }

    if start.y == end.y && position.y == start.y {
        let min_x = start.x.min(end.x);
        let max_x = start.x.max(end.x);
        return position.x >= min_x && position.x <= max_x;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::AutoPilot;
    use crate::{
        actors::GhostKind,
        fruit::Fruit,
        ghosts::GhostGroup,
        nodes::NodeGroup,
        pacman::{Direction, NodePacman},
        pellets::PelletGroup,
    };

    fn line_nodes() -> (NodeGroup, usize, usize, usize) {
        let nodes = NodeGroup::from_pacman_layout(
            "
            + . + . +
            ",
        );
        let left = nodes
            .get_node_from_tiles(0.0, 0.0)
            .expect("left node should exist");
        let center = nodes
            .get_node_from_tiles(2.0, 0.0)
            .expect("center node should exist");
        let right = nodes
            .get_node_from_tiles(4.0, 0.0)
            .expect("right node should exist");
        (nodes, left, center, right)
    }

    #[test]
    fn autopilot_chases_reachable_freight_ghosts() {
        let (nodes, left, center, right) = line_nodes();
        let mut pacman = NodePacman::new(center, &nodes);
        pacman.configure_start(center, Direction::Stop, None, &nodes);
        let pellets = PelletGroup::from_layout(
            "
            + . + . +
            ",
        );
        let mut ghosts = GhostGroup::new(left, &nodes);
        ghosts
            .ghost_mut(GhostKind::Blinky)
            .set_start_node(right, &nodes);
        ghosts.ghost_mut(GhostKind::Blinky).start_freight();

        let mut autopilot = AutoPilot::default();
        autopilot.toggle();

        assert_eq!(
            autopilot.choose_direction(&nodes, &pacman, &pellets, &ghosts, None),
            Direction::Right
        );
    }

    #[test]
    fn autopilot_prioritizes_fruit_when_it_is_on_the_route() {
        let (nodes, left, center, _) = line_nodes();
        let mut pacman = NodePacman::new(center, &nodes);
        pacman.configure_start(center, Direction::Stop, None, &nodes);
        let pellets = PelletGroup::from_layout(
            "
            + . + . +
            ",
        );
        let ghosts = GhostGroup::new(left, &nodes);
        let fruit = Fruit::new(center, &nodes);

        let mut autopilot = AutoPilot::default();
        autopilot.toggle();

        assert_eq!(
            autopilot.choose_direction(&nodes, &pacman, &pellets, &ghosts, Some(&fruit)),
            Direction::Right
        );
    }

    #[test]
    fn autopilot_delays_power_pellets_while_other_pellets_remain() {
        let nodes = NodeGroup::from_pacman_layout(
            "
            X X + p +
            X X . X X
            X X + X X
            X X X X X
            + . + X X
            ",
        );
        let center = nodes
            .get_node_from_tiles(2.0, 0.0)
            .expect("center node should exist");
        let ghost_holding = nodes
            .get_node_from_tiles(0.0, 4.0)
            .expect("ghost holding node should exist");
        let mut pacman = NodePacman::new(center, &nodes);
        pacman.configure_start(center, Direction::Stop, None, &nodes);
        let pellets = PelletGroup::from_layout(
            "
            X X + p +
            X X . X X
            X X + X X
            X X X X X
            + . + X X
            ",
        );
        let ghosts = GhostGroup::new(ghost_holding, &nodes);

        let mut autopilot = AutoPilot::default();
        autopilot.toggle();

        assert_eq!(
            autopilot.choose_direction(&nodes, &pacman, &pellets, &ghosts, None),
            Direction::Down
        );
    }

    #[test]
    fn autopilot_steers_away_from_nearby_dangerous_ghosts() {
        let nodes = NodeGroup::from_pacman_layout(
            "
            + . + . +
            X X . X X
            X X + X X
            ",
        );
        let center = nodes
            .get_node_from_tiles(2.0, 0.0)
            .expect("center node should exist");
        let right = nodes
            .get_node_from_tiles(4.0, 0.0)
            .expect("right node should exist");
        let left = nodes
            .get_node_from_tiles(0.0, 0.0)
            .expect("left node should exist");
        let mut pacman = NodePacman::new(center, &nodes);
        pacman.configure_start(center, Direction::Stop, None, &nodes);
        let pellets = PelletGroup::from_layout(
            "
            + . + . +
            X X . X X
            X X + X X
            ",
        );
        let mut ghosts = GhostGroup::new(left, &nodes);
        ghosts
            .ghost_mut(GhostKind::Blinky)
            .set_start_node(right, &nodes);

        let mut autopilot = AutoPilot::default();
        autopilot.toggle();

        assert_eq!(
            autopilot.choose_direction(&nodes, &pacman, &pellets, &ghosts, None),
            Direction::Down
        );
    }
}
