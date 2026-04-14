use crate::{
    actors::{EntityKind, GhostKind},
    fruit::Fruit,
    ghosts::{Ghost, GhostGroup},
    modes::GhostMode,
    nodes::NodeGroup,
    pacman::{BasicPacman, Direction, NodeMovementMode, NodePacman},
    pause::PauseController,
    pellets::{PelletGroup, PelletKind},
    render::FrameData,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    BlankScreen,
    BasicMovement,
    Nodes,
    NodeMovement1,
    NodeMovement2,
    NodeMovement3,
    MazeBasics,
    PacmanMaze,
    Portals,
    Pellets,
    EatingPellets,
    Level3,
    Level4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level4Action {
    ShowEntities,
    ResetLevel,
    NextLevel,
    RestartGame,
}

#[derive(Clone, Debug)]
pub struct Game {
    scene: Scene,
}

#[derive(Clone, Debug)]
enum Scene {
    BlankScreen,
    BasicMovement {
        pacman: BasicPacman,
    },
    Nodes {
        nodes: NodeGroup,
        pacman: BasicPacman,
    },
    NodeMovement {
        nodes: NodeGroup,
        pacman: NodePacman,
    },
    Maze {
        nodes: NodeGroup,
        pacman: NodePacman,
        pellets: Option<PelletGroup>,
        eat_pellets: bool,
    },
    Ghosts {
        nodes: NodeGroup,
        pacman: NodePacman,
        pellets: PelletGroup,
        ghost: Ghost,
    },
    Level4(Box<Level4State>),
}

#[derive(Clone, Debug)]
struct Level4State {
    nodes: NodeGroup,
    pacman: NodePacman,
    pellets: PelletGroup,
    ghosts: GhostGroup,
    fruit: Option<Fruit>,
    pause: PauseController<Level4Action>,
    level: u32,
    lives: u32,
    fruit_thresholds_spawned: [bool; 2],
}

impl Game {
    pub fn new(stage: Stage) -> Self {
        let scene = match stage {
            Stage::BlankScreen => Scene::BlankScreen,
            Stage::BasicMovement => Scene::BasicMovement {
                pacman: BasicPacman::new(),
            },
            Stage::Nodes => Scene::Nodes {
                nodes: NodeGroup::setup_test_nodes(),
                pacman: BasicPacman::new(),
            },
            Stage::NodeMovement1 => {
                let nodes = NodeGroup::setup_test_nodes();
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Teleport);
                Scene::NodeMovement { nodes, pacman }
            }
            Stage::NodeMovement2 => {
                let nodes = NodeGroup::setup_test_nodes();
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::OvershootStop);
                Scene::NodeMovement { nodes, pacman }
            }
            Stage::NodeMovement3 => {
                let nodes = NodeGroup::setup_test_nodes();
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                Scene::NodeMovement { nodes, pacman }
            }
            Stage::MazeBasics => {
                let nodes = NodeGroup::maze_basics();
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                Scene::Maze {
                    nodes,
                    pacman,
                    pellets: None,
                    eat_pellets: false,
                }
            }
            Stage::PacmanMaze => {
                let nodes = NodeGroup::pacman_maze();
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                Scene::Maze {
                    nodes,
                    pacman,
                    pellets: None,
                    eat_pellets: false,
                }
            }
            Stage::Portals => {
                let mut nodes = NodeGroup::pacman_maze();
                nodes.set_portal_pair((0.0, 17.0), (27.0, 17.0));
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                Scene::Maze {
                    nodes,
                    pacman,
                    pellets: None,
                    eat_pellets: false,
                }
            }
            Stage::Pellets => {
                let mut nodes = NodeGroup::pacman_maze();
                nodes.set_portal_pair((0.0, 17.0), (27.0, 17.0));
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                Scene::Maze {
                    nodes,
                    pacman,
                    pellets: Some(PelletGroup::maze1()),
                    eat_pellets: false,
                }
            }
            Stage::EatingPellets => {
                let mut nodes = NodeGroup::pacman_maze();
                nodes.set_portal_pair((0.0, 17.0), (27.0, 17.0));
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                Scene::Maze {
                    nodes,
                    pacman,
                    pellets: Some(PelletGroup::maze1()),
                    eat_pellets: true,
                }
            }
            Stage::Level3 => {
                let mut nodes = NodeGroup::pacman_maze();
                nodes.set_portal_pair((0.0, 17.0), (27.0, 17.0));
                let home = nodes.create_home_nodes(11.5, 14.0);
                nodes.connect_home_nodes(home, (12.0, 14.0), Direction::Left);
                nodes.connect_home_nodes(home, (15.0, 14.0), Direction::Right);
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                let mut ghost = Ghost::new(GhostKind::Blinky, nodes.start_node(), &nodes);
                let spawn_node = nodes
                    .get_node_from_tiles(13.5, 17.0)
                    .expect("level 3 spawn node should exist");
                ghost.set_spawn_node(spawn_node);

                Scene::Ghosts {
                    nodes,
                    pacman,
                    pellets: PelletGroup::maze1(),
                    ghost,
                }
            }
            Stage::Level4 => Scene::Level4(Box::new(Level4State::new())),
        };

        Self { scene }
    }

    pub fn update(&mut self, dt: f32, requested_direction: Direction, pause_requested: bool) {
        match &mut self.scene {
            Scene::BlankScreen => {}
            Scene::BasicMovement { pacman } | Scene::Nodes { pacman, .. } => {
                pacman.update(dt, requested_direction);
            }
            Scene::NodeMovement { nodes, pacman } => pacman.update(dt, requested_direction, nodes),
            Scene::Maze {
                nodes,
                pacman,
                pellets,
                eat_pellets,
            } => {
                pacman.update(dt, requested_direction, nodes);
                if let Some(pellets) = pellets {
                    pellets.update(dt);
                    if *eat_pellets {
                        pellets.try_eat(pacman.position(), pacman.collide_radius());
                    }
                }
            }
            Scene::Ghosts {
                nodes,
                pacman,
                pellets,
                ghost,
            } => {
                pacman.update(dt, requested_direction, nodes);
                ghost.update(
                    dt,
                    nodes,
                    pacman.position(),
                    pacman.direction(),
                    ghost.position(),
                );
                pellets.update(dt);

                if let Some(pellet) = pellets.try_eat(pacman.position(), pacman.collide_radius())
                    && pellet.kind() == PelletKind::PowerPellet
                {
                    ghost.start_freight();
                }

                if pacman.collide_check(ghost.position(), ghost.collide_radius())
                    && ghost.mode() == GhostMode::Freight
                {
                    ghost.start_spawn(nodes);
                }
            }
            Scene::Level4(state) => state.update(dt, requested_direction, pause_requested),
        }
    }

    pub fn frame(&self) -> FrameData {
        let mut frame = FrameData::default();

        match &self.scene {
            Scene::BlankScreen => {}
            Scene::BasicMovement { pacman } => frame.circles.push(pacman.renderable()),
            Scene::Nodes { nodes, pacman } => {
                nodes.append_renderables(&mut frame);
                frame.circles.push(pacman.renderable());
            }
            Scene::NodeMovement { nodes, pacman } => {
                nodes.append_renderables(&mut frame);
                frame.circles.push(pacman.renderable());
            }
            Scene::Maze {
                nodes,
                pacman,
                pellets,
                ..
            } => {
                nodes.append_renderables(&mut frame);
                if let Some(pellets) = pellets {
                    pellets.append_renderables(&mut frame);
                }
                frame.circles.push(pacman.renderable());
            }
            Scene::Ghosts {
                nodes,
                pacman,
                pellets,
                ghost,
            } => {
                nodes.append_renderables(&mut frame);
                pellets.append_renderables(&mut frame);
                frame.circles.push(pacman.renderable());
                frame.circles.push(ghost.renderable());
            }
            Scene::Level4(state) => state.append_renderables(&mut frame),
        }

        frame
    }
}

impl Level4State {
    const FRUIT_THRESHOLDS: [usize; 2] = [50, 140];

    fn new() -> Self {
        Self::start_level(1, 5)
    }

    fn start_level(level: u32, lives: u32) -> Self {
        let mut nodes = NodeGroup::pacman_maze();
        nodes.set_portal_pair((0.0, 17.0), (27.0, 17.0));
        let home = nodes.create_home_nodes(11.5, 14.0);
        nodes.connect_home_nodes(home, (12.0, 14.0), Direction::Left);
        nodes.connect_home_nodes(home, (15.0, 14.0), Direction::Right);

        let pacman_start = nodes
            .get_node_from_tiles(15.0, 26.0)
            .expect("level 4 pacman start node should exist");
        let mut pacman = NodePacman::new(pacman_start, &nodes, NodeMovementMode::Reversible);
        pacman.configure_start(pacman_start, Direction::Left, Some(Direction::Left), &nodes);

        let mut ghosts = GhostGroup::new(nodes.start_node(), &nodes);
        ghosts.ghost_mut(GhostKind::Blinky).set_start_node(
            nodes
                .get_node_from_tiles(13.5, 14.0)
                .expect("blinky start node should exist"),
            &nodes,
        );
        ghosts.ghost_mut(GhostKind::Pinky).set_start_node(
            nodes
                .get_node_from_tiles(13.5, 17.0)
                .expect("pinky start node should exist"),
            &nodes,
        );
        ghosts.ghost_mut(GhostKind::Inky).set_start_node(
            nodes
                .get_node_from_tiles(11.5, 17.0)
                .expect("inky start node should exist"),
            &nodes,
        );
        ghosts.ghost_mut(GhostKind::Clyde).set_start_node(
            nodes
                .get_node_from_tiles(15.5, 17.0)
                .expect("clyde start node should exist"),
            &nodes,
        );
        ghosts.set_spawn_node(
            nodes
                .get_node_from_tiles(13.5, 17.0)
                .expect("ghost spawn node should exist"),
        );

        nodes.deny_home_access(EntityKind::Pacman);
        nodes.deny_home_access_list(ghosts.entity_kinds());
        nodes.deny_access_list(13.5, 17.0, Direction::Left, ghosts.entity_kinds());
        nodes.deny_access_list(13.5, 17.0, Direction::Right, ghosts.entity_kinds());
        nodes.deny_access(11.5, 17.0, Direction::Right, EntityKind::Inky);
        nodes.deny_access(15.5, 17.0, Direction::Left, EntityKind::Clyde);
        nodes.deny_access_list(12.0, 14.0, Direction::Up, ghosts.entity_kinds());
        nodes.deny_access_list(15.0, 14.0, Direction::Up, ghosts.entity_kinds());
        nodes.deny_access_list(12.0, 26.0, Direction::Up, ghosts.entity_kinds());
        nodes.deny_access_list(15.0, 26.0, Direction::Up, ghosts.entity_kinds());

        Self {
            nodes,
            pacman,
            pellets: PelletGroup::maze1(),
            ghosts,
            fruit: None,
            pause: PauseController::new(true),
            level,
            lives,
            fruit_thresholds_spawned: [false; 2],
        }
    }

    fn update(&mut self, dt: f32, requested_direction: Direction, pause_requested: bool) {
        self.pellets.update(dt);

        if !self.pause.paused() {
            self.pacman.update(dt, requested_direction, &self.nodes);

            let returned_to_normal = self.ghosts.update(
                dt,
                &self.nodes,
                self.pacman.position(),
                self.pacman.direction(),
            );
            for entity in returned_to_normal {
                self.nodes.deny_home_access(entity);
            }

            if let Some(fruit) = &mut self.fruit {
                fruit.update(dt);
            }

            self.check_pellet_events();
            self.check_ghost_events();
            self.check_fruit_events();
        }

        if let Some(action) = self.pause.update(dt) {
            self.handle_after_pause(action);
        }

        if pause_requested && self.pacman.alive() && !self.pause.is_timed() {
            if self.pause.toggle() {
                self.hide_entities();
            } else {
                self.show_entities();
            }
        }
    }

    fn check_pellet_events(&mut self) {
        let Some(pellet) = self
            .pellets
            .try_eat(self.pacman.position(), self.pacman.collide_radius())
        else {
            return;
        };

        if self.pellets.num_eaten() == 30 {
            self.nodes
                .allow_access(11.5, 17.0, Direction::Right, EntityKind::Inky);
        }
        if self.pellets.num_eaten() == 70 {
            self.nodes
                .allow_access(15.5, 17.0, Direction::Left, EntityKind::Clyde);
        }

        if pellet.kind() == PelletKind::PowerPellet {
            self.ghosts.start_freight();
        }

        if self.pellets.is_empty() {
            self.hide_entities();
            self.pause.start_timed_pause(3.0, Level4Action::NextLevel);
        }
    }

    fn check_ghost_events(&mut self) {
        let mut collision = None;
        for ghost in self.ghosts.iter() {
            if self
                .pacman
                .collide_check(ghost.position(), ghost.collide_radius())
            {
                collision = Some((ghost.kind(), ghost.mode()));
                break;
            }
        }

        let Some((ghost_kind, ghost_mode)) = collision else {
            return;
        };

        match ghost_mode {
            GhostMode::Freight => {
                self.pacman.hide();
                self.ghosts.ghost_mut(ghost_kind).hide();
                self.pause
                    .start_timed_pause(1.0, Level4Action::ShowEntities);
                self.ghosts.ghost_mut(ghost_kind).start_spawn(&self.nodes);
                self.nodes.allow_home_access(ghost_kind.entity());
                self.ghosts.update_points();
            }
            GhostMode::Spawn => {}
            GhostMode::Scatter | GhostMode::Chase => {
                if !self.pacman.alive() {
                    return;
                }

                self.lives = self.lives.saturating_sub(1);
                self.pacman.die();
                self.ghosts.hide();
                let action = if self.lives == 0 {
                    Level4Action::RestartGame
                } else {
                    Level4Action::ResetLevel
                };
                self.pause.start_timed_pause(3.0, action);
            }
        }
    }

    fn check_fruit_events(&mut self) {
        for (index, threshold) in Self::FRUIT_THRESHOLDS.iter().enumerate() {
            if !self.fruit_thresholds_spawned[index]
                && self.pellets.num_eaten() >= *threshold
                && self.fruit.is_none()
            {
                let node = self
                    .nodes
                    .get_node_from_tiles(9.0, 20.0)
                    .expect("fruit spawn node should exist");
                self.fruit = Some(Fruit::new(node, &self.nodes));
                self.fruit_thresholds_spawned[index] = true;
                break;
            }
        }

        let Some(fruit) = &self.fruit else {
            return;
        };

        if self
            .pacman
            .collide_check(fruit.position(), fruit.collide_radius())
            || fruit.destroyed()
        {
            self.fruit = None;
        }
    }

    fn handle_after_pause(&mut self, action: Level4Action) {
        match action {
            Level4Action::ShowEntities => self.show_entities(),
            Level4Action::ResetLevel => self.reset_level(),
            Level4Action::NextLevel => {
                *self = Self::start_level(self.level + 1, self.lives);
            }
            Level4Action::RestartGame => {
                *self = Self::start_level(1, 5);
            }
        }
    }

    fn reset_level(&mut self) {
        self.pause.set_paused(true);
        self.pacman.reset(&self.nodes);
        self.ghosts.reset(&self.nodes);
        self.nodes.deny_home_access_list(self.ghosts.entity_kinds());
        self.fruit = None;
        self.show_entities();
    }

    fn show_entities(&mut self) {
        self.pacman.show();
        self.ghosts.show();
    }

    fn hide_entities(&mut self) {
        self.pacman.hide();
        self.ghosts.hide();
    }

    fn append_renderables(&self, frame: &mut FrameData) {
        self.nodes.append_renderables(frame);
        self.pellets.append_renderables(frame);
        if let Some(fruit) = &self.fruit {
            frame.circles.push(fruit.renderable());
        }
        if self.pacman.visible() {
            frame.circles.push(self.pacman.renderable());
        }
        for ghost in self.ghosts.iter() {
            if ghost.visible() {
                frame.circles.push(ghost.renderable());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Game, Level4Action, Level4State, Stage};
    use crate::{pacman::Direction, render::FrameData};

    #[test]
    fn nodes_stage_renders_graph_and_pacman() {
        let game = Game::new(Stage::Nodes);
        let frame = game.frame();

        assert!(!frame.lines.is_empty());
        assert_eq!(frame.circles.len(), 8);
    }

    #[test]
    fn reversible_stage_updates_without_panicking() {
        let mut game = Game::new(Stage::NodeMovement3);
        game.update(0.2, Direction::Right, false);
        game.update(0.2, Direction::Left, false);

        let frame = game.frame();
        assert_eq!(frame.circles.len(), 8);
    }

    #[test]
    fn pellets_stage_renders_nodes_pellets_and_pacman() {
        let game = Game::new(Stage::Pellets);
        let frame = game.frame();

        assert_eq!(frame.circles.len(), 313);
        assert!(!frame.lines.is_empty());
    }

    #[test]
    fn eating_pellets_stage_consumes_the_starting_pellet() {
        let mut game = Game::new(Stage::EatingPellets);
        let before = game.frame().circles.len();

        game.update(0.0, Direction::Stop, false);
        let after = game.frame().circles.len();

        assert_eq!(before, 313);
        assert_eq!(after, 312);
    }

    #[test]
    fn level3_stage_renders_ghosts_pellets_and_pacman() {
        let game = Game::new(Stage::Level3);
        let frame = game.frame();

        assert_eq!(frame.circles.len(), 322);
        assert!(!frame.lines.is_empty());
    }

    #[test]
    fn level3_stage_updates_without_panicking() {
        let mut game = Game::new(Stage::Level3);

        game.update(0.1, Direction::Right, false);
        game.update(0.1, Direction::Stop, false);

        let frame = game.frame();
        assert!(frame.circles.len() >= 315);
    }

    #[test]
    fn level4_stage_starts_paused_with_all_entities_visible() {
        let game = Game::new(Stage::Level4);
        let frame = game.frame();

        assert_eq!(frame.circles.len(), 325);
        assert!(!frame.lines.is_empty());
    }

    #[test]
    fn level4_player_pause_hides_pacman_and_ghosts() {
        let mut game = Game::new(Stage::Level4);

        game.update(0.0, Direction::Stop, true);
        game.update(0.0, Direction::Stop, true);

        let frame = game.frame();
        assert_eq!(frame.circles.len(), 319);
    }

    #[test]
    fn timed_pause_ignores_player_pause_requests() {
        let mut state = Level4State::new();
        state
            .pause
            .start_timed_pause(1.0, Level4Action::ShowEntities);
        state.hide_entities();

        state.update(0.0, Direction::Stop, true);

        let mut frame = FrameData::default();
        state.append_renderables(&mut frame);
        assert_eq!(frame.circles.len(), 320);
    }

    #[test]
    fn level4_starts_on_level_one() {
        let state = Level4State::new();

        assert_eq!(state.level, 1);
    }
}
