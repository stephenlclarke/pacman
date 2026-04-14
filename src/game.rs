use std::sync::Arc;

use crate::{
    actors::{EntityKind, GhostKind},
    constants::{
        BUTTON_CLICK, BUTTON_COLOR, BUTTON_HOVER, SCREEN_HEIGHT, SCREEN_WIDTH, TILE_HEIGHT,
        TILE_WIDTH, WHITE, YELLOW,
    },
    fruit::Fruit,
    ghosts::{Ghost, GhostGroup},
    mazedata::MazeSpec,
    modes::GhostMode,
    nodes::NodeGroup,
    pacman::{BasicPacman, Direction, NodeMovementMode, NodePacman},
    pause::PauseController,
    pellets::{PelletGroup, PelletKind},
    render::{FrameData, RenderedImage, Sprite, SpriteAnchor},
    sprites::{FruitSprites, GhostSprites, LifeSprites, MazeSprites, PacmanSprites},
    text::{StatusText, TextGroup, rasterize_text_image},
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
    PacmanDeath,
    LevelFlash,
    MoreFruit,
    MoreMazes,
    AddTitleScreen,
    AddButtons,
    AddSoundMusic,
    Level7,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level6Action {
    ShowEntities,
    ResetLevel,
    NextLevel,
    RestartGame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Level6Config {
    flash_background: bool,
    more_fruit: bool,
    more_mazes: bool,
    return_to_title: bool,
}

impl Level6Config {
    const fn pacman_death() -> Self {
        Self {
            flash_background: false,
            more_fruit: false,
            more_mazes: false,
            return_to_title: false,
        }
    }

    const fn level_flash() -> Self {
        Self {
            flash_background: true,
            more_fruit: false,
            more_mazes: false,
            return_to_title: false,
        }
    }

    const fn more_fruit() -> Self {
        Self {
            flash_background: true,
            more_fruit: true,
            more_mazes: false,
            return_to_title: false,
        }
    }

    const fn more_mazes() -> Self {
        Self {
            flash_background: true,
            more_fruit: true,
            more_mazes: true,
            return_to_title: false,
        }
    }

    const fn level7() -> Self {
        Self {
            flash_background: true,
            more_fruit: true,
            more_mazes: true,
            return_to_title: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameEvent {
    TitleScreenEntered,
    ButtonClicked,
    GameStarted,
    SmallPelletEaten,
    PowerPelletEaten,
    GhostEaten,
    FruitEaten,
    PacmanDied,
    LevelCompleted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Level7Config {
    buttons: bool,
}

impl Level7Config {
    const fn title_only() -> Self {
        Self { buttons: false }
    }

    const fn buttons() -> Self {
        Self { buttons: true }
    }

    const fn audio() -> Self {
        Self { buttons: true }
    }
}

#[derive(Debug)]
pub struct Game {
    scene: Scene,
}

#[derive(Debug)]
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
    Level6(Box<Level6State>),
    Level7(Box<Level7State>),
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

#[derive(Clone, Debug)]
struct Level6State {
    nodes: NodeGroup,
    pacman: NodePacman,
    pellets: PelletGroup,
    ghosts: GhostGroup,
    fruit: Option<Fruit>,
    pause: PauseController<Level6Action>,
    level: u32,
    lives: u32,
    score: u32,
    fruit_thresholds_spawned: [bool; 2],
    background: Arc<RenderedImage>,
    background_norm: Arc<RenderedImage>,
    background_flash: Arc<RenderedImage>,
    flash_background: bool,
    flash_time: f32,
    flash_timer: f32,
    text_group: TextGroup,
    life_sprites: LifeSprites,
    pacman_sprites: PacmanSprites,
    ghost_sprites: GhostSprites,
    fruit_sprites: FruitSprites,
    fruit_captured: Vec<usize>,
    config: Level6Config,
    maze_spec: MazeSpec,
    events: Vec<GameEvent>,
    return_to_title_requested: bool,
}

#[derive(Clone, Debug)]
struct TitleButtonState {
    position: Vector2,
    size: Vector2,
    normal_image: Arc<RenderedImage>,
    hover_image: Arc<RenderedImage>,
    pressed_image: Arc<RenderedImage>,
    label_image: Arc<RenderedImage>,
    hovered: bool,
    pressed: bool,
}

#[derive(Clone, Debug)]
struct TitleScreenState {
    title_image: Arc<RenderedImage>,
    prompt_image: Arc<RenderedImage>,
    button: Option<TitleButtonState>,
}

#[derive(Clone, Debug)]
struct Level7State {
    config: Level7Config,
    title_screen: TitleScreenState,
    gameplay: Option<Level6State>,
    events: Vec<GameEvent>,
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
            Stage::PacmanDeath => {
                Scene::Level6(Box::new(Level6State::new(Level6Config::pacman_death())))
            }
            Stage::LevelFlash => {
                Scene::Level6(Box::new(Level6State::new(Level6Config::level_flash())))
            }
            Stage::MoreFruit => {
                Scene::Level6(Box::new(Level6State::new(Level6Config::more_fruit())))
            }
            Stage::MoreMazes => {
                Scene::Level6(Box::new(Level6State::new(Level6Config::more_mazes())))
            }
            Stage::AddTitleScreen => {
                Scene::Level7(Box::new(Level7State::new(Level7Config::title_only())))
            }
            Stage::AddButtons => Scene::Level7(Box::new(Level7State::new(Level7Config::buttons()))),
            Stage::AddSoundMusic | Stage::Level7 => {
                Scene::Level7(Box::new(Level7State::new(Level7Config::audio())))
            }
        };

        Self { scene }
    }

    pub fn update(&mut self, dt: f32, requested_direction: Direction, pause_requested: bool) {
        self.update_with_input(dt, requested_direction, pause_requested, false, None, None);
    }

    pub fn update_with_input(
        &mut self,
        dt: f32,
        requested_direction: Direction,
        pause_requested: bool,
        start_requested: bool,
        mouse_position: Option<Vector2>,
        mouse_click_position: Option<Vector2>,
    ) {
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
            Scene::Level6(state) => state.update(dt, requested_direction, pause_requested),
            Scene::Level7(state) => state.update(
                dt,
                requested_direction,
                pause_requested,
                start_requested,
                mouse_position,
                mouse_click_position,
            ),
        }
    }

    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        match &mut self.scene {
            Scene::Level6(state) => state.drain_events(),
            Scene::Level7(state) => state.drain_events(),
            _ => Vec::new(),
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
            Scene::Level6(state) => state.append_renderables(&mut frame),
            Scene::Level7(state) => state.append_renderables(&mut frame),
        }

        frame
    }
}

fn build_gameplay_level(maze_spec: MazeSpec) -> (NodeGroup, NodePacman, PelletGroup, GhostGroup) {
    let mut nodes = NodeGroup::from_pacman_layout(maze_spec.layout);
    for &(left, right) in maze_spec.portal_pairs {
        nodes.set_portal_pair(left, right);
    }

    let home = nodes.create_home_nodes(maze_spec.home_offset.0, maze_spec.home_offset.1);
    nodes.connect_home_nodes(home, maze_spec.home_connect_left, Direction::Left);
    nodes.connect_home_nodes(home, maze_spec.home_connect_right, Direction::Right);

    let node_at = |nodes: &NodeGroup, position: (f32, f32), label: &str| {
        nodes
            .get_node_from_tiles(position.0, position.1)
            .unwrap_or_else(|| panic!("{label} should exist"))
    };

    let pacman_start = node_at(&nodes, maze_spec.pacman_start, "pacman start node");
    let mut pacman = NodePacman::new(pacman_start, &nodes, NodeMovementMode::Reversible);
    pacman.configure_start(pacman_start, Direction::Left, Some(Direction::Left), &nodes);

    let mut ghosts = GhostGroup::new(nodes.start_node(), &nodes);
    ghosts.ghost_mut(GhostKind::Blinky).set_start_node(
        node_at(&nodes, maze_spec.blinky_start(), "blinky start node"),
        &nodes,
    );
    ghosts.ghost_mut(GhostKind::Pinky).set_start_node(
        node_at(&nodes, maze_spec.pinky_start(), "pinky start node"),
        &nodes,
    );
    ghosts.ghost_mut(GhostKind::Inky).set_start_node(
        node_at(&nodes, maze_spec.inky_start(), "inky start node"),
        &nodes,
    );
    ghosts.ghost_mut(GhostKind::Clyde).set_start_node(
        node_at(&nodes, maze_spec.clyde_start(), "clyde start node"),
        &nodes,
    );
    ghosts.set_spawn_node(node_at(&nodes, maze_spec.spawn_node(), "ghost spawn node"));

    nodes.deny_home_access(EntityKind::Pacman);
    nodes.deny_home_access_list(ghosts.entity_kinds());
    for (direction, position) in maze_spec.deny_ghost_access_positions() {
        nodes.deny_access_list(position.0, position.1, direction, ghosts.entity_kinds());
    }

    let (direction, position, ghost) = maze_spec.inky_start_restriction();
    nodes.deny_access(position.0, position.1, direction, ghost.entity());
    let (direction, position, ghost) = maze_spec.clyde_start_restriction();
    nodes.deny_access(position.0, position.1, direction, ghost.entity());
    for &(col, row) in maze_spec.ghost_deny_up {
        nodes.deny_access_list(col, row, Direction::Up, ghosts.entity_kinds());
    }

    let pellets = PelletGroup::from_layout(maze_spec.layout);

    (nodes, pacman, pellets, ghosts)
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
                image: self.fruit_sprites.image(0),
                position: sprite_draw_position(fruit.position()),
                anchor: SpriteAnchor::TopLeft,
            });
        }
        if self.pacman.visible() {
            frame.sprites.push(Sprite {
                image: self.pacman_sprites.current(),
                position: sprite_draw_position(self.pacman.position()),
                anchor: SpriteAnchor::TopLeft,
            });
        }
        for ghost in self.ghosts.iter() {
            if ghost.visible() {
                frame.sprites.push(Sprite {
                    image: self
                        .ghost_sprites
                        .image(ghost.kind(), ghost.mode(), ghost.direction()),
                    position: sprite_draw_position(ghost.position()),
                    anchor: SpriteAnchor::TopLeft,
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

impl Level6State {
    const FRUIT_THRESHOLDS: [usize; 2] = [50, 140];
    const FLASH_TIME: f32 = 0.2;

    fn new(config: Level6Config) -> Self {
        Self::start_level(1, 5, 0, Vec::new(), config)
    }

    fn start_level(
        level: u32,
        lives: u32,
        score: u32,
        fruit_captured: Vec<usize>,
        config: Level6Config,
    ) -> Self {
        let maze_spec = MazeSpec::for_level(level, config.more_mazes);
        let (nodes, pacman, pellets, ghosts) = build_gameplay_level(maze_spec);
        let maze_sprites = MazeSprites::from_layout(maze_spec.layout, maze_spec.rotation);
        let background_norm = maze_sprites.construct_background(level);
        let background_flash = if config.flash_background {
            maze_sprites.construct_flash_background()
        } else {
            background_norm.clone()
        };

        let mut text_group = TextGroup::new();
        text_group.update_score(score);
        text_group.update_level(level);
        text_group.show_status(StatusText::Ready);

        Self {
            nodes,
            pacman,
            pellets,
            ghosts,
            fruit: None,
            pause: PauseController::new(true),
            level,
            lives,
            score,
            fruit_thresholds_spawned: [false; 2],
            background: background_norm.clone(),
            background_norm,
            background_flash,
            flash_background: false,
            flash_time: Self::FLASH_TIME,
            flash_timer: 0.0,
            text_group,
            life_sprites: LifeSprites::new(lives),
            pacman_sprites: PacmanSprites::new(),
            ghost_sprites: GhostSprites::new(),
            fruit_sprites: FruitSprites::new(),
            fruit_captured,
            config,
            maze_spec,
            events: Vec::new(),
            return_to_title_requested: false,
        }
    }

    fn update(&mut self, dt: f32, requested_direction: Direction, pause_requested: bool) {
        self.text_group.update(dt);
        self.pellets.update(dt);

        if !self.pause.paused() {
            self.ghosts.update(
                dt,
                &self.nodes,
                self.pacman.position(),
                self.pacman.direction(),
            );

            if let Some(fruit) = &mut self.fruit {
                fruit.update(dt);
            }

            self.check_pellet_events();
            self.check_ghost_events();
            self.check_fruit_events();
        }

        if self.pacman.alive() {
            if !self.pause.paused() {
                self.pacman.update(dt, requested_direction, &self.nodes);
                self.pacman_sprites
                    .update_for_state(dt, self.pacman.direction(), true);
            }
        } else {
            self.pacman_sprites
                .update_for_state(dt, self.pacman.direction(), false);
        }

        if self.flash_background {
            self.flash_timer += dt;
            if self.flash_timer >= self.flash_time {
                self.flash_timer = 0.0;
                if Arc::ptr_eq(&self.background, &self.background_norm) {
                    self.background = self.background_flash.clone();
                } else {
                    self.background = self.background_norm.clone();
                }
            }
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

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    fn update_score(&mut self, points: u32) {
        self.score += points;
        self.text_group.update_score(self.score);
    }

    fn fruit_level_index(&self) -> u32 {
        if self.config.more_fruit {
            self.level.saturating_sub(1)
        } else {
            0
        }
    }

    fn check_pellet_events(&mut self) {
        let Some(pellet) = self
            .pellets
            .try_eat(self.pacman.position(), self.pacman.collide_radius())
        else {
            return;
        };

        self.update_score(pellet.points());
        self.events.push(GameEvent::SmallPelletEaten);

        if self.pellets.num_eaten() == 30 {
            let (_, position, ghost) = self.maze_spec.inky_start_restriction();
            self.nodes
                .allow_access(position.0, position.1, Direction::Right, ghost.entity());
        }
        if self.pellets.num_eaten() == 70 {
            let (_, position, ghost) = self.maze_spec.clyde_start_restriction();
            self.nodes
                .allow_access(position.0, position.1, Direction::Left, ghost.entity());
        }

        if pellet.kind() == PelletKind::PowerPellet {
            self.ghosts.start_freight();
            self.events.push(GameEvent::PowerPelletEaten);
        }

        if self.pellets.is_empty() {
            self.events.push(GameEvent::LevelCompleted);
            if self.config.flash_background {
                self.flash_background = true;
                self.flash_timer = 0.0;
                self.background = self.background_norm.clone();
            }
            self.hide_entities();
            self.pause.start_timed_pause(3.0, Level6Action::NextLevel);
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
                    .start_timed_pause(1.0, Level6Action::ShowEntities);
                self.ghosts.ghost_mut(ghost_kind).start_spawn(&self.nodes);
                self.nodes.allow_home_access(ghost_kind.entity());
                self.ghosts.update_points();
                self.events.push(GameEvent::GhostEaten);
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
                self.events.push(GameEvent::PacmanDied);
                let action = if self.lives == 0 {
                    self.text_group.show_status(StatusText::GameOver);
                    Level6Action::RestartGame
                } else {
                    Level6Action::ResetLevel
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
                    .get_node_from_tiles(self.maze_spec.fruit_start.0, self.maze_spec.fruit_start.1)
                    .expect("fruit spawn node should exist");
                self.fruit = Some(Fruit::for_level(
                    node,
                    &self.nodes,
                    self.fruit_level_index(),
                ));
                self.fruit_thresholds_spawned[index] = true;
                break;
            }
        }

        let Some(fruit) = &self.fruit else {
            return;
        };

        let fruit_position = fruit.position();
        let fruit_points = fruit.points();
        let fruit_sprite_index = fruit.sprite_index();
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
            if self.config.more_fruit && !self.fruit_captured.contains(&fruit_sprite_index) {
                self.fruit_captured.push(fruit_sprite_index);
            }
            self.events.push(GameEvent::FruitEaten);
            self.fruit = None;
        } else if expired {
            self.fruit = None;
        }
    }

    fn handle_after_pause(&mut self, action: Level6Action) {
        match action {
            Level6Action::ShowEntities => {
                self.text_group.hide_status();
                self.show_entities();
            }
            Level6Action::ResetLevel => self.reset_level(),
            Level6Action::NextLevel => {
                *self = Self::start_level(
                    self.level + 1,
                    self.lives,
                    self.score,
                    self.fruit_captured.clone(),
                    self.config,
                );
            }
            Level6Action::RestartGame => {
                if self.config.return_to_title {
                    self.return_to_title_requested = true;
                } else {
                    *self = Self::start_level(1, 5, 0, Vec::new(), self.config);
                }
            }
        }
    }

    fn reset_level(&mut self) {
        self.pause.set_paused(true);
        self.pacman.reset(&self.nodes);
        self.pacman_sprites.reset();
        self.ghosts.reset(&self.nodes);
        self.nodes.deny_home_access_list(self.ghosts.entity_kinds());
        for (direction, position) in self.maze_spec.deny_ghost_access_positions() {
            self.nodes.deny_access_list(
                position.0,
                position.1,
                direction,
                self.ghosts.entity_kinds(),
            );
        }
        self.fruit = None;
        self.flash_background = false;
        self.flash_timer = 0.0;
        self.background = self.background_norm.clone();
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
                image: self.fruit_sprites.image(fruit.sprite_index()),
                position: sprite_draw_position(fruit.position()),
                anchor: SpriteAnchor::TopLeft,
            });
        }
        if self.pacman.visible() {
            frame.sprites.push(Sprite {
                image: self.pacman_sprites.current(),
                position: sprite_draw_position(self.pacman.position()),
                anchor: SpriteAnchor::TopLeft,
            });
        }
        for ghost in self.ghosts.iter() {
            if ghost.visible() {
                frame.sprites.push(Sprite {
                    image: self
                        .ghost_sprites
                        .image(ghost.kind(), ghost.mode(), ghost.direction()),
                    position: sprite_draw_position(ghost.position()),
                    anchor: SpriteAnchor::TopLeft,
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
        for (index, fruit_index) in self.fruit_captured.iter().enumerate() {
            let image = self.fruit_sprites.image(*fruit_index);
            frame.sprites.push(Sprite {
                image: image.clone(),
                position: Vector2::new(
                    crate::constants::SCREEN_WIDTH as f32 - image.width as f32 * (index + 1) as f32,
                    SCREEN_HEIGHT as f32 - image.height as f32,
                ),
                anchor: SpriteAnchor::TopLeft,
            });
        }
    }
}

impl TitleButtonState {
    fn new(x: f32, y: f32, width: u32, height: u32, label: &str) -> Self {
        Self {
            position: Vector2::new(x, y),
            size: Vector2::new(width as f32, height as f32),
            normal_image: button_image(width, height, BUTTON_COLOR, WHITE),
            hover_image: button_image(width, height, BUTTON_HOVER, WHITE),
            pressed_image: button_image(width, height, BUTTON_CLICK, WHITE),
            label_image: rasterize_text_image(label, YELLOW, 16.0),
            hovered: false,
            pressed: false,
        }
    }

    fn contains(&self, point: Vector2) -> bool {
        point.x >= self.position.x
            && point.x <= self.position.x + self.size.x
            && point.y >= self.position.y
            && point.y <= self.position.y + self.size.y
    }

    fn set_mouse_position(&mut self, mouse_position: Option<Vector2>) {
        self.hovered = mouse_position.is_some_and(|position| self.contains(position));
        if !self.hovered {
            self.pressed = false;
        }
    }

    fn current_image(&self) -> Arc<RenderedImage> {
        if self.pressed {
            self.pressed_image.clone()
        } else if self.hovered {
            self.hover_image.clone()
        } else {
            self.normal_image.clone()
        }
    }

    fn label_position(&self) -> Vector2 {
        Vector2::new(
            self.position.x + (self.size.x - self.label_image.width as f32) * 0.5,
            self.position.y + (self.size.y - self.label_image.height as f32) * 0.5,
        )
    }

    fn append_renderables(&self, frame: &mut FrameData) {
        frame.sprites.push(Sprite {
            image: self.current_image(),
            position: self.position,
            anchor: SpriteAnchor::TopLeft,
        });
        frame.sprites.push(Sprite {
            image: self.label_image.clone(),
            position: self.label_position(),
            anchor: SpriteAnchor::TopLeft,
        });
    }
}

impl TitleScreenState {
    fn new(show_button: bool) -> Self {
        let button = show_button.then(|| {
            TitleButtonState::new(
                SCREEN_WIDTH as f32 / 2.0 - 60.0,
                SCREEN_HEIGHT as f32 / 2.0 - 30.0,
                120,
                60,
                "START",
            )
        });
        let prompt_image = rasterize_text_image("PRESS ENTER", WHITE, 16.0);

        Self {
            title_image: rasterize_text_image("PACMAN", YELLOW, 64.0),
            prompt_image,
            button,
        }
    }

    fn update(&mut self, mouse_position: Option<Vector2>) {
        if let Some(button) = &mut self.button {
            button.set_mouse_position(mouse_position);
        }
    }

    fn start_requested(
        &mut self,
        start_requested: bool,
        mouse_click_position: Option<Vector2>,
    ) -> bool {
        if start_requested {
            return true;
        }

        let Some(button) = &mut self.button else {
            return false;
        };

        if mouse_click_position.is_some_and(|position| button.contains(position)) {
            button.pressed = true;
            return true;
        }

        false
    }

    fn append_renderables(&self, frame: &mut FrameData) {
        frame.sprites.push(Sprite {
            image: self.title_image.clone(),
            position: Vector2::new(32.0, 10.0),
            anchor: SpriteAnchor::TopLeft,
        });

        if let Some(button) = &self.button {
            button.append_renderables(frame);
        } else {
            frame.sprites.push(Sprite {
                image: self.prompt_image.clone(),
                position: Vector2::new(
                    (SCREEN_WIDTH as f32 - self.prompt_image.width as f32) * 0.5,
                    SCREEN_HEIGHT as f32 * 0.5,
                ),
                anchor: SpriteAnchor::TopLeft,
            });
        }
    }
}

impl Level7State {
    fn new(config: Level7Config) -> Self {
        Self {
            config,
            title_screen: TitleScreenState::new(config.buttons),
            gameplay: None,
            events: vec![GameEvent::TitleScreenEntered],
        }
    }

    fn update(
        &mut self,
        dt: f32,
        requested_direction: Direction,
        pause_requested: bool,
        start_requested: bool,
        mouse_position: Option<Vector2>,
        mouse_click_position: Option<Vector2>,
    ) {
        if let Some(gameplay) = &mut self.gameplay {
            gameplay.update(dt, requested_direction, pause_requested);
            self.events.extend(gameplay.drain_events());

            if gameplay.return_to_title_requested {
                self.gameplay = None;
                self.title_screen = TitleScreenState::new(self.config.buttons);
                self.events.push(GameEvent::TitleScreenEntered);
            }
            return;
        }

        self.title_screen.update(mouse_position);
        let should_start = self
            .title_screen
            .start_requested(start_requested, mouse_click_position);

        if mouse_click_position.is_some_and(|position| {
            self.config.buttons
                && self
                    .title_screen
                    .button
                    .as_ref()
                    .is_some_and(|button| button.contains(position))
        }) {
            self.events.push(GameEvent::ButtonClicked);
        }

        if should_start {
            self.gameplay = Some(Level6State::new(Level6Config::level7()));
            self.events.push(GameEvent::GameStarted);
        }
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    fn append_renderables(&self, frame: &mut FrameData) {
        if let Some(gameplay) = &self.gameplay {
            gameplay.append_renderables(frame);
        } else {
            self.title_screen.append_renderables(frame);
        }
    }
}

fn button_image(width: u32, height: u32, fill: [u8; 4], border: [u8; 4]) -> Arc<RenderedImage> {
    let mut pixels = vec![0; width as usize * height as usize * 4];
    let border_thickness = 3;

    for y in 0..height {
        for x in 0..width {
            let color = if x < border_thickness
                || y < border_thickness
                || x >= width - border_thickness
                || y >= height - border_thickness
            {
                border
            } else {
                fill
            };
            let index = ((y * width + x) as usize) * 4;
            pixels[index..index + 4].copy_from_slice(&color);
        }
    }

    Arc::new(RenderedImage {
        width,
        height,
        pixels,
    })
}

fn sprite_draw_position(position: Vector2) -> Vector2 {
    position - Vector2::new(TILE_WIDTH as f32 / 2.0, TILE_HEIGHT as f32 / 2.0)
}

#[cfg(test)]
mod tests {
    use super::{
        Game, GameEvent, Level4Action, Level4State, Level5State, Level6Config, Level6State, Stage,
        sprite_draw_position,
    };
    use crate::{
        constants::{SCREEN_HEIGHT, SCREEN_WIDTH, TILE_HEIGHT, TILE_WIDTH},
        nodes::NodeGroup,
        pacman::Direction,
        pellets::PelletGroup,
        render::{FrameData, SpriteAnchor},
        vector::Vector2,
    };

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
    fn level5_sprite_positions_match_tutorial_entity_offset() {
        let state = Level5State::new();
        let mut frame = FrameData::default();
        state.append_renderables(&mut frame);

        let expected_offset = Vector2::new(TILE_WIDTH as f32 / 2.0, TILE_HEIGHT as f32 / 2.0);
        assert_eq!(
            sprite_draw_position(state.pacman.position()),
            state.pacman.position() - expected_offset
        );

        let pacman_sprite = &frame.sprites[0];
        assert_eq!(pacman_sprite.anchor, SpriteAnchor::TopLeft);
        assert_eq!(
            pacman_sprite.position,
            state.pacman.position() - expected_offset
        );

        for (sprite, ghost) in frame.sprites[1..5].iter().zip(state.ghosts.iter()) {
            assert_eq!(sprite.anchor, SpriteAnchor::TopLeft);
            assert_eq!(sprite.position, ghost.position() - expected_offset);
        }
    }

    #[test]
    fn level5_updates_the_score_display_when_points_are_added() {
        let mut state = Level5State::new();
        state.update_score(10);

        assert_eq!(state.score, 10);
    }

    #[test]
    fn level6_stage_renders_background_and_sprites() {
        let game = Game::new(Stage::MoreMazes);
        let frame = game.frame();

        assert!(frame.background.is_some());
        assert!(frame.lines.is_empty());
        assert!(frame.sprites.len() >= 10);
    }

    #[test]
    fn level6_uses_the_second_maze_on_level_two() {
        let state = Level6State::start_level(2, 5, 0, Vec::new(), Level6Config::more_mazes());

        assert_eq!(state.maze_spec.name, "maze2");
    }

    #[test]
    fn level6_updates_the_death_animation_while_paused() {
        let mut state = Level6State::new(Level6Config::pacman_death());
        let before = state.pacman_sprites.current();
        state.pacman.die();

        state.update(0.2, Direction::Stop, false);

        assert_ne!(before.pixels, state.pacman_sprites.current().pixels);
    }

    #[test]
    fn level7_title_screen_emits_an_entered_event() {
        let mut game = Game::new(Stage::AddTitleScreen);

        assert_eq!(game.drain_events(), vec![GameEvent::TitleScreenEntered]);
    }

    #[test]
    fn level7_enter_starts_the_gameplay_screen() {
        let mut game = Game::new(Stage::AddTitleScreen);
        let _ = game.drain_events();

        game.update_with_input(0.0, Direction::Stop, false, true, None, None);

        let events = game.drain_events();
        assert_eq!(events, vec![GameEvent::GameStarted]);
        assert!(game.frame().background.is_some());
    }

    #[test]
    fn level7_button_click_starts_the_gameplay_screen() {
        let mut game = Game::new(Stage::AddButtons);
        let _ = game.drain_events();
        let button_center = Vector2::new(SCREEN_WIDTH as f32 / 2.0, SCREEN_HEIGHT as f32 / 2.0);

        game.update_with_input(
            0.0,
            Direction::Stop,
            false,
            false,
            Some(button_center),
            Some(button_center),
        );

        let events = game.drain_events();
        assert_eq!(
            events,
            vec![GameEvent::ButtonClicked, GameEvent::GameStarted]
        );
        assert!(game.frame().background.is_some());
    }

    #[test]
    fn level7_button_click_uses_the_click_position() {
        let mut game = Game::new(Stage::AddButtons);
        let _ = game.drain_events();
        let button_center = Vector2::new(SCREEN_WIDTH as f32 / 2.0, SCREEN_HEIGHT as f32 / 2.0);

        game.update_with_input(
            0.0,
            Direction::Stop,
            false,
            false,
            None,
            Some(button_center),
        );

        let events = game.drain_events();
        assert_eq!(
            events,
            vec![GameEvent::ButtonClicked, GameEvent::GameStarted]
        );
    }
}
