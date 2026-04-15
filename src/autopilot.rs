use crate::{
    actors::GhostKind,
    arcade::{ORIGINAL_FRAME_TIME, level_spec},
    constants::{PACMAN_SPEED, TILE_WIDTH},
    fruit::Fruit,
    ghosts::{Ghost, GhostGroup, GhostGroupUpdateContext},
    modes::GhostMode,
    nodes::{NodeGroup, NodeId},
    pacman::{Direction, NodePacman},
    pellets::{PelletGroup, PelletKind},
    vector::Vector2,
};

const GHOST_LOOKAHEAD: f32 = 0.75;
const FALLBACK_LOOKAHEAD: f32 = 2.0;
const DANGER_BLOCK_DISTANCE: f32 = TILE_WIDTH as f32 * 2.5;
const POWER_PELLET_TRIGGER_DISTANCE: f32 = TILE_WIDTH as f32 * 8.0;
const POWER_PELLET_EMERGENCY_DISTANCE: f32 = TILE_WIDTH as f32 * 3.5;
const NORMAL_PELLET_REWARD: f32 = 140.0;
const POWER_PELLET_REWARD: f32 = 90.0;
const FRUIT_REWARD: f32 = 1_800.0;
const FREIGHT_REWARD: f32 = 3_000.0;
const TRAVEL_COST_SCALE: f32 = 8.0;
const REVERSE_COST: f32 = 20.0;
const BLOCKED_ROUTE_PENALTY: f32 = 50_000.0;
const SAFETY_REWARD_SCALE: f32 = 45.0;
const SIMULATION_DT: f32 = ORIGINAL_FRAME_TIME;
const SETTLE_TIME: f32 = 1.0;
const ROUTE_COMMIT_PELLET_THRESHOLD: usize = 20;
const ENDGAME_STAGING_BONUS: f32 = 220.0;
const ENDGAME_STAGING_PROXIMITY_SCALE: f32 = 28.0;
const CLEANUP_RADIUS: f32 = TILE_WIDTH as f32 * 6.0;
const LATE_ENDGAME_CLEANUP_RADIUS: f32 = TILE_WIDTH as f32 * 10.0;
const CLEANUP_PROGRESS_SCALE: f32 = 90.0;
const CLEANUP_COMPLETE_BONUS: f32 = 140.0;
const CLEANUP_STRAGGLER_PENALTY: f32 = 160.0;
const LEVEL_CLEAR_BONUS: f32 = 2_500.0;
const TUNNEL_ESCAPE_BONUS: f32 = 180.0;
const ENDGAME_PORTAL_STAGING_BONUS: f32 = 260.0;
const PREFERRED_FREIGHT_KIND_BONUS: f32 = 240.0;
const ROUNDUP_BONUS_SCALE: f32 = 210.0;
const TRAP_NODE_PENALTY: f32 = 450.0;
const HALLWAY_TRAP_PENALTY: f32 = 180.0;
const TARGET_EVALUATION_LIMIT: usize = 18;

#[derive(Clone, Debug, Default)]
pub struct AutoPilot {
    active: bool,
    route: Option<PlannedRoute>,
}

#[derive(Clone, Copy, Debug)]
pub struct AutoPilotContext {
    pub level: u32,
    pub elroy_enabled: bool,
}

#[derive(Clone, Copy, Debug)]
struct StartVariant {
    start_node: NodeId,
    reverse_now: bool,
}

#[derive(Clone, Debug)]
struct RouteChoice {
    requested_direction: Direction,
    score: f32,
    path: Vec<NodeId>,
}

struct RouteScoreInput<'a> {
    variant: StartVariant,
    target: NodeId,
    path: &'a [NodeId],
    route: &'a [(Vector2, Vector2)],
    outcome: &'a SimOutcome,
    fruit_bonus: f32,
    allow_endgame_staging: bool,
}

#[derive(Clone, Debug)]
struct PlannedRoute {
    path: Vec<NodeId>,
}

#[derive(Clone, Copy, Debug)]
struct GhostSnapshot {
    kind: GhostKind,
    position: Vector2,
    projected: Vector2,
    mode: GhostMode,
    visible: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct RoundupSummary {
    nearby: usize,
    preferred: usize,
    emergency: bool,
    value: f32,
}

#[derive(Clone, Copy, Debug, Default)]
struct SimOutcome {
    travel_time: f32,
    normal_pellets: usize,
    power_pellets: usize,
    fruit_hit: bool,
    freight_score: f32,
    preferred_freight_hits: usize,
    min_danger_distance: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GoalProgress {
    Continue,
    Reached,
}

struct Planner<'a> {
    nodes: &'a NodeGroup,
    pacman: &'a NodePacman,
    pellets: &'a PelletGroup,
    ghost_group: &'a GhostGroup,
    level: u32,
    elroy_enabled: bool,
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
        self.route = None;
        self.active
    }

    pub fn disable(&mut self) {
        self.active = false;
        self.route = None;
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        self.route = None;
    }

    pub fn invalidate_route(&mut self) {
        self.route = None;
    }

    pub fn choose_direction(
        &mut self,
        nodes: &NodeGroup,
        pacman: &NodePacman,
        pellets: &PelletGroup,
        ghosts: &GhostGroup,
        fruit: Option<&Fruit>,
        context: AutoPilotContext,
    ) -> Direction {
        if !self.active || pellets.is_empty() {
            self.route = None;
            return Direction::Stop;
        }

        let planner = Planner::new(
            nodes,
            pacman,
            pellets,
            ghosts,
            fruit,
            context.level,
            context.elroy_enabled,
        );

        let at_decision_point = planner.at_decision_point();
        let emergency_replan = planner.emergency_replan_needed();

        if let Some(route) = self.route.as_ref()
            && let Some(direction) = planner.cached_direction_for_path(&route.path)
            && !at_decision_point
            && !emergency_replan
        {
            return direction;
        }

        if !at_decision_point && !emergency_replan {
            return pacman.direction();
        }

        if pellets.len() <= ROUTE_COMMIT_PELLET_THRESHOLD
            && let Some(route) = self.route.as_ref()
            && planner.plan_is_safe(&route.path)
            && let Some(direction) = planner.cached_direction_for_path(&route.path)
        {
            return direction;
        }

        let Some(choice) = planner.choose_route() else {
            self.route = None;
            return planner.safest_fallback_direction();
        };

        self.route = (pellets.len() <= ROUTE_COMMIT_PELLET_THRESHOLD).then(|| PlannedRoute {
            path: choice.path.clone(),
        });
        choice.requested_direction
    }
}

impl<'a> Planner<'a> {
    fn new(
        nodes: &'a NodeGroup,
        pacman: &'a NodePacman,
        pellets: &'a PelletGroup,
        ghosts: &'a GhostGroup,
        fruit: Option<&'a Fruit>,
        level: u32,
        elroy_enabled: bool,
    ) -> Self {
        Self {
            nodes,
            pacman,
            pellets,
            ghost_group: ghosts,
            level,
            elroy_enabled,
            ghosts: ghosts.iter().map(GhostSnapshot::from_ghost).collect(),
            fruit,
            node_ids: nodes.node_ids().collect(),
        }
    }

    fn choose_route(&self) -> Option<RouteChoice> {
        let variants = self.start_variants();
        self.best_choice(&variants, false)
            .or_else(|| self.best_choice(&variants, true))
    }

    fn at_decision_point(&self) -> bool {
        self.pacman.direction() == Direction::Stop
            || self.pacman.current_node() == self.pacman.target()
    }

    fn emergency_replan_needed(&self) -> bool {
        self.closest_danger_distance(self.pacman.position()) <= DANGER_BLOCK_DISTANCE
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
        let candidate_targets = self.candidate_targets(&search);

        candidate_targets
            .into_iter()
            .filter_map(|target| {
                self.evaluate_target(variant, target, &search, allow_power_pellets)
            })
            .max_by(|lhs, rhs| lhs.score.total_cmp(&rhs.score))
    }

    fn candidate_targets(&self, search: &SearchTree) -> Vec<NodeId> {
        let mut candidates: Vec<_> = self
            .node_ids
            .iter()
            .copied()
            .filter(|&node| {
                search
                    .distances
                    .get(node)
                    .is_some_and(|distance| distance.is_finite())
            })
            .collect();

        candidates.sort_by(|&lhs, &rhs| search.distances[lhs].total_cmp(&search.distances[rhs]));
        candidates.truncate(TARGET_EVALUATION_LIMIT);
        candidates
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
        let route_has_power_pellet = self.route_has_power_pellet(&route);
        if self.rejects_unready_power_pellet_route(
            route_has_power_pellet,
            &route,
            allow_power_pellets,
        ) {
            return None;
        }

        let initial_pacman = self.initial_pacman_for(variant);

        let planned_direction = self.route_request(self.nodes, &initial_pacman, &path);
        if self.immediate_reverse_rejected(&initial_pacman, planned_direction) {
            return None;
        }

        let outcome = self.simulate_path(variant, &path)?;
        let total_pellets_eaten = outcome.normal_pellets + outcome.power_pellets;
        if self.pellets.len() <= 3 && total_pellets_eaten == 0 {
            return None;
        }
        let fruit_bonus = self.fruit_bonus(&outcome);
        let rewardless_route = self.rewardless_route(&outcome, fruit_bonus);
        let allow_endgame_staging = self.allow_endgame_staging(rewardless_route);

        if self.rejects_rewardless_route(rewardless_route, allow_endgame_staging) {
            return None;
        }

        if self.rejects_unproductive_power_pellet_route(
            route_has_power_pellet,
            allow_power_pellets,
            &outcome,
        ) {
            return None;
        }

        let requested_direction = self.requested_direction_for(variant, planned_direction);

        if requested_direction == Direction::Stop {
            return None;
        }

        let score = self.score_route(RouteScoreInput {
            variant,
            target,
            path: &path,
            route: &route,
            outcome: &outcome,
            fruit_bonus,
            allow_endgame_staging,
        });

        Some(RouteChoice {
            requested_direction,
            score,
            path,
        })
    }

    fn rejects_unready_power_pellet_route(
        &self,
        route_has_power_pellet: bool,
        route: &[(Vector2, Vector2)],
        allow_power_pellets: bool,
    ) -> bool {
        route_has_power_pellet
            && !allow_power_pellets
            && self.remaining_normal_pellets() > 0
            && !self.power_pellet_ready(route)
    }

    fn initial_pacman_for(&self, variant: StartVariant) -> NodePacman {
        let mut initial_pacman = self.pacman.clone();
        if variant.reverse_now {
            initial_pacman.update(0.0, initial_pacman.direction().opposite(), self.nodes);
        }
        initial_pacman
    }

    fn immediate_reverse_rejected(
        &self,
        initial_pacman: &NodePacman,
        planned_direction: Direction,
    ) -> bool {
        initial_pacman.direction() != Direction::Stop
            && planned_direction == initial_pacman.direction().opposite()
    }

    fn fruit_bonus(&self, outcome: &SimOutcome) -> f32 {
        if outcome.fruit_hit {
            self.fruit
                .map_or(0.0, |fruit| FRUIT_REWARD + fruit.points() as f32)
        } else {
            0.0
        }
    }

    fn rewardless_route(&self, outcome: &SimOutcome, fruit_bonus: f32) -> bool {
        outcome.normal_pellets == 0
            && outcome.power_pellets == 0
            && outcome.freight_score == 0.0
            && fruit_bonus == 0.0
    }

    fn allow_endgame_staging(&self, rewardless_route: bool) -> bool {
        rewardless_route
            && self.pellets.len() <= ROUTE_COMMIT_PELLET_THRESHOLD
            && self.pellets.len() > 2
    }

    fn rejects_rewardless_route(
        &self,
        rewardless_route: bool,
        allow_endgame_staging: bool,
    ) -> bool {
        rewardless_route && !allow_endgame_staging
    }

    fn rejects_unproductive_power_pellet_route(
        &self,
        route_has_power_pellet: bool,
        allow_power_pellets: bool,
        outcome: &SimOutcome,
    ) -> bool {
        route_has_power_pellet
            && !allow_power_pellets
            && self.remaining_normal_pellets() > 0
            && outcome.freight_score == 0.0
    }

    fn requested_direction_for(
        &self,
        variant: StartVariant,
        planned_direction: Direction,
    ) -> Direction {
        if variant.reverse_now {
            self.pacman.direction().opposite()
        } else {
            planned_direction
        }
    }

    fn score_route(&self, input: RouteScoreInput<'_>) -> f32 {
        let travel_tiles =
            input.outcome.travel_time * self.nominal_pacman_speed() / TILE_WIDTH as f32;
        let pellet_reward = input.outcome.normal_pellets as f32 * NORMAL_PELLET_REWARD
            + input.outcome.power_pellets as f32 * POWER_PELLET_REWARD;
        let reverse_cost = if input.variant.reverse_now {
            REVERSE_COST
        } else {
            0.0
        };
        let safety_bonus = (input
            .outcome
            .min_danger_distance
            .min(TILE_WIDTH as f32 * 12.0)
            / TILE_WIDTH as f32)
            * SAFETY_REWARD_SCALE;
        let staging_bonus = if input.allow_endgame_staging {
            self.endgame_staging_bonus(input.target)
        } else {
            0.0
        };
        let cleanup_bonus = self.cleanup_bonus(input.route, input.target);
        let level_clear_bonus = ((input.outcome.normal_pellets + input.outcome.power_pellets)
            == self.pellets.len()) as u8 as f32
            * LEVEL_CLEAR_BONUS;
        let tunnel_bonus = self.tunnel_escape_bonus(input.path);
        let preferred_freight_bonus =
            input.outcome.preferred_freight_hits as f32 * PREFERRED_FREIGHT_KIND_BONUS;
        let roundup_bonus = self.power_pellet_roundup_bonus(input.route);
        let trap_penalty = self.target_trap_penalty(input.target);

        pellet_reward
            + input.fruit_bonus
            + input.outcome.freight_score
            + safety_bonus
            + staging_bonus
            + cleanup_bonus
            + level_clear_bonus
            + tunnel_bonus
            + preferred_freight_bonus
            + roundup_bonus
            - travel_tiles * TRAVEL_COST_SCALE
            - reverse_cost
            - trap_penalty
    }

    fn safest_fallback_direction(&self) -> Direction {
        self.fallback_candidates()
            .into_iter()
            .max_by(|&lhs, &rhs| {
                self.fallback_score(lhs)
                    .total_cmp(&self.fallback_score(rhs))
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

    fn fallback_candidates(&self) -> Vec<Direction> {
        let mut candidates = Vec::new();
        let decision_node = self.fallback_decision_node();

        if self.pacman.direction() != Direction::Stop {
            candidates.push(self.pacman.direction().opposite());
        }

        for direction in Direction::cardinals() {
            if direction == self.pacman.direction().opposite() {
                continue;
            }

            if self
                .nodes
                .can_travel(decision_node, direction, crate::actors::EntityKind::Pacman)
            {
                candidates.push(direction);
            }
        }

        candidates
    }

    fn fallback_decision_node(&self) -> NodeId {
        if self.pacman.direction() == Direction::Stop
            || self.pacman.current_node() == self.pacman.target()
        {
            return self.pacman.current_node();
        }

        self.nodes
            .portal(self.pacman.target())
            .unwrap_or(self.pacman.target())
    }

    fn fallback_score(&self, direction: Direction) -> f32 {
        let Some((survival_time, min_danger_distance)) = self.simulate_fallback(direction) else {
            return f32::NEG_INFINITY;
        };

        survival_time * 10_000.0 + min_danger_distance + self.immediate_safety(direction)
    }

    fn simulate_fallback(&self, direction: Direction) -> Option<(f32, f32)> {
        if direction == Direction::Stop {
            return None;
        }

        let mut nodes = self.nodes.clone();
        let mut pacman = self.pacman.clone();
        let mut ghosts = self.ghost_group.clone();
        let mut outcome = SimOutcome::default();
        let _rng_guard = PreserveRng::new();
        let mut elapsed = 0.0;
        let mut min_danger_distance = f32::INFINITY;
        let steps = (FALLBACK_LOOKAHEAD / SIMULATION_DT).ceil() as usize;

        for _ in 0..steps {
            pacman.set_frightened(ghosts.has_freight_mode());
            pacman.update(SIMULATION_DT, direction, &nodes);

            ghosts.update(
                SIMULATION_DT,
                &nodes,
                GhostGroupUpdateContext {
                    pacman_position: pacman.position(),
                    pacman_direction: pacman.direction(),
                    level: self.level,
                    dots_remaining: self.pellets.len(),
                    elroy_enabled: self.elroy_enabled,
                },
            );

            if !self.resolve_simulated_ghost_collision(
                &mut nodes,
                &pacman,
                &mut ghosts,
                &mut outcome,
            ) {
                return Some((elapsed, min_danger_distance));
            }

            min_danger_distance = min_danger_distance.min(Self::closest_danger_distance_for(
                &ghosts,
                pacman.position(),
            ));
            elapsed += SIMULATION_DT;
        }

        Some((elapsed, min_danger_distance))
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

            if let Some(portal) = self.nodes.portal(current) {
                let candidate = distances[current] + self.node_danger_cost(portal);
                if candidate < distances[portal] {
                    distances[portal] = candidate;
                    previous[portal] = Some(current);
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
        self.danger_pressure(position) + 120.0 / (tiles + 0.5)
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

    fn remaining_normal_pellets(&self) -> usize {
        self.pellets
            .iter()
            .filter(|pellet| pellet.kind() == PelletKind::Pellet)
            .count()
    }

    fn endgame_staging_bonus(&self, target: NodeId) -> f32 {
        let target_position = self.nodes.position(target);
        let closest_pellet_distance = self
            .pellets
            .iter()
            .map(|pellet| (target_position - pellet.position()).magnitude())
            .fold(f32::INFINITY, f32::min);
        let pellet_distance_tiles = closest_pellet_distance / TILE_WIDTH as f32;

        let mut bonus =
            ENDGAME_STAGING_BONUS - pellet_distance_tiles * ENDGAME_STAGING_PROXIMITY_SCALE;
        if self.pellets.len() <= 8
            && self.closest_danger_distance(self.pacman.position()) <= TILE_WIDTH as f32 * 8.0
            && self.nodes.portal(target).is_some()
        {
            bonus += ENDGAME_PORTAL_STAGING_BONUS;
        }

        bonus
    }

    fn cleanup_bonus(&self, route: &[(Vector2, Vector2)], target: NodeId) -> f32 {
        let target_position = self.nodes.position(target);
        let cleanup_radius = if self.pellets.len() <= 12 {
            LATE_ENDGAME_CLEANUP_RADIUS
        } else {
            CLEANUP_RADIUS
        };
        let complete_bonus = if self.pellets.len() <= 12 {
            CLEANUP_COMPLETE_BONUS * 2.0
        } else {
            CLEANUP_COMPLETE_BONUS
        };
        let straggler_penalty = if self.pellets.len() <= 12 {
            CLEANUP_STRAGGLER_PENALTY * 2.0
        } else {
            CLEANUP_STRAGGLER_PENALTY
        };
        let mut nearby_total = 0usize;
        let mut nearby_cleared = 0usize;

        for pellet in self.pellets.iter() {
            if (pellet.position() - target_position).magnitude() > cleanup_radius {
                continue;
            }

            nearby_total += 1;
            if self.route_contains_position(route, pellet.position()) {
                nearby_cleared += 1;
            }
        }

        if nearby_total == 0 || nearby_cleared == 0 {
            return 0.0;
        }

        let remaining = nearby_total - nearby_cleared;
        let completion = nearby_cleared as f32 / nearby_total as f32;
        let mut bonus = completion * completion * nearby_cleared as f32 * CLEANUP_PROGRESS_SCALE;
        if remaining == 0 {
            bonus += complete_bonus;
        } else if remaining <= 2 {
            bonus -= remaining as f32 * straggler_penalty;
        }

        bonus
    }

    fn route_has_power_pellet(&self, route: &[(Vector2, Vector2)]) -> bool {
        self.pellets.iter().any(|pellet| {
            pellet.kind() == PelletKind::PowerPellet
                && self.route_contains_position(route, pellet.position())
        })
    }

    fn power_pellet_ready(&self, route: &[(Vector2, Vector2)]) -> bool {
        let pacman_danger = self.closest_danger_distance(self.pacman.position());
        route.iter().any(|&(start, end)| {
            self.pellets.iter().any(|pellet| {
                pellet.kind() == PelletKind::PowerPellet
                    && segment_contains_position(start, end, pellet.position())
                    && self
                        .roundup_summary(pellet.position())
                        .worth_triggering(pacman_danger, self.pellets.len())
            })
        })
    }

    fn power_pellet_roundup_bonus(&self, route: &[(Vector2, Vector2)]) -> f32 {
        self.pellets
            .iter()
            .filter(|pellet| {
                pellet.kind() == PelletKind::PowerPellet
                    && self.route_contains_position(route, pellet.position())
            })
            .map(|pellet| self.roundup_summary(pellet.position()).value)
            .fold(0.0, f32::max)
    }

    fn roundup_summary(&self, position: Vector2) -> RoundupSummary {
        let mut summary = RoundupSummary::default();

        for ghost in self.ghosts.iter().filter(|ghost| {
            ghost.visible && matches!(ghost.mode, GhostMode::Scatter | GhostMode::Chase)
        }) {
            let distance = self.ghost_distance_to(position, *ghost);
            if distance > POWER_PELLET_TRIGGER_DISTANCE {
                continue;
            }

            let weight = self.ghost_weight(ghost.kind);
            summary.nearby += 1;
            summary.preferred +=
                usize::from(matches!(ghost.kind, GhostKind::Blinky | GhostKind::Pinky));
            summary.emergency |= distance <= POWER_PELLET_EMERGENCY_DISTANCE;
            summary.value += ((POWER_PELLET_TRIGGER_DISTANCE - distance) / TILE_WIDTH as f32)
                * ROUNDUP_BONUS_SCALE
                * weight;
        }

        if summary.nearby < 2 && !summary.emergency {
            summary.value *= 0.35;
        }

        if summary.preferred > 0 {
            summary.value += summary.preferred as f32 * PREFERRED_FREIGHT_KIND_BONUS;
        }

        summary
    }

    fn target_trap_penalty(&self, target: NodeId) -> f32 {
        if self.power_pellet_escape_hatch(target) {
            return 0.0;
        }

        let exits = self.exit_count(target);
        if exits >= 3 {
            return 0.0;
        }

        let distance = self.closest_danger_distance(self.nodes.position(target));
        if !distance.is_finite() {
            return 0.0;
        }

        let tiles = (distance / TILE_WIDTH as f32).max(0.5);
        if exits <= 1 {
            TRAP_NODE_PENALTY / tiles
        } else if self.nodes.portal(target).is_none() {
            HALLWAY_TRAP_PENALTY / tiles
        } else {
            0.0
        }
    }

    fn power_pellet_escape_hatch(&self, target: NodeId) -> bool {
        let target_position = self.nodes.position(target);
        let pacman_danger = self.closest_danger_distance(self.pacman.position());
        self.pellets.iter().any(|pellet| {
            pellet.kind() == PelletKind::PowerPellet
                && (pellet.position() - target_position).magnitude() <= TILE_WIDTH as f32 * 1.5
                && self
                    .roundup_summary(pellet.position())
                    .worth_triggering(pacman_danger, self.pellets.len())
        })
    }

    fn exit_count(&self, node: NodeId) -> usize {
        Direction::cardinals()
            .into_iter()
            .filter(|&direction| {
                self.nodes
                    .can_travel(node, direction, crate::actors::EntityKind::Pacman)
            })
            .count()
    }

    fn danger_pressure(&self, position: Vector2) -> f32 {
        self.ghosts
            .iter()
            .filter(|ghost| {
                ghost.visible && matches!(ghost.mode, GhostMode::Scatter | GhostMode::Chase)
            })
            .map(|ghost| {
                let distance = self.ghost_distance_to(position, *ghost) / TILE_WIDTH as f32;
                (70.0 * self.ghost_weight(ghost.kind)) / (distance + 0.5)
            })
            .sum()
    }

    fn ghost_distance_to(&self, position: Vector2, ghost: GhostSnapshot) -> f32 {
        let current = (position - ghost.position).magnitude();
        let projected = (position - ghost.projected).magnitude();
        current.min(projected)
    }

    fn ghost_weight(&self, kind: GhostKind) -> f32 {
        match kind {
            GhostKind::Blinky => 1.45,
            GhostKind::Pinky => 1.25,
            GhostKind::Inky => 1.0,
            GhostKind::Clyde => 0.9,
        }
    }

    fn route_segments(&self, variant: StartVariant, path: &[NodeId]) -> Vec<(Vector2, Vector2)> {
        let mut segments = Vec::new();
        let start_position = self.nodes.position(variant.start_node);
        if self.pacman.position() != start_position {
            segments.push((self.pacman.position(), start_position));
        }

        for window in path.windows(2) {
            if self.nodes.portal(window[0]) == Some(window[1]) {
                continue;
            }
            let start = self.nodes.position(window[0]);
            let end = self.nodes.position(window[1]);
            segments.push((start, end));
        }

        segments
    }

    fn route_length(&self, route: &[(Vector2, Vector2)]) -> f32 {
        route
            .iter()
            .map(|&(start, end)| (end - start).magnitude())
            .sum()
    }

    fn route_contains_position(&self, route: &[(Vector2, Vector2)], position: Vector2) -> bool {
        route
            .iter()
            .any(|&(start, end)| segment_contains_position(start, end, position))
    }

    fn route_request(&self, nodes: &NodeGroup, pacman: &NodePacman, path: &[NodeId]) -> Direction {
        let Some(anchor_index) = self.path_anchor_index_for(pacman, path) else {
            return Direction::Stop;
        };

        self.next_path_direction(nodes, path, anchor_index)
    }

    fn cached_direction_for_path(&self, path: &[NodeId]) -> Option<Direction> {
        let direction = self.route_request(self.nodes, self.pacman, path);
        (direction != Direction::Stop).then_some(direction)
    }

    fn plan_is_safe(&self, path: &[NodeId]) -> bool {
        self.simulate_committed_path(path).is_some()
    }

    fn next_path_direction(
        &self,
        nodes: &NodeGroup,
        path: &[NodeId],
        start_index: usize,
    ) -> Direction {
        for window in path[start_index..].windows(2) {
            if nodes.portal(window[0]) == Some(window[1]) {
                continue;
            }

            if let Some(direction) = Direction::cardinals()
                .into_iter()
                .find(|&direction| nodes.neighbor(window[0], direction) == Some(window[1]))
            {
                return direction;
            }
        }

        Direction::Stop
    }

    fn path_anchor_index_for(&self, pacman: &NodePacman, path: &[NodeId]) -> Option<usize> {
        let primary = if pacman.direction() == Direction::Stop {
            pacman.current_node()
        } else {
            pacman.target()
        };

        path.iter()
            .position(|&node| node == primary)
            .or_else(|| path.iter().position(|&node| node == pacman.current_node()))
    }

    fn simulate_path(&self, variant: StartVariant, path: &[NodeId]) -> Option<SimOutcome> {
        let mut pacman = self.pacman.clone();
        if variant.reverse_now && pacman.direction() != Direction::Stop {
            pacman.update(0.0, pacman.direction().opposite(), self.nodes);
        }

        self.simulate_pacman_path(pacman, path)
    }

    fn simulate_committed_path(&self, path: &[NodeId]) -> Option<SimOutcome> {
        self.simulate_pacman_path(self.pacman.clone(), path)
    }

    fn simulate_pacman_path(&self, mut pacman: NodePacman, path: &[NodeId]) -> Option<SimOutcome> {
        let goal = *path.last()?;
        let route = self.route_segments_from_state(&pacman, path);
        let max_time =
            (self.route_length(&route) / self.nominal_pacman_speed() + SETTLE_TIME + 6.0).max(1.5);
        let max_steps = (max_time / SIMULATION_DT).ceil() as usize;
        let mut nodes = self.nodes.clone();
        let mut pellets = self.pellets.clone();
        let mut ghosts = self.ghost_group.clone();
        let mut fruit = self.fruit.cloned();
        let mut outcome = SimOutcome {
            min_danger_distance: f32::INFINITY,
            ..SimOutcome::default()
        };
        let mut reached_goal_at = None;
        let mut elapsed = 0.0;
        let _rng_guard = PreserveRng::new();

        for _ in 0..max_steps {
            if !self.update_simulated_world(
                &mut nodes,
                &pacman,
                &mut ghosts,
                &mut pellets,
                &mut fruit,
                &mut outcome,
            ) {
                return None;
            }

            self.collect_simulated_fruit(&pacman, &mut fruit, &mut outcome);

            outcome.min_danger_distance =
                outcome
                    .min_danger_distance
                    .min(Self::closest_danger_distance_for(
                        &ghosts,
                        pacman.position(),
                    ));

            if let Some(completed) =
                Self::settled_goal_outcome(&mut outcome, reached_goal_at, elapsed)
            {
                return Some(completed);
            }

            if Self::advance_if_goal_settling(reached_goal_at, &mut elapsed)
                == GoalProgress::Reached
            {
                continue;
            }

            let requested_direction = self.route_request(&nodes, &pacman, path);
            pacman.set_frightened(ghosts.has_freight_mode());
            pacman.update(SIMULATION_DT, requested_direction, &nodes);
            elapsed += SIMULATION_DT;

            if reached_goal_at.is_none() && pacman.current_node() == goal {
                reached_goal_at = Some(elapsed);
            }
        }

        None
    }

    fn update_simulated_world(
        &self,
        nodes: &mut NodeGroup,
        pacman: &NodePacman,
        ghosts: &mut GhostGroup,
        pellets: &mut PelletGroup,
        fruit: &mut Option<Fruit>,
        outcome: &mut SimOutcome,
    ) -> bool {
        self.update_simulated_ghosts(ghosts, pacman, pellets.len(), nodes);
        self.update_simulated_fruit(fruit);
        self.collect_simulated_pellet(pellets, pacman, ghosts, outcome);
        self.resolve_simulated_ghost_collision(nodes, pacman, ghosts, outcome)
    }

    fn update_simulated_ghosts(
        &self,
        ghosts: &mut GhostGroup,
        pacman: &NodePacman,
        dots_remaining: usize,
        nodes: &NodeGroup,
    ) {
        ghosts.update(
            SIMULATION_DT,
            nodes,
            GhostGroupUpdateContext {
                pacman_position: pacman.position(),
                pacman_direction: pacman.direction(),
                level: self.level,
                dots_remaining,
                elroy_enabled: self.elroy_enabled,
            },
        );
    }

    fn update_simulated_fruit(&self, fruit: &mut Option<Fruit>) {
        if let Some(current_fruit) = fruit {
            current_fruit.update(SIMULATION_DT);
            if current_fruit.destroyed() {
                *fruit = None;
            }
        }
    }

    fn collect_simulated_pellet(
        &self,
        pellets: &mut PelletGroup,
        pacman: &NodePacman,
        ghosts: &mut GhostGroup,
        outcome: &mut SimOutcome,
    ) {
        if let Some(pellet) = pellets.try_eat(pacman.position(), pacman.collide_radius()) {
            match pellet.kind() {
                PelletKind::Pellet => outcome.normal_pellets += 1,
                PelletKind::PowerPellet => {
                    outcome.power_pellets += 1;
                    ghosts.start_freight();
                }
            }
        }
    }

    fn collect_simulated_fruit(
        &self,
        pacman: &NodePacman,
        fruit: &mut Option<Fruit>,
        outcome: &mut SimOutcome,
    ) {
        if let Some(current_fruit) = fruit
            && pacman.collide_check(current_fruit.position(), current_fruit.collide_radius())
        {
            outcome.fruit_hit = true;
            *fruit = None;
        }
    }

    fn settled_goal_outcome(
        outcome: &mut SimOutcome,
        reached_goal_at: Option<f32>,
        elapsed: f32,
    ) -> Option<SimOutcome> {
        let goal_time = reached_goal_at?;
        if elapsed < goal_time + SETTLE_TIME {
            return None;
        }

        outcome.travel_time = goal_time;
        Some(*outcome)
    }

    fn advance_if_goal_settling(reached_goal_at: Option<f32>, elapsed: &mut f32) -> GoalProgress {
        if reached_goal_at.is_none() {
            return GoalProgress::Continue;
        }

        *elapsed += SIMULATION_DT;
        GoalProgress::Reached
    }

    fn route_segments_from_state(
        &self,
        pacman: &NodePacman,
        path: &[NodeId],
    ) -> Vec<(Vector2, Vector2)> {
        let mut segments = Vec::new();
        let anchor = if pacman.direction() == Direction::Stop {
            pacman.current_node()
        } else {
            pacman.target()
        };
        let anchor_index = path.iter().position(|&node| node == anchor).unwrap_or(0);
        let anchor_position = self.nodes.position(anchor);

        if pacman.position() != anchor_position {
            segments.push((pacman.position(), anchor_position));
        }

        for window in path[anchor_index..].windows(2) {
            if self.nodes.portal(window[0]) == Some(window[1]) {
                continue;
            }

            let start = self.nodes.position(window[0]);
            let end = self.nodes.position(window[1]);
            segments.push((start, end));
        }

        segments
    }

    fn resolve_simulated_ghost_collision(
        &self,
        nodes: &mut NodeGroup,
        pacman: &NodePacman,
        ghosts: &mut GhostGroup,
        outcome: &mut SimOutcome,
    ) -> bool {
        for kind in GhostKind::ALL {
            let ghost = ghosts.ghost(kind);
            if !pacman.collide_check(ghost.position(), ghost.collide_radius()) {
                continue;
            }

            match ghost.mode() {
                GhostMode::Freight => {
                    let points = ghost.points();
                    if matches!(kind, GhostKind::Blinky | GhostKind::Pinky) {
                        outcome.preferred_freight_hits += 1;
                    }
                    ghosts.ghost_mut(kind).start_spawn(nodes);
                    nodes.allow_home_access(kind.entity());
                    ghosts.update_points();
                    outcome.freight_score += FREIGHT_REWARD + points as f32 * 4.0;
                }
                GhostMode::Spawn => {}
                GhostMode::Scatter | GhostMode::Chase => return false,
            }
        }

        true
    }

    fn closest_danger_distance_for(ghosts: &GhostGroup, position: Vector2) -> f32 {
        ghosts
            .iter()
            .filter(|ghost| {
                ghost.visible() && matches!(ghost.mode(), GhostMode::Scatter | GhostMode::Chase)
            })
            .map(|ghost| (position - ghost.position()).magnitude())
            .fold(f32::INFINITY, f32::min)
    }

    fn nominal_pacman_speed(&self) -> f32 {
        PACMAN_SPEED * level_spec(self.level).pacman_speed
    }

    fn tunnel_escape_bonus(&self, path: &[NodeId]) -> f32 {
        let late_endgame = self.pellets.len() <= 8;
        if !late_endgame
            && self.closest_danger_distance(self.pacman.position()) > TILE_WIDTH as f32 * 6.0
        {
            return 0.0;
        }

        let uses_tunnel = path
            .windows(2)
            .any(|window| self.nodes.portal(window[0]) == Some(window[1]));
        let bonus = if late_endgame {
            TUNNEL_ESCAPE_BONUS * 2.0
        } else {
            TUNNEL_ESCAPE_BONUS
        };
        uses_tunnel as u8 as f32 * bonus
    }
}

impl GhostSnapshot {
    fn from_ghost(ghost: &Ghost) -> Self {
        Self {
            kind: ghost.kind(),
            position: ghost.position(),
            projected: ghost.position()
                + ghost.direction().vector() * ghost.speed() * GHOST_LOOKAHEAD,
            mode: ghost.mode(),
            visible: ghost.visible(),
        }
    }
}

impl RoundupSummary {
    fn worth_triggering(self, pacman_danger: f32, pellets_remaining: usize) -> bool {
        self.emergency
            || self.nearby >= 2
            || (self.preferred >= 1 && self.nearby >= 1)
            || pacman_danger <= POWER_PELLET_TRIGGER_DISTANCE * 0.5
            || (pellets_remaining <= 80 && self.nearby >= 1)
    }
}

#[derive(Clone, Debug)]
struct SearchTree {
    start: NodeId,
    distances: Vec<f32>,
    previous: Vec<Option<NodeId>>,
}

struct PreserveRng(u64);

impl PreserveRng {
    fn new() -> Self {
        Self(fastrand::get_seed())
    }
}

impl Drop for PreserveRng {
    fn drop(&mut self) {
        fastrand::seed(self.0);
    }
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
    use super::{AutoPilot, AutoPilotContext, Planner};
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
        pacman.configure_start(center, Direction::Stop, None, None, &nodes);
        let pellets = PelletGroup::from_layout(
            "
            + . + . +
            ",
        );
        let mut ghosts = GhostGroup::new(left, &nodes, 1);
        ghosts
            .ghost_mut(GhostKind::Blinky)
            .set_start_node(right, &nodes);
        ghosts.ghost_mut(GhostKind::Blinky).start_freight();

        let mut autopilot = AutoPilot::default();
        autopilot.toggle();

        assert_eq!(
            autopilot.choose_direction(
                &nodes,
                &pacman,
                &pellets,
                &ghosts,
                None,
                AutoPilotContext {
                    level: 1,
                    elroy_enabled: true,
                },
            ),
            Direction::Right
        );
    }

    #[test]
    fn autopilot_prioritizes_fruit_when_it_is_on_the_route() {
        let (nodes, left, center, _) = line_nodes();
        let mut pacman = NodePacman::new(center, &nodes);
        pacman.configure_start(center, Direction::Stop, None, None, &nodes);
        let pellets = PelletGroup::from_layout(
            "
            + . + . +
            ",
        );
        let ghosts = GhostGroup::new(left, &nodes, 1);
        let fruit = Fruit::new(nodes.position(center));

        let mut autopilot = AutoPilot::default();
        autopilot.toggle();

        assert_eq!(
            autopilot.choose_direction(
                &nodes,
                &pacman,
                &pellets,
                &ghosts,
                Some(&fruit),
                AutoPilotContext {
                    level: 1,
                    elroy_enabled: true,
                },
            ),
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
        pacman.configure_start(center, Direction::Stop, None, None, &nodes);
        let pellets = PelletGroup::from_layout(
            "
            X X + p +
            X X . X X
            X X + X X
            X X X X X
            + . + X X
            ",
        );
        let ghosts = GhostGroup::new(ghost_holding, &nodes, 1);

        let mut autopilot = AutoPilot::default();
        autopilot.toggle();

        assert_eq!(
            autopilot.choose_direction(
                &nodes,
                &pacman,
                &pellets,
                &ghosts,
                None,
                AutoPilotContext {
                    level: 1,
                    elroy_enabled: true,
                },
            ),
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
        pacman.configure_start(center, Direction::Stop, None, None, &nodes);
        let pellets = PelletGroup::from_layout(
            "
            + . + . +
            X X . X X
            X X + X X
            ",
        );
        let mut ghosts = GhostGroup::new(left, &nodes, 1);
        ghosts
            .ghost_mut(GhostKind::Blinky)
            .set_start_node(right, &nodes);

        let mut autopilot = AutoPilot::default();
        autopilot.toggle();

        assert_eq!(
            autopilot.choose_direction(
                &nodes,
                &pacman,
                &pellets,
                &ghosts,
                None,
                AutoPilotContext {
                    level: 1,
                    elroy_enabled: true,
                },
            ),
            Direction::Down
        );
    }

    #[test]
    fn autopilot_uses_portal_routes_when_needed() {
        let mut nodes = NodeGroup::from_pacman_layout(
            "
            + . + X X X + . +
            ",
        );
        nodes.set_portal_pair((0.0, 0.0), (8.0, 0.0));
        let start = nodes
            .get_node_from_tiles(2.0, 0.0)
            .expect("start node should exist");
        let mut pacman = NodePacman::new(start, &nodes);
        pacman.configure_start(start, Direction::Stop, None, None, &nodes);
        let pellets = PelletGroup::from_layout(
            "
            X X X X X X . . +
            ",
        );
        let mut ghosts = GhostGroup::new(start, &nodes, 1);
        ghosts.hide();

        let mut autopilot = AutoPilot::default();
        autopilot.toggle();

        assert_eq!(
            autopilot.choose_direction(
                &nodes,
                &pacman,
                &pellets,
                &ghosts,
                None,
                AutoPilotContext {
                    level: 1,
                    elroy_enabled: true,
                },
            ),
            Direction::Left
        );
    }

    #[test]
    fn fallback_reverses_before_running_into_a_dead_end() {
        let (nodes, _, center, right) = line_nodes();
        let mut pacman = NodePacman::new(center, &nodes);
        pacman.configure_start(
            center,
            Direction::Right,
            Some(Direction::Right),
            None,
            &nodes,
        );
        let pellets = PelletGroup::from_layout(
            "
            + . + . +
            ",
        );
        let mut ghosts = GhostGroup::new(center, &nodes, 1);
        ghosts.hide();
        ghosts
            .ghost_mut(GhostKind::Blinky)
            .set_start_node(right, &nodes);
        let planner = Planner::new(&nodes, &pacman, &pellets, &ghosts, None, 1, true);

        assert_eq!(planner.safest_fallback_direction(), Direction::Left);
    }
}
