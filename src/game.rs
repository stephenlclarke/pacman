use std::sync::Arc;

use crate::{
    actors::{EntityKind, GhostKind},
    constants::SCREEN_HEIGHT,
    fruit::Fruit,
    ghosts::{Ghost, GhostGroup},
    modes::GhostMode,
    nodes::NodeGroup,
    pacman::{BasicPacman, Direction, NodeMovementMode, NodePacman},
    pause::PauseController,
    pellets::{PelletGroup, PelletKind},
    render::{FrameData, RenderedImage, Sprite, SpriteAnchor},
    sprites::{FruitSprites, GhostSprites, LifeSprites, MazeSprites, PacmanSprites},
    text::{StatusText, TextGroup},
    vector::Vector2,
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
    Level5,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level4Action {
    ShowEntities,
    ResetLevel,
    NextLevel,
    RestartGame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level5Action {
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
    Level5(Box<Level5State>),
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

#[derive(Clone, Debug)]
struct Level5State {
    nodes: NodeGroup,
    pacman: NodePacman,
    pellets: PelletGroup,
    ghosts: GhostGroup,
    fruit: Option<Fruit>,
    pause: PauseController<Level5Action>,
    level: u32,
    lives: u32,
    score: u32,
    fruit_thresholds_spawned: [bool; 2],
    background: Arc<RenderedImage>,
    text_group: TextGroup,
    life_sprites: LifeSprites,
    pacman_sprites: PacmanSprites,
    ghost_sprites: GhostSprites,
    fruit_sprites: FruitSprites,
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
            Stage::Level5 => Scene::Level5(Box::new(Level5State::new())),
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
            Scene::Level5(state) => state.update(dt, requested_direction, pause_requested),
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
            Scene::Level5(state) => state.append_renderables(&mut frame),
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

impl Level5State {
    const FRUIT_THRESHOLDS: [usize; 2] = [50, 140];

    fn new() -> Self {
        Self::start_level(1, 5, 0)
    }

    fn start_level(level: u32, lives: u32, score: u32) -> Self {
        let mut nodes = NodeGroup::pacman_maze();
        nodes.set_portal_pair((0.0, 17.0), (27.0, 17.0));
        let home = nodes.create_home_nodes(11.5, 14.0);
        nodes.connect_home_nodes(home, (12.0, 14.0), Direction::Left);
        nodes.connect_home_nodes(home, (15.0, 14.0), Direction::Right);

        let pacman_start = nodes
            .get_node_from_tiles(15.0, 26.0)
            .expect("level 5 pacman start node should exist");
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

        let mut text_group = TextGroup::new();
        text_group.update_score(score);
        text_group.update_level(level);
        text_group.show_status(StatusText::Ready);

        Self {
            nodes,
            pacman,
            pellets: PelletGroup::maze1(),
            ghosts,
            fruit: None,
            pause: PauseController::new(true),
            level,
            lives,
            score,
            fruit_thresholds_spawned: [false; 2],
            background: MazeSprites::new().construct_background(level),
            text_group,
            life_sprites: LifeSprites::new(lives),
            pacman_sprites: PacmanSprites::new(),
            ghost_sprites: GhostSprites::new(),
            fruit_sprites: FruitSprites::new(),
        }
    }

    fn update(&mut self, dt: f32, requested_direction: Direction, pause_requested: bool) {
        self.text_group.update(dt);
        self.pellets.update(dt);

        if !self.pause.paused() {
            self.pacman.update(dt, requested_direction, &self.nodes);
            self.pacman_sprites.update(dt, self.pacman.direction());

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
                self.text_group.show_status(StatusText::Paused);
                self.hide_entities();
            } else {
                self.text_group.hide_status();
                self.show_entities();
            }
        }
    }

    fn update_score(&mut self, points: u32) {
        self.score += points;
        self.text_group.update_score(self.score);
    }

    fn check_pellet_events(&mut self) {
        let Some(pellet) = self
            .pellets
            .try_eat(self.pacman.position(), self.pacman.collide_radius())
        else {
            return;
        };

        self.update_score(pellet.points());

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
            self.pause.start_timed_pause(3.0, Level5Action::NextLevel);
        }
    }

    fn check_ghost_events(&mut self) {
        let mut collision = None;
        for ghost in self.ghosts.iter() {
            if self
                .pacman
                .collide_check(ghost.position(), ghost.collide_radius())
            {
                collision = Some((ghost.kind(), ghost.mode(), ghost.position(), ghost.points()));
                break;
            }
        }

        let Some((ghost_kind, ghost_mode, ghost_position, ghost_points)) = collision else {
            return;
        };

        match ghost_mode {
            GhostMode::Freight => {
                self.pacman.hide();
                self.ghosts.ghost_mut(ghost_kind).hide();
                self.update_score(ghost_points);
                self.text_group.add_popup(
                    ghost_points.to_string(),
                    crate::constants::WHITE,
                    ghost_position.x,
                    ghost_position.y,
                );
                self.pause
                    .start_timed_pause(1.0, Level5Action::ShowEntities);
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
                self.life_sprites.remove_image();
                self.pacman.die();
                self.ghosts.hide();
                let action = if self.lives == 0 {
                    self.text_group.show_status(StatusText::GameOver);
                    Level5Action::RestartGame
                } else {
                    Level5Action::ResetLevel
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

        let fruit_position = fruit.position();
        let fruit_points = fruit.points();
        let hit_fruit = self
            .pacman
            .collide_check(fruit.position(), fruit.collide_radius());
        let expired = fruit.destroyed();

        if hit_fruit {
            self.update_score(fruit_points);
            self.text_group.add_popup(
                fruit_points.to_string(),
                crate::constants::WHITE,
                fruit_position.x,
                fruit_position.y,
            );
            self.fruit = None;
        } else if expired {
            self.fruit = None;
        }
    }

    fn handle_after_pause(&mut self, action: Level5Action) {
        match action {
            Level5Action::ShowEntities => {
                self.text_group.hide_status();
                self.show_entities();
            }
            Level5Action::ResetLevel => self.reset_level(),
            Level5Action::NextLevel => {
                *self = Self::start_level(self.level + 1, self.lives, self.score);
            }
            Level5Action::RestartGame => {
                *self = Self::start_level(1, 5, 0);
            }
        }
    }

    fn reset_level(&mut self) {
        self.pause.set_paused(true);
        self.pacman.reset(&self.nodes);
        self.pacman_sprites.reset();
        self.ghosts.reset(&self.nodes);
        self.nodes.deny_home_access_list(self.ghosts.entity_kinds());
        self.fruit = None;
        self.show_entities();
        self.text_group.show_status(StatusText::Ready);
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
        frame.background = Some(self.background.clone());
        self.pellets.append_renderables(frame);

        if let Some(fruit) = &self.fruit {
            frame.sprites.push(Sprite {
                image: self.fruit_sprites.image(),
                position: fruit.position(),
                anchor: SpriteAnchor::Center,
            });
        }
        if self.pacman.visible() {
            frame.sprites.push(Sprite {
                image: self.pacman_sprites.current(),
                position: self.pacman.position(),
                anchor: SpriteAnchor::Center,
            });
        }
        for ghost in self.ghosts.iter() {
            if ghost.visible() {
                frame.sprites.push(Sprite {
                    image: self
                        .ghost_sprites
                        .image(ghost.kind(), ghost.mode(), ghost.direction()),
                    position: ghost.position(),
                    anchor: SpriteAnchor::Center,
                });
            }
        }

        self.text_group.append_renderables(frame);
        let life_icon = self.life_sprites.image();
        let icon_y = SCREEN_HEIGHT as f32 - life_icon.height as f32;
        for index in 0..self.life_sprites.lives() {
            frame.sprites.push(Sprite {
                image: life_icon.clone(),
                position: Vector2::new(index as f32 * life_icon.width as f32, icon_y),
                anchor: SpriteAnchor::TopLeft,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Game, Level4Action, Level4State, Level5State, Stage};
    use crate::{nodes::NodeGroup, pacman::Direction, pellets::PelletGroup, render::FrameData};

    fn gameplay_node_count() -> usize {
        let mut nodes = NodeGroup::pacman_maze();
        let home = nodes.create_home_nodes(11.5, 14.0);
        nodes.connect_home_nodes(home, (12.0, 14.0), Direction::Left);
        nodes.connect_home_nodes(home, (15.0, 14.0), Direction::Right);
        nodes.node_count()
    }

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

        assert_eq!(
            frame.circles.len(),
            NodeGroup::pacman_maze().node_count() + PelletGroup::maze1().len() + 1
        );
        assert!(!frame.lines.is_empty());
    }

    #[test]
    fn eating_pellets_stage_consumes_the_starting_pellet() {
        let mut game = Game::new(Stage::EatingPellets);
        let before = game.frame().circles.len();

        game.update(0.0, Direction::Stop, false);
        let after = game.frame().circles.len();

        assert_eq!(
            before,
            NodeGroup::pacman_maze().node_count() + PelletGroup::maze1().len() + 1
        );
        assert_eq!(after, before - 1);
    }

    #[test]
    fn level3_stage_renders_ghosts_pellets_and_pacman() {
        let game = Game::new(Stage::Level3);
        let frame = game.frame();

        assert_eq!(
            frame.circles.len(),
            gameplay_node_count() + PelletGroup::maze1().len() + 2
        );
        assert!(!frame.lines.is_empty());
    }

    #[test]
    fn level3_stage_updates_without_panicking() {
        let mut game = Game::new(Stage::Level3);

        game.update(0.1, Direction::Right, false);
        game.update(0.1, Direction::Stop, false);

        let frame = game.frame();
        let initial = gameplay_node_count() + PelletGroup::maze1().len() + 2;
        assert!(frame.circles.len() < initial);
        assert!(frame.circles.len() >= initial.saturating_sub(10));
    }

    #[test]
    fn level4_stage_starts_paused_with_all_entities_visible() {
        let game = Game::new(Stage::Level4);
        let frame = game.frame();

        assert_eq!(
            frame.circles.len(),
            gameplay_node_count() + PelletGroup::maze1().len() + 5
        );
        assert!(!frame.lines.is_empty());
    }

    #[test]
    fn level4_player_pause_hides_pacman_and_ghosts() {
        let mut game = Game::new(Stage::Level4);

        game.update(0.0, Direction::Stop, true);
        game.update(0.0, Direction::Stop, true);

        let frame = game.frame();
        assert_eq!(
            frame.circles.len(),
            gameplay_node_count() + PelletGroup::maze1().len()
        );
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
        assert_eq!(
            frame.circles.len(),
            gameplay_node_count() + PelletGroup::maze1().len()
        );
    }

    #[test]
    fn level4_starts_on_level_one() {
        let state = Level4State::new();

        assert_eq!(state.level, 1);
    }

    #[test]
    fn level5_stage_renders_background_and_sprites() {
        let game = Game::new(Stage::Level5);
        let frame = game.frame();

        assert!(frame.background.is_some());
        assert!(frame.lines.is_empty());
        assert!(frame.sprites.len() >= 10);
    }

    #[test]
    fn level5_starts_on_level_one() {
        let state = Level5State::new();

        assert_eq!(state.level, 1);
    }

    #[test]
    fn level5_updates_the_score_display_when_points_are_added() {
        let mut state = Level5State::new();
        state.update_score(10);

        assert_eq!(state.score, 10);
    }
}
