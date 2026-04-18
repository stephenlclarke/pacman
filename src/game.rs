//! Contains the main game state machine, gameplay update logic, title screens, and headless simulation support.

use std::sync::Arc;

use crate::{
    actors::{EntityKind, GhostKind},
    arcade::{
        ORIGINAL_FRAME_TIME, dot_pause_seconds, fruit_release_dots, ghost_personal_dot_limit,
        global_release_dot, release_timer_limit,
    },
    autopilot::{AutoPilot, AutoPilotContext},
    constants::{
        BUTTON_CLICK, BUTTON_COLOR, BUTTON_HOVER, ORANGE, PINK, RED, SCREEN_HEIGHT, SCREEN_WIDTH,
        TEAL, TILE_HEIGHT, TILE_WIDTH, WHITE, YELLOW,
    },
    fruit::Fruit,
    ghosts::{GhostGroup, GhostGroupUpdateContext},
    mazedata::MazeSpec,
    modes::GhostMode,
    nodes::NodeGroup,
    pacman::{Direction, NodePacman},
    pause::PauseController,
    pellets::{PelletGroup, PelletKind},
    render::{Circle, FrameData, RenderedImage, Sprite, SpriteAnchor},
    sprites::{FruitSprites, GhostSprites, LifeSprites, MazeSprites, PacmanSprites},
    text::{StatusText, TextGroup, rasterize_text_image},
    vector::Vector2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GameplayAction {
    ShowEntities,
    ResetLevel,
    NextLevel,
    RestartGame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SecretModeFlags {
    easter_egg_active: bool,
    easter_egg_force_freight: bool,
    autopilot_active: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameEvent {
    TitleScreenEntered,
    ButtonClicked,
    GameStarted,
    SmallPelletEaten,
    PowerPelletEaten,
    FreightModeStarted,
    FreightModeEnded,
    GhostEaten,
    FruitEaten,
    PacmanDied,
    LevelCompleted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeadlessAutopilotStopReason {
    PacmanDied,
    StepLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HeadlessGhostSnapshot {
    pub kind: GhostKind,
    pub mode: GhostMode,
    pub position: (f32, f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct HeadlessDeathSnapshot {
    pub level: u32,
    pub score: u32,
    pub lives: u32,
    pub pellets_remaining: usize,
    pub pacman_position: (f32, f32),
    pub pacman_direction: Direction,
    pub ghosts: Vec<HeadlessGhostSnapshot>,
    pub remaining_pellets: Vec<(f32, f32)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HeadlessAutopilotReport {
    pub seed: u64,
    pub steps: usize,
    pub levels_cleared: u32,
    pub level_reached: u32,
    pub pellets_eaten: u32,
    pub ghosts_eaten: u32,
    pub fruit_eaten: u32,
    pub score: u32,
    pub stop_reason: HeadlessAutopilotStopReason,
    pub death_snapshot: Option<HeadlessDeathSnapshot>,
}

#[derive(Clone, Debug, Default)]
pub struct UpdateInput {
    pub requested_direction: Direction,
    pub pause_requested: bool,
    pub start_requested: bool,
    pub mouse_position: Option<Vector2>,
    pub mouse_click_position: Option<Vector2>,
    pub typed_chars: Vec<char>,
}

#[derive(Debug)]
pub struct Game {
    state: AppState,
    quit_requested: bool,
}

#[derive(Clone, Debug)]
struct GameplayState {
    nodes: NodeGroup,
    pacman: NodePacman,
    pellets: PelletGroup,
    ghosts: GhostGroup,
    fruit: Option<Fruit>,
    pause: PauseController<GameplayAction>,
    level: u32,
    lives: u32,
    score: u32,
    pacman_pause_remaining: f32,
    fruit_thresholds_spawned: [bool; 2],
    personal_dot_counters: [usize; 4],
    global_dot_counter: Option<usize>,
    dot_release_timer: f32,
    ghost_release_locks: [bool; 4],
    elroy_suspended: bool,
    background: Arc<RenderedImage>,
    background_norm: Arc<RenderedImage>,
    background_flash: Arc<RenderedImage>,
    flash_background: bool,
    flash_timer: f32,
    text_group: TextGroup,
    life_sprites: LifeSprites,
    pacman_sprites: PacmanSprites,
    ghost_sprites: GhostSprites,
    fruit_sprites: FruitSprites,
    fruit_captured: Vec<usize>,
    maze_spec: MazeSpec,
    events: Vec<GameEvent>,
    easter_egg_active: bool,
    easter_egg_sequence_index: usize,
    easter_egg_force_freight: bool,
    easter_egg_autopilot: AutoPilot,
    easter_egg_blink: BlinkFeedback,
    return_to_title_requested: bool,
    #[cfg(test)]
    last_pacman_sprite_direction: Direction,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BlinkFeedback {
    toggles_remaining: u8,
    timer: f32,
    visible: bool,
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
    scene: TitleAttractScene,
    scene_timer: f32,
    prompt_blink_timer: f32,
    prompt_visible: bool,
    title_image: Arc<RenderedImage>,
    company_image: Arc<RenderedImage>,
    bonus_image: Arc<RenderedImage>,
    push_start_image: Arc<RenderedImage>,
    one_player_image: Arc<RenderedImage>,
    two_player_image: Arc<RenderedImage>,
    scoring_image: Arc<RenderedImage>,
    pellet_score_image: Arc<RenderedImage>,
    power_score_image: Arc<RenderedImage>,
    ghost_score_images: [Arc<RenderedImage>; 4],
    fruit_score_images: [Arc<RenderedImage>; 8],
    character_image: Arc<RenderedImage>,
    nickname_image: Arc<RenderedImage>,
    nickname_rows: [AttractNicknameRow; 4],
    pacman_sprites: PacmanSprites,
    ghost_sprites: GhostSprites,
    fruit_sprites: FruitSprites,
    button: TitleButtonState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TitleAttractScene {
    Title,
    Scoring,
    Nicknames,
}

#[derive(Clone, Debug)]
struct AttractNicknameRow {
    kind: GhostKind,
    nickname_image: Arc<RenderedImage>,
    name_image: Arc<RenderedImage>,
}

#[derive(Clone, Debug)]
struct AppState {
    title_screen: TitleScreenState,
    gameplay: Option<GameplayState>,
    events: Vec<GameEvent>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            state: AppState::new(),
            quit_requested: false,
        }
    }

    /// Updates with input.
    pub fn update_with_input(&mut self, dt: f32, input: UpdateInput) {
        let q_pressed = input
            .typed_chars
            .iter()
            .any(|character| character.eq_ignore_ascii_case(&'q'));
        self.state.update(dt, &input);

        if q_pressed {
            self.quit_requested = true;
        }
    }

    pub fn quit_requested(&self) -> bool {
        self.quit_requested
    }

    pub fn drain_events(&mut self) -> Vec<GameEvent> {
        self.state.drain_events()
    }

    pub fn frame(&self) -> FrameData {
        let mut frame = FrameData::default();
        self.state.append_renderables(&mut frame);
        frame
    }
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

/// Runs headless autopilot.
pub fn run_headless_autopilot(seed: u64, max_steps: usize) -> HeadlessAutopilotReport {
    fastrand::seed(seed);

    let mut state = GameplayState::new();
    state.pause.set_paused(false);
    state.easter_egg_active = true;
    state.easter_egg_autopilot.set_active(true);

    let mut report = HeadlessAutopilotReport {
        seed,
        steps: 0,
        levels_cleared: 0,
        level_reached: state.level,
        pellets_eaten: 0,
        ghosts_eaten: 0,
        fruit_eaten: 0,
        score: 0,
        stop_reason: HeadlessAutopilotStopReason::StepLimit,
        death_snapshot: None,
    };

    for _ in 0..max_steps {
        state.update_headless(ORIGINAL_FRAME_TIME);
        report.steps += 1;
        report.level_reached = report.level_reached.max(state.level);

        for event in state.drain_events() {
            match event {
                GameEvent::SmallPelletEaten => report.pellets_eaten += 1,
                GameEvent::GhostEaten => report.ghosts_eaten += 1,
                GameEvent::FruitEaten => report.fruit_eaten += 1,
                GameEvent::LevelCompleted => report.levels_cleared += 1,
                GameEvent::PacmanDied => {
                    report.score = state.score;
                    report.stop_reason = HeadlessAutopilotStopReason::PacmanDied;
                    report.death_snapshot = Some(state.headless_death_snapshot());
                    return report;
                }
                GameEvent::TitleScreenEntered
                | GameEvent::ButtonClicked
                | GameEvent::GameStarted
                | GameEvent::PowerPelletEaten
                | GameEvent::FreightModeStarted
                | GameEvent::FreightModeEnded => {}
            }
        }
    }

    report.score = state.score;
    report
}

fn build_gameplay_level(
    maze_spec: MazeSpec,
    level: u32,
) -> (NodeGroup, NodePacman, PelletGroup, GhostGroup) {
    let mut nodes = NodeGroup::from_pacman_layout(maze_spec.layout);
    for &(left, right) in &maze_spec.portal_pairs {
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
    let mut pacman = NodePacman::new(pacman_start, &nodes);
    pacman.configure_level(level);
    pacman.configure_start(
        pacman_start,
        Direction::Left,
        Some(Direction::Left),
        Some(Vector2::new(
            maze_spec.pacman_start_pixels.0,
            maze_spec.pacman_start_pixels.1,
        )),
        &nodes,
    );

    let mut ghosts = GhostGroup::new(nodes.start_node(), &nodes, level);
    ghosts.ghost_mut(GhostKind::Blinky).set_start_state(
        node_at(&nodes, maze_spec.blinky_start(), "blinky start node"),
        Some(Vector2::new(
            maze_spec.blinky_start_pixels.0,
            maze_spec.blinky_start_pixels.1,
        )),
        &nodes,
    );
    ghosts.ghost_mut(GhostKind::Pinky).set_start_state(
        node_at(&nodes, maze_spec.pinky_start(), "pinky start node"),
        Some(Vector2::new(
            maze_spec.pinky_start_pixels.0,
            maze_spec.pinky_start_pixels.1,
        )),
        &nodes,
    );
    ghosts.ghost_mut(GhostKind::Inky).set_start_state(
        node_at(&nodes, maze_spec.inky_start(), "inky start node"),
        Some(Vector2::new(
            maze_spec.inky_start_pixels.0,
            maze_spec.inky_start_pixels.1,
        )),
        &nodes,
    );
    ghosts.ghost_mut(GhostKind::Clyde).set_start_state(
        node_at(&nodes, maze_spec.clyde_start(), "clyde start node"),
        Some(Vector2::new(
            maze_spec.clyde_start_pixels.0,
            maze_spec.clyde_start_pixels.1,
        )),
        &nodes,
    );
    ghosts.set_spawn_node(node_at(&nodes, maze_spec.spawn_node(), "ghost spawn node"));

    nodes.deny_home_access(EntityKind::Pacman);
    nodes.deny_home_access_list(ghosts.entity_kinds());
    for (direction, position) in maze_spec.deny_ghost_access_positions() {
        nodes.deny_access_list(position.0, position.1, direction, ghosts.entity_kinds());
    }

    for &(col, row) in &maze_spec.ghost_deny_up {
        nodes.deny_access_list(col, row, Direction::Up, ghosts.entity_kinds());
    }

    let pellets = PelletGroup::from_layout(maze_spec.layout);

    (nodes, pacman, pellets, ghosts)
}

impl GameplayState {
    const FLASH_TIME: f32 = 0.2;
    const EASTER_EGG_CODE: [char; 5] = ['x', 'y', 'z', 'z', 'y'];
    const READY_TIME: f32 = 3.0;

    fn new() -> Self {
        Self::start_level(1, 5, 0, Vec::new())
    }

    /// Starts level.
    fn start_level(level: u32, lives: u32, score: u32, fruit_captured: Vec<usize>) -> Self {
        let maze_spec = MazeSpec::arcade();
        let (nodes, pacman, pellets, ghosts) = build_gameplay_level(maze_spec, level);
        let maze_sprites = MazeSprites::from_layout(maze_spec.layout);
        let background_norm = maze_sprites.construct_background(level);
        let background_flash = maze_sprites.construct_flash_background();

        let mut text_group = TextGroup::new();
        text_group.update_score(score);
        text_group.update_level(level);
        text_group.show_status(StatusText::Ready);

        let mut state = Self {
            nodes,
            pacman,
            pellets,
            ghosts,
            fruit: None,
            pause: PauseController::new(true),
            level,
            lives,
            score,
            pacman_pause_remaining: 0.0,
            fruit_thresholds_spawned: [false; 2],
            personal_dot_counters: [0; 4],
            global_dot_counter: None,
            dot_release_timer: 0.0,
            ghost_release_locks: [false; 4],
            elroy_suspended: false,
            background: background_norm.clone(),
            background_norm,
            background_flash,
            flash_background: false,
            flash_timer: 0.0,
            text_group,
            life_sprites: LifeSprites::new(lives),
            pacman_sprites: PacmanSprites::new(),
            ghost_sprites: GhostSprites::new(),
            fruit_sprites: FruitSprites::new(),
            fruit_captured,
            maze_spec,
            events: Vec::new(),
            easter_egg_active: false,
            easter_egg_sequence_index: 0,
            easter_egg_force_freight: false,
            easter_egg_autopilot: AutoPilot::default(),
            easter_egg_blink: BlinkFeedback::default(),
            return_to_title_requested: false,
            #[cfg(test)]
            last_pacman_sprite_direction: Direction::Stop,
        };
        state
            .pause
            .start_timed_pause(Self::READY_TIME, GameplayAction::ShowEntities);
        state.apply_level_start_release_rules();
        state
    }

    fn update(
        &mut self,
        dt: f32,
        requested_direction: Direction,
        pause_requested: bool,
        typed_chars: &[char],
    ) {
        let freight_was_active = self.ghosts.has_freight_mode();
        self.update_ui(dt, typed_chars);

        if !self.pause.paused() {
            self.update_live_entities(dt, requested_direction);
        }

        self.update_pacman_animation(dt);
        self.update_background_flash(dt);

        if let Some(action) = self.pause.update(dt) {
            self.handle_after_pause(action);
        }

        self.handle_pause_request(pause_requested);

        self.sync_freight_events(freight_was_active);
    }

    /// Updates headless.
    fn update_headless(&mut self, dt: f32) {
        let mut requested_direction = Direction::Stop;
        let freight_was_active = self.ghosts.has_freight_mode();

        if !self.pause.paused() {
            self.tick_ghost_release_timer(dt);
            self.sustain_secret_freight_mode();

            if self.pacman.alive() && self.easter_egg_autopilot.active() {
                requested_direction = self.easter_egg_autopilot.choose_direction(
                    &self.nodes,
                    &self.pacman,
                    &self.pellets,
                    &self.ghosts,
                    self.fruit.as_ref(),
                    AutoPilotContext {
                        level: self.level,
                        elroy_enabled: !self.elroy_suspended,
                    },
                );
            }
            if self.pacman.alive() {
                self.update_pacman_motion(dt, requested_direction);
            }

            self.ghosts.update(
                dt,
                &self.nodes,
                GhostGroupUpdateContext {
                    pacman_position: self.pacman.position(),
                    pacman_direction: self.pacman.direction(),
                    level: self.level,
                    dots_remaining: self.pellets.len(),
                    elroy_enabled: !self.elroy_suspended,
                },
            );

            if let Some(fruit) = &mut self.fruit {
                fruit.update(dt);
            }

            self.check_pellet_events_headless();
            self.check_ghost_events_headless();
            self.check_fruit_events_headless();
        }
        if let Some(action) = self.pause.update(dt) {
            self.handle_after_pause_headless(action);
        }

        self.sync_freight_events(freight_was_active);
    }

    /// Updates ui.
    fn update_ui(&mut self, dt: f32, typed_chars: &[char]) {
        self.handle_easter_egg_input(typed_chars);
        self.easter_egg_blink.update(dt);
        self.text_group.update(dt);
        self.pellets.update(dt);
    }

    /// Updates live entities.
    fn update_live_entities(&mut self, dt: f32, requested_direction: Direction) {
        self.tick_ghost_release_timer(dt);
        self.sustain_secret_freight_mode();

        let requested_direction = self.autopilot_direction(requested_direction);
        if self.pacman.alive() {
            self.update_pacman_motion(dt, requested_direction);
        }

        self.update_ghosts(dt);
        self.update_fruit(dt);
        self.check_pellet_events();
        self.check_ghost_events();
        self.check_fruit_events();
    }

    fn autopilot_direction(&mut self, requested_direction: Direction) -> Direction {
        if !self.pacman.alive() || !self.easter_egg_autopilot.active() {
            return requested_direction;
        }

        self.easter_egg_autopilot.choose_direction(
            &self.nodes,
            &self.pacman,
            &self.pellets,
            &self.ghosts,
            self.fruit.as_ref(),
            AutoPilotContext {
                level: self.level,
                elroy_enabled: !self.elroy_suspended,
            },
        )
    }

    /// Updates pacman animation.
    fn update_pacman_animation(&mut self, dt: f32) {
        if !self.pacman.alive() {
            self.pacman_sprites
                .update_for_state(dt, self.pacman.direction(), false);
            return;
        }

        #[cfg(test)]
        {
            self.last_pacman_sprite_direction = self.pacman.direction();
        }
        self.pacman_sprites
            .update_for_state(dt, self.pacman.direction(), true);
    }

    /// Updates ghosts.
    fn update_ghosts(&mut self, dt: f32) {
        self.ghosts.update(
            dt,
            &self.nodes,
            GhostGroupUpdateContext {
                pacman_position: self.pacman.position(),
                pacman_direction: self.pacman.direction(),
                level: self.level,
                dots_remaining: self.pellets.len(),
                elroy_enabled: !self.elroy_suspended,
            },
        );
    }

    /// Updates fruit.
    fn update_fruit(&mut self, dt: f32) {
        if let Some(fruit) = &mut self.fruit {
            fruit.update(dt);
        }
    }

    /// Updates background flash.
    fn update_background_flash(&mut self, dt: f32) {
        if !self.flash_background {
            return;
        }

        self.flash_timer += dt;
        if self.flash_timer < Self::FLASH_TIME {
            return;
        }

        self.flash_timer = 0.0;
        if Arc::ptr_eq(&self.background, &self.background_norm) {
            self.background = self.background_flash.clone();
        } else {
            self.background = self.background_norm.clone();
        }
    }

    fn handle_pause_request(&mut self, pause_requested: bool) {
        if !pause_requested || !self.pacman.alive() || self.pause.is_timed() {
            return;
        }

        if self.pause.toggle() {
            self.text_group.show_status(StatusText::Paused);
            self.hide_entities();
        } else {
            self.text_group.hide_status();
            self.show_entities();
        }
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    fn handle_easter_egg_input(&mut self, typed_chars: &[char]) {
        for &character in typed_chars {
            let character = character.to_ascii_lowercase();
            let is_secret_code_input = self.update_easter_egg_sequence(character);

            if !self.easter_egg_active {
                continue;
            }

            match character {
                'a' => {
                    self.easter_egg_autopilot.toggle();
                }
                'f' => self.toggle_secret_freight_mode(),
                't' => {
                    self.teleport_pacman_to_safest_node();
                    self.easter_egg_autopilot.invalidate_route();
                }
                'r' if !is_secret_code_input => {
                    self.reset_ghosts_to_start_positions();
                    self.easter_egg_autopilot.invalidate_route();
                }
                _ => {}
            }
        }
    }

    /// Updates easter egg sequence.
    fn update_easter_egg_sequence(&mut self, character: char) -> bool {
        if character == Self::EASTER_EGG_CODE[self.easter_egg_sequence_index] {
            self.easter_egg_sequence_index += 1;
            if self.easter_egg_sequence_index == Self::EASTER_EGG_CODE.len() {
                self.easter_egg_sequence_index = 0;
                self.toggle_easter_egg_mode();
            }
            return true;
        }

        self.easter_egg_sequence_index = usize::from(character == Self::EASTER_EGG_CODE[0]);
        character == Self::EASTER_EGG_CODE[0]
    }

    /// Toggles easter egg mode.
    fn toggle_easter_egg_mode(&mut self) {
        self.easter_egg_active = !self.easter_egg_active;
        self.easter_egg_sequence_index = 0;
        self.easter_egg_blink.start();
        if !self.easter_egg_active {
            self.easter_egg_autopilot.disable();
            self.disable_secret_freight_mode();
        }
    }

    /// Toggles secret freight mode.
    fn toggle_secret_freight_mode(&mut self) {
        if self.easter_egg_force_freight {
            self.disable_secret_freight_mode();
            return;
        }

        self.easter_egg_force_freight = true;
        self.ghosts.start_freight();
    }

    /// Disables secret freight mode.
    fn disable_secret_freight_mode(&mut self) {
        if !self.easter_egg_force_freight {
            return;
        }

        self.easter_egg_force_freight = false;
        self.ghosts.end_freight();
    }

    /// Sustains secret freight mode.
    fn sustain_secret_freight_mode(&mut self) {
        if !self.easter_egg_force_freight {
            return;
        }

        self.ghosts.sustain_freight();
    }

    fn teleport_pacman_to_safest_node(&mut self) {
        let Some(target) = self
            .nodes
            .node_ids()
            .filter(|&node_id| {
                Direction::cardinals().into_iter().any(|direction| {
                    self.nodes
                        .can_travel(node_id, direction, EntityKind::Pacman)
                })
            })
            .max_by(|&lhs, &rhs| {
                let lhs_distance = self.minimum_ghost_distance_squared(lhs);
                let rhs_distance = self.minimum_ghost_distance_squared(rhs);
                lhs_distance.total_cmp(&rhs_distance)
            })
        else {
            return;
        };

        self.pacman.teleport_to_node(target, &self.nodes);
    }

    /// Computes ghost distance squared.
    fn minimum_ghost_distance_squared(&self, node_id: usize) -> f32 {
        let position = self.nodes.position(node_id);
        self.ghosts
            .iter()
            .map(|ghost| (position - ghost.position()).magnitude_squared())
            .fold(f32::INFINITY, f32::min)
    }

    /// Resets ghosts to start positions.
    fn reset_ghosts_to_start_positions(&mut self) {
        let freight_active = self.ghosts.has_freight_mode();
        self.ghosts.reset(&self.nodes, self.level);
        self.apply_level_start_release_rules();
        if freight_active || self.easter_egg_force_freight {
            self.ghosts.start_freight();
        }
    }

    /// Restores ghost access rules.
    fn restore_ghost_access_rules(&mut self) {
        self.nodes.deny_home_access_list(self.ghosts.entity_kinds());
        for (direction, position) in self.maze_spec.deny_ghost_access_positions() {
            self.nodes.deny_access_list(
                position.0,
                position.1,
                direction,
                self.ghosts.entity_kinds(),
            );
        }
        for &(col, row) in &self.maze_spec.ghost_deny_up {
            self.nodes
                .deny_access_list(col, row, Direction::Up, self.ghosts.entity_kinds());
        }
        for kind in GhostKind::ALL {
            if self.ghost_release_locks[kind.index()] {
                self.apply_release_lock(kind);
            } else {
                self.remove_release_lock(kind);
            }
        }
    }

    /// Sets red zone restrictions.
    fn set_red_zone_restrictions(&mut self, enabled: bool) {
        for &(col, row) in &self.maze_spec.ghost_deny_up {
            if enabled {
                self.nodes
                    .deny_access_list(col, row, Direction::Up, self.ghosts.entity_kinds());
            } else {
                self.nodes
                    .allow_access_list(col, row, Direction::Up, self.ghosts.entity_kinds());
            }
        }
    }

    /// Updates pacman motion.
    fn update_pacman_motion(&mut self, dt: f32, requested_direction: Direction) {
        let frightened = self.ghosts.has_freight_mode();
        self.pacman.set_frightened(frightened);

        let moving_dt = (dt - self.pacman_pause_remaining).max(0.0);
        self.pacman_pause_remaining = (self.pacman_pause_remaining - dt).max(0.0);
        self.pacman
            .update(moving_dt, requested_direction, &self.nodes);
    }

    fn tick_ghost_release_timer(&mut self, dt: f32) {
        self.dot_release_timer += dt;
        if self.dot_release_timer < release_timer_limit(self.level) {
            return;
        }

        self.dot_release_timer = 0.0;
        if let Some(kind) = self.next_locked_ghost() {
            self.release_ghost(kind);
        }
    }

    fn record_dot_for_ghost_house(&mut self) {
        self.dot_release_timer = 0.0;

        if let Some(counter) = &mut self.global_dot_counter {
            *counter += 1;
            for kind in [GhostKind::Pinky, GhostKind::Inky, GhostKind::Clyde] {
                if self.ghost_release_locks[kind.index()]
                    && global_release_dot(kind).is_some_and(|threshold| *counter == threshold)
                {
                    self.release_ghost(kind);
                    if kind == GhostKind::Clyde {
                        self.global_dot_counter = None;
                    }
                    return;
                }
            }
            return;
        }

        let Some(kind) = self.next_locked_ghost() else {
            return;
        };
        self.personal_dot_counters[kind.index()] += 1;
        if self.personal_dot_counters[kind.index()] >= ghost_personal_dot_limit(kind, self.level) {
            self.release_ghost(kind);
        }
    }

    fn apply_level_start_release_rules(&mut self) {
        self.pacman_pause_remaining = 0.0;
        self.dot_release_timer = 0.0;
        self.global_dot_counter = None;
        self.ghost_release_locks = [false; 4];
        self.personal_dot_counters = [0; 4];
        self.elroy_suspended = false;

        if ghost_personal_dot_limit(GhostKind::Inky, self.level) > 0 {
            self.ghost_release_locks[GhostKind::Inky.index()] = true;
        }
        if ghost_personal_dot_limit(GhostKind::Clyde, self.level) > 0 {
            self.ghost_release_locks[GhostKind::Clyde.index()] = true;
        }
        self.restore_ghost_access_rules();
    }

    fn apply_post_death_release_rules(&mut self) {
        self.pacman_pause_remaining = 0.0;
        self.dot_release_timer = 0.0;
        self.global_dot_counter = Some(0);
        self.ghost_release_locks = [false; 4];
        self.ghost_release_locks[GhostKind::Pinky.index()] = true;
        self.ghost_release_locks[GhostKind::Inky.index()] = true;
        self.ghost_release_locks[GhostKind::Clyde.index()] = true;
        self.elroy_suspended = true;
        self.restore_ghost_access_rules();
    }

    fn next_locked_ghost(&self) -> Option<GhostKind> {
        [GhostKind::Pinky, GhostKind::Inky, GhostKind::Clyde]
            .into_iter()
            .find(|kind| self.ghost_release_locks[kind.index()])
    }

    fn release_ghost(&mut self, kind: GhostKind) {
        self.ghost_release_locks[kind.index()] = false;
        self.remove_release_lock(kind);
        if kind == GhostKind::Clyde {
            self.elroy_suspended = false;
        }
    }

    fn apply_release_lock(&mut self, kind: GhostKind) {
        let (direction, position, ghost) = match kind {
            GhostKind::Pinky => self.maze_spec.pinky_start_restriction(),
            GhostKind::Inky => self.maze_spec.inky_start_restriction(),
            GhostKind::Clyde => self.maze_spec.clyde_start_restriction(),
            GhostKind::Blinky => return,
        };
        self.nodes
            .deny_access(position.0, position.1, direction, ghost.entity());
    }

    fn remove_release_lock(&mut self, kind: GhostKind) {
        let (direction, position, ghost) = match kind {
            GhostKind::Pinky => self.maze_spec.pinky_start_restriction(),
            GhostKind::Inky => self.maze_spec.inky_start_restriction(),
            GhostKind::Clyde => self.maze_spec.clyde_start_restriction(),
            GhostKind::Blinky => return,
        };
        self.nodes
            .allow_access(position.0, position.1, direction, ghost.entity());
    }

    fn secret_mode_flags(&self) -> SecretModeFlags {
        SecretModeFlags {
            easter_egg_active: self.easter_egg_active,
            easter_egg_force_freight: self.easter_egg_force_freight,
            autopilot_active: self.easter_egg_autopilot.active(),
        }
    }

    fn apply_secret_mode_flags(&mut self, flags: SecretModeFlags) {
        self.easter_egg_active = flags.easter_egg_active;
        self.easter_egg_force_freight = flags.easter_egg_force_freight;
        self.easter_egg_sequence_index = 0;
        self.easter_egg_autopilot.set_active(flags.autopilot_active);
        if self.easter_egg_force_freight {
            self.ghosts.start_freight();
        }
    }

    /// Synchronizes freight events.
    fn sync_freight_events(&mut self, freight_was_active: bool) {
        let freight_is_active = self.ghosts.has_freight_mode();
        self.set_red_zone_restrictions(!freight_is_active);
        match (freight_was_active, freight_is_active) {
            (false, true) => {
                self.events.push(GameEvent::FreightModeStarted);
            }
            (true, false) => {
                self.events.push(GameEvent::FreightModeEnded);
            }
            _ => {}
        }
    }

    /// Updates score.
    fn update_score(&mut self, points: u32) {
        self.score += points;
        self.text_group.update_score(self.score);
    }

    /// Updates score headless.
    fn update_score_headless(&mut self, points: u32) {
        self.score += points;
    }

    /// Checks pellet events.
    fn check_pellet_events(&mut self) {
        let Some(pellet) = self
            .pellets
            .try_eat(self.pacman.position(), self.pacman.collide_radius())
        else {
            return;
        };

        self.update_score(pellet.points());
        self.events.push(GameEvent::SmallPelletEaten);
        self.pacman_pause_remaining += dot_pause_seconds(pellet.kind() == PelletKind::PowerPellet);
        self.record_dot_for_ghost_house();

        if pellet.kind() == PelletKind::PowerPellet {
            self.ghosts.start_freight();
            self.events.push(GameEvent::PowerPelletEaten);
        }

        if self.pellets.is_empty() {
            self.easter_egg_autopilot.invalidate_route();
            self.events.push(GameEvent::LevelCompleted);
            self.flash_background = true;
            self.flash_timer = 0.0;
            self.background = self.background_norm.clone();
            self.hide_entities();
            self.pause.start_timed_pause(3.0, GameplayAction::NextLevel);
        }
    }

    /// Checks pellet events headless.
    fn check_pellet_events_headless(&mut self) {
        let Some(pellet) = self
            .pellets
            .try_eat(self.pacman.position(), self.pacman.collide_radius())
        else {
            return;
        };

        self.update_score_headless(pellet.points());
        self.events.push(GameEvent::SmallPelletEaten);
        self.pacman_pause_remaining += dot_pause_seconds(pellet.kind() == PelletKind::PowerPellet);
        self.record_dot_for_ghost_house();

        if pellet.kind() == PelletKind::PowerPellet {
            self.ghosts.start_freight();
            self.events.push(GameEvent::PowerPelletEaten);
        }

        if self.pellets.is_empty() {
            self.easter_egg_autopilot.invalidate_route();
            self.events.push(GameEvent::LevelCompleted);
            self.pause.start_timed_pause(3.0, GameplayAction::NextLevel);
        }
    }

    /// Checks ghost events.
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
                    .start_timed_pause(1.0, GameplayAction::ShowEntities);
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
                    GameplayAction::RestartGame
                } else {
                    GameplayAction::ResetLevel
                };
                self.pause.start_timed_pause(3.0, action);
            }
        }
    }

    /// Checks ghost events headless.
    fn check_ghost_events_headless(&mut self) {
        let mut collision = None;
        for ghost in self.ghosts.iter() {
            if self
                .pacman
                .collide_check(ghost.position(), ghost.collide_radius())
            {
                collision = Some((ghost.kind(), ghost.mode(), ghost.points()));
                break;
            }
        }

        let Some((ghost_kind, ghost_mode, ghost_points)) = collision else {
            return;
        };

        match ghost_mode {
            GhostMode::Freight => {
                self.update_score_headless(ghost_points);
                self.pause
                    .start_timed_pause(1.0, GameplayAction::ShowEntities);
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
                self.pacman.die();
                self.events.push(GameEvent::PacmanDied);
                let action = if self.lives == 0 {
                    GameplayAction::RestartGame
                } else {
                    GameplayAction::ResetLevel
                };
                self.pause.start_timed_pause(3.0, action);
            }
        }
    }

    /// Checks fruit events.
    fn check_fruit_events(&mut self) {
        for (index, threshold) in fruit_release_dots().into_iter().enumerate() {
            if !self.fruit_thresholds_spawned[index]
                && self.pellets.num_eaten() >= threshold
                && self.fruit.is_none()
            {
                self.fruit = Some(Fruit::for_level(
                    Vector2::new(
                        self.maze_spec.fruit_start_pixels.0,
                        self.maze_spec.fruit_start_pixels.1,
                    ),
                    self.level,
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
            if !self.fruit_captured.contains(&fruit_sprite_index) {
                self.fruit_captured.push(fruit_sprite_index);
            }
            self.events.push(GameEvent::FruitEaten);
            self.fruit = None;
        } else if expired {
            self.fruit = None;
        }
    }

    /// Checks fruit events headless.
    fn check_fruit_events_headless(&mut self) {
        for (index, threshold) in fruit_release_dots().into_iter().enumerate() {
            if !self.fruit_thresholds_spawned[index]
                && self.pellets.num_eaten() >= threshold
                && self.fruit.is_none()
            {
                self.fruit = Some(Fruit::for_level(
                    Vector2::new(
                        self.maze_spec.fruit_start_pixels.0,
                        self.maze_spec.fruit_start_pixels.1,
                    ),
                    self.level,
                ));
                self.fruit_thresholds_spawned[index] = true;
                break;
            }
        }

        let Some(fruit) = &self.fruit else {
            return;
        };

        let fruit_points = fruit.points();
        let fruit_sprite_index = fruit.sprite_index();
        let hit_fruit = self
            .pacman
            .collide_check(fruit.position(), fruit.collide_radius());
        let expired = fruit.destroyed();

        if hit_fruit {
            self.update_score_headless(fruit_points);
            if !self.fruit_captured.contains(&fruit_sprite_index) {
                self.fruit_captured.push(fruit_sprite_index);
            }
            self.events.push(GameEvent::FruitEaten);
            self.fruit = None;
        } else if expired {
            self.fruit = None;
        }
    }

    fn handle_after_pause(&mut self, action: GameplayAction) {
        match action {
            GameplayAction::ShowEntities => {
                self.text_group.hide_status();
                self.show_entities();
            }
            GameplayAction::ResetLevel => self.reset_level(),
            GameplayAction::NextLevel => {
                let flags = self.secret_mode_flags();
                *self = Self::start_level(
                    self.level + 1,
                    self.lives,
                    self.score,
                    self.fruit_captured.clone(),
                );
                self.apply_secret_mode_flags(flags);
            }
            GameplayAction::RestartGame => {
                self.return_to_title_requested = true;
            }
        }
    }

    fn handle_after_pause_headless(&mut self, action: GameplayAction) {
        match action {
            GameplayAction::ShowEntities => {}
            GameplayAction::ResetLevel => self.reset_level_headless(),
            GameplayAction::NextLevel => {
                let flags = self.secret_mode_flags();
                *self = Self::start_level(
                    self.level + 1,
                    self.lives,
                    self.score,
                    self.fruit_captured.clone(),
                );
                self.apply_secret_mode_flags(flags);
                self.pause.set_paused(false);
            }
            GameplayAction::RestartGame => {
                self.return_to_title_requested = true;
            }
        }
    }

    /// Resets level.
    fn reset_level(&mut self) {
        self.pacman.reset(&self.nodes);
        self.pacman_sprites.reset();
        self.ghosts.reset(&self.nodes, self.level);
        self.easter_egg_autopilot.invalidate_route();
        self.apply_post_death_release_rules();
        if self.easter_egg_force_freight {
            self.ghosts.start_freight();
        }
        self.fruit = None;
        self.flash_background = false;
        self.flash_timer = 0.0;
        self.background = self.background_norm.clone();
        self.show_entities();
        self.text_group.show_status(StatusText::Ready);
        self.pause
            .start_timed_pause(Self::READY_TIME, GameplayAction::ShowEntities);
    }

    /// Resets level headless.
    fn reset_level_headless(&mut self) {
        self.pacman.reset(&self.nodes);
        self.ghosts.reset(&self.nodes, self.level);
        self.easter_egg_autopilot.invalidate_route();
        self.apply_post_death_release_rules();
        if self.easter_egg_force_freight {
            self.ghosts.start_freight();
        }
        self.fruit = None;
        self.pause.set_paused(false);
    }

    fn headless_death_snapshot(&self) -> HeadlessDeathSnapshot {
        HeadlessDeathSnapshot {
            level: self.level,
            score: self.score,
            lives: self.lives,
            pellets_remaining: self.pellets.len(),
            pacman_position: self.pacman.position().as_tuple(),
            pacman_direction: self.pacman.direction(),
            ghosts: self
                .ghosts
                .iter()
                .map(|ghost| HeadlessGhostSnapshot {
                    kind: ghost.kind(),
                    mode: ghost.mode(),
                    position: ghost.position().as_tuple(),
                })
                .collect(),
            remaining_pellets: self
                .pellets
                .iter()
                .map(|pellet| pellet.position().as_tuple())
                .collect(),
        }
    }

    /// Shows entities.
    fn show_entities(&mut self) {
        self.pacman.show();
        self.ghosts.show();
    }

    /// Hides entities.
    fn hide_entities(&mut self) {
        self.pacman.hide();
        self.ghosts.hide();
    }

    /// Appends renderables.
    fn append_renderables(&self, frame: &mut FrameData) {
        frame.background = Some(self.background.clone());
        self.pellets.append_renderables(frame);

        if let Some(fruit) = &self.fruit {
            let image = self.fruit_sprites.item_image(fruit.sprite_index());
            frame.sprites.push(Sprite {
                position: gameplay_sprite_draw_position(fruit.position(), image.as_ref()),
                image,
                anchor: SpriteAnchor::TopLeft,
            });
        }
        if self.pacman.visible() && self.easter_egg_blink.visible() {
            let image = self.pacman_sprites.current();
            frame.sprites.push(Sprite {
                position: gameplay_sprite_draw_position(self.pacman.position(), image.as_ref()),
                image,
                anchor: SpriteAnchor::TopLeft,
            });
        }
        for ghost in self.ghosts.iter() {
            if ghost.visible() {
                let image = self.ghost_sprites.image(
                    ghost.kind(),
                    ghost.mode(),
                    ghost.direction(),
                    ghost.freight_remaining(),
                    ghost.fright_total_duration(),
                );
                frame.sprites.push(Sprite {
                    position: gameplay_sprite_draw_position(ghost.position(), image.as_ref()),
                    image,
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
            let image = self.fruit_sprites.icon_image(*fruit_index);
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

impl BlinkFeedback {
    const TOGGLES: u8 = 6;
    const INTERVAL: f32 = 0.1;

    /// Starts start.
    fn start(&mut self) {
        self.toggles_remaining = Self::TOGGLES;
        self.timer = 0.0;
        self.visible = false;
    }

    fn update(&mut self, dt: f32) {
        if self.toggles_remaining == 0 {
            self.visible = true;
            return;
        }

        self.timer += dt;
        while self.timer >= Self::INTERVAL && self.toggles_remaining > 0 {
            self.timer -= Self::INTERVAL;
            self.toggles_remaining = self.toggles_remaining.saturating_sub(1);
            self.visible = !self.visible;
        }

        if self.toggles_remaining == 0 {
            self.timer = 0.0;
            self.visible = true;
        }
    }

    fn visible(&self) -> bool {
        self.visible
    }
}

fn gameplay_sprite_draw_position(position: Vector2, _image: &RenderedImage) -> Vector2 {
    position - Vector2::new(TILE_WIDTH as f32 / 2.0, TILE_HEIGHT as f32 / 2.0)
}

impl Default for BlinkFeedback {
    fn default() -> Self {
        Self {
            toggles_remaining: 0,
            timer: 0.0,
            visible: true,
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

    /// Sets mouse position.
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

    /// Appends renderables.
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

impl TitleAttractScene {
    const SCENE_COUNT: usize = 3;

    const fn duration(self) -> f32 {
        match self {
            Self::Title => 6.0,
            Self::Scoring => 8.5,
            Self::Nicknames => 7.5,
        }
    }

    const fn next(self) -> Self {
        match self {
            Self::Title => Self::Scoring,
            Self::Scoring => Self::Nicknames,
            Self::Nicknames => Self::Title,
        }
    }
}

impl TitleScreenState {
    fn new() -> Self {
        Self {
            scene: TitleAttractScene::Title,
            scene_timer: 0.0,
            prompt_blink_timer: 0.0,
            prompt_visible: true,
            title_image: rasterize_text_image("PACMAN", YELLOW, 64.0),
            company_image: rasterize_text_image("1980 MIDWAY MFG CO", WHITE, 16.0),
            bonus_image: rasterize_text_image("BONUS PACMAN FOR 10000 PTS", WHITE, 16.0),
            push_start_image: rasterize_text_image("PUSH START BUTTON", YELLOW, 16.0),
            one_player_image: rasterize_text_image("1 PLAYER ONLY", WHITE, 16.0),
            two_player_image: rasterize_text_image("1 OR 2 PLAYERS", WHITE, 16.0),
            scoring_image: rasterize_text_image("SCORING", YELLOW, 24.0),
            pellet_score_image: rasterize_text_image("10 PTS", WHITE, 16.0),
            power_score_image: rasterize_text_image("50 PTS", WHITE, 16.0),
            ghost_score_images: [
                rasterize_text_image("200", WHITE, 16.0),
                rasterize_text_image("400", WHITE, 16.0),
                rasterize_text_image("800", WHITE, 16.0),
                rasterize_text_image("1600", WHITE, 16.0),
            ],
            fruit_score_images: [
                rasterize_text_image("100", WHITE, 12.0),
                rasterize_text_image("300", WHITE, 12.0),
                rasterize_text_image("500", WHITE, 12.0),
                rasterize_text_image("700", WHITE, 12.0),
                rasterize_text_image("1000", WHITE, 12.0),
                rasterize_text_image("2000", WHITE, 12.0),
                rasterize_text_image("3000", WHITE, 12.0),
                rasterize_text_image("5000", WHITE, 12.0),
            ],
            character_image: rasterize_text_image("CHARACTER", YELLOW, 24.0),
            nickname_image: rasterize_text_image("NICKNAME", WHITE, 16.0),
            nickname_rows: [
                AttractNicknameRow::new(GhostKind::Blinky, "SHADOW", "BLINKY", RED),
                AttractNicknameRow::new(GhostKind::Pinky, "SPEEDY", "PINKY", PINK),
                AttractNicknameRow::new(GhostKind::Inky, "BASHFUL", "INKY", TEAL),
                AttractNicknameRow::new(GhostKind::Clyde, "POKEY", "CLYDE", ORANGE),
            ],
            pacman_sprites: PacmanSprites::new(),
            ghost_sprites: GhostSprites::new(),
            fruit_sprites: FruitSprites::new(),
            button: TitleButtonState::new(
                SCREEN_WIDTH as f32 / 2.0 - 60.0,
                SCREEN_HEIGHT as f32 - 72.0,
                120,
                60,
                "START",
            ),
        }
    }

    fn update(&mut self, dt: f32, mouse_position: Option<Vector2>) {
        self.button.set_mouse_position(mouse_position);
        self.update_prompt_blink(dt);
        self.scene_timer += dt;
        for _ in 0..TitleAttractScene::SCENE_COUNT {
            let duration = self.scene.duration();
            if self.scene_timer < duration {
                break;
            }
            self.scene_timer -= duration;
            self.scene = self.scene.next();
            self.on_scene_changed();
        }

        if self.scene == TitleAttractScene::Scoring {
            self.pacman_sprites.update(dt, Direction::Right);
        }
    }

    /// Starts requested.
    fn start_requested(
        &mut self,
        start_requested: bool,
        mouse_click_position: Option<Vector2>,
    ) -> bool {
        if start_requested {
            return true;
        }

        if mouse_click_position.is_some_and(|position| self.click_starts(position)) {
            self.button.pressed = true;
            return true;
        }

        false
    }

    /// Updates prompt blink.
    fn update_prompt_blink(&mut self, dt: f32) {
        if self.scene != TitleAttractScene::Title {
            self.prompt_visible = false;
            self.prompt_blink_timer = 0.0;
            return;
        }

        self.prompt_blink_timer += dt;
        let toggles = (self.prompt_blink_timer / 0.35) as usize;
        self.prompt_blink_timer -= toggles as f32 * 0.35;
        if toggles % 2 == 1 {
            self.prompt_visible = !self.prompt_visible;
        }
    }

    fn on_scene_changed(&mut self) {
        self.pacman_sprites.reset();
        if self.scene == TitleAttractScene::Title {
            self.prompt_visible = true;
            self.prompt_blink_timer = 0.0;
        }
    }

    /// Appends renderables.
    fn append_renderables(&self, frame: &mut FrameData) {
        match self.scene {
            TitleAttractScene::Title => self.append_title_scene(frame),
            TitleAttractScene::Scoring => self.append_scoring_scene(frame),
            TitleAttractScene::Nicknames => self.append_nickname_scene(frame),
        }
        if self.shows_button() {
            self.button.append_renderables(frame);
        }
    }

    fn shows_button(&self) -> bool {
        self.scene == TitleAttractScene::Title
    }

    fn click_starts(&self, position: Vector2) -> bool {
        if self.shows_button() {
            self.button.contains(position)
        } else {
            true
        }
    }

    /// Appends title scene.
    fn append_title_scene(&self, frame: &mut FrameData) {
        append_centered_text(frame, &self.title_image, 68.0);
        append_centered_text(frame, &self.company_image, 168.0);
        append_centered_text(frame, &self.bonus_image, 204.0);
        if self.prompt_visible {
            append_centered_text(frame, &self.push_start_image, 286.0);
        }
        append_centered_text(frame, &self.one_player_image, 334.0);
        append_centered_text(frame, &self.two_player_image, 364.0);
    }

    /// Appends scoring scene.
    fn append_scoring_scene(&self, frame: &mut FrameData) {
        append_centered_text(frame, &self.scoring_image, 44.0);

        frame.circles.push(Circle {
            center: Vector2::new(112.0, 132.0),
            radius: 2.0,
            color: WHITE,
        });
        append_text(frame, &self.pellet_score_image, 156.0, 120.0);

        frame.circles.push(Circle {
            center: Vector2::new(112.0, 186.0),
            radius: 8.0,
            color: WHITE,
        });
        append_text(frame, &self.power_score_image, 156.0, 174.0);

        let ghost_positions = [176.0, 240.0, 304.0, 368.0];
        let eaten_times = [1.4, 2.1, 2.8, 3.5];
        let freight_image = self.ghost_sprites.image(
            GhostKind::Blinky,
            GhostMode::Freight,
            Direction::Left,
            Some(6.0),
            Some(6.0),
        );

        for (index, x) in ghost_positions.into_iter().enumerate() {
            if self.scene_timer >= eaten_times[index] {
                let score = &self.ghost_score_images[index];
                append_text(frame, score, x - score.width as f32 * 0.5, 248.0);
            } else {
                append_actor_sprite(frame, freight_image.clone(), Vector2::new(x, 264.0));
            }
        }

        let chase_window_start = 1.0;
        let chase_window_end = 4.1;
        if self.scene_timer >= chase_window_start {
            let progress = ((self.scene_timer - chase_window_start)
                / (chase_window_end - chase_window_start))
                .clamp(0.0, 1.0);
            let x = 88.0 + progress * 336.0;
            append_actor_sprite(frame, self.pacman_sprites.current(), Vector2::new(x, 264.0));
        }

        let fruit_positions = [
            Vector2::new(96.0, 394.0),
            Vector2::new(184.0, 394.0),
            Vector2::new(272.0, 394.0),
            Vector2::new(360.0, 394.0),
            Vector2::new(96.0, 470.0),
            Vector2::new(184.0, 470.0),
            Vector2::new(272.0, 470.0),
            Vector2::new(360.0, 470.0),
        ];
        for (index, position) in fruit_positions.into_iter().enumerate() {
            append_actor_sprite(frame, self.fruit_sprites.item_image(index), position);
            let score = &self.fruit_score_images[index];
            append_text(
                frame,
                score,
                position.x - score.width as f32 * 0.5,
                position.y + 22.0,
            );
        }
    }

    /// Appends nickname scene.
    fn append_nickname_scene(&self, frame: &mut FrameData) {
        append_centered_text(frame, &self.character_image, 36.0);
        append_centered_text(frame, &self.nickname_image, 72.0);

        for (index, row) in self.nickname_rows.iter().enumerate() {
            let local_time = self.scene_timer - index as f32 * 1.1;
            if local_time < 0.0 {
                continue;
            }

            let progress = (local_time / 0.55).clamp(0.0, 1.0);
            let center_y = 176.0 + index as f32 * 68.0;
            let center_x = -24.0 + progress * 124.0;
            let ghost_image =
                self.ghost_sprites
                    .image(row.kind, GhostMode::Chase, Direction::Right, None, None);
            append_actor_sprite(frame, ghost_image, Vector2::new(center_x, center_y));

            if local_time >= 0.2 {
                append_text(frame, &row.nickname_image, 128.0, center_y - 18.0);
                append_text(frame, &row.name_image, 320.0, center_y - 18.0);
            }
        }
    }
}

impl AttractNicknameRow {
    fn new(kind: GhostKind, nickname: &str, name: &str, name_color: [u8; 4]) -> Self {
        Self {
            kind,
            nickname_image: rasterize_text_image(nickname, WHITE, 16.0),
            name_image: rasterize_text_image(name, name_color, 16.0),
        }
    }
}

/// Appends text.
fn append_text(frame: &mut FrameData, image: &Arc<RenderedImage>, x: f32, y: f32) {
    frame.sprites.push(Sprite {
        image: image.clone(),
        position: Vector2::new(x, y),
        anchor: SpriteAnchor::TopLeft,
    });
}

/// Appends centered text.
fn append_centered_text(frame: &mut FrameData, image: &Arc<RenderedImage>, y: f32) {
    append_text(
        frame,
        image,
        (SCREEN_WIDTH as f32 - image.width as f32) * 0.5,
        y,
    );
}

/// Appends actor sprite.
fn append_actor_sprite(frame: &mut FrameData, image: Arc<RenderedImage>, center: Vector2) {
    frame.sprites.push(Sprite {
        image,
        position: center,
        anchor: SpriteAnchor::Center,
    });
}

impl AppState {
    fn new() -> Self {
        Self {
            title_screen: TitleScreenState::new(),
            gameplay: None,
            events: vec![GameEvent::TitleScreenEntered],
        }
    }

    fn update(&mut self, dt: f32, input: &UpdateInput) {
        if let Some(gameplay) = &mut self.gameplay {
            gameplay.update(
                dt,
                input.requested_direction,
                input.pause_requested,
                &input.typed_chars,
            );
            self.events.extend(gameplay.drain_events());

            if gameplay.return_to_title_requested {
                self.gameplay = None;
                self.title_screen = TitleScreenState::new();
                self.events.push(GameEvent::TitleScreenEntered);
            }
            return;
        }

        self.title_screen.update(dt, input.mouse_position);
        let should_start = self
            .title_screen
            .start_requested(input.start_requested, input.mouse_click_position);

        if input
            .mouse_click_position
            .is_some_and(|position| self.title_screen.click_starts(position))
        {
            self.events.push(GameEvent::ButtonClicked);
        }

        if should_start {
            self.gameplay = Some(GameplayState::new());
            self.events.push(GameEvent::GameStarted);
        }
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    /// Appends renderables.
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

#[cfg(test)]
mod tests {
    use super::{
        BlinkFeedback, Game, GameEvent, GameplayState, ORIGINAL_FRAME_TIME, SecretModeFlags,
        TitleAttractScene, TitleScreenState, UpdateInput,
    };
    use crate::{
        actors::EntityKind,
        constants::{TILE_HEIGHT, TILE_WIDTH},
        pacman::Direction,
        render::{FrameData, SpriteAnchor},
        sprites::PacmanSprites,
        vector::Vector2,
    };

    /// Starts game.
    fn start_game(game: &mut Game) {
        let _ = game.drain_events();
        game.update_with_input(
            0.0,
            UpdateInput {
                requested_direction: Direction::Stop,
                start_requested: true,
                ..UpdateInput::default()
            },
        );
        assert_eq!(game.drain_events(), vec![GameEvent::GameStarted]);
    }

    #[test]
    fn title_screen_emits_an_entered_event() {
        let mut game = Game::new();

        assert_eq!(game.drain_events(), vec![GameEvent::TitleScreenEntered]);
    }

    #[test]
    fn enter_starts_the_gameplay_screen() {
        let mut game = Game::new();
        start_game(&mut game);

        assert!(game.frame().background.is_some());
    }

    #[test]
    fn button_click_starts_the_gameplay_screen() {
        let mut game = Game::new();
        let _ = game.drain_events();
        let button_center = Vector2::new(
            game.state.title_screen.button.position.x + game.state.title_screen.button.size.x * 0.5,
            game.state.title_screen.button.position.y + game.state.title_screen.button.size.y * 0.5,
        );

        game.update_with_input(
            0.0,
            UpdateInput {
                requested_direction: Direction::Stop,
                mouse_position: Some(button_center),
                mouse_click_position: Some(button_center),
                ..UpdateInput::default()
            },
        );

        assert_eq!(
            game.drain_events(),
            vec![GameEvent::ButtonClicked, GameEvent::GameStarted]
        );
        assert!(game.frame().background.is_some());
    }

    #[test]
    fn button_click_uses_the_click_position() {
        let mut game = Game::new();
        let _ = game.drain_events();
        let button_center = Vector2::new(
            game.state.title_screen.button.position.x + game.state.title_screen.button.size.x * 0.5,
            game.state.title_screen.button.position.y + game.state.title_screen.button.size.y * 0.5,
        );

        game.update_with_input(
            0.0,
            UpdateInput {
                requested_direction: Direction::Stop,
                mouse_click_position: Some(button_center),
                ..UpdateInput::default()
            },
        );

        assert_eq!(
            game.drain_events(),
            vec![GameEvent::ButtonClicked, GameEvent::GameStarted]
        );
    }

    #[test]
    fn gameplay_renders_background_and_sprites() {
        let mut game = Game::new();
        start_game(&mut game);

        let frame = game.frame();
        assert!(frame.background.is_some());
        assert!(frame.sprites.len() >= 5);
    }

    #[test]
    fn title_screen_cycles_through_midway_attract_scenes() {
        let mut title = TitleScreenState::new();

        title.update(TitleAttractScene::Title.duration() + 0.1, None);
        assert_eq!(title.scene, TitleAttractScene::Scoring);

        title.update(TitleAttractScene::Scoring.duration() + 0.1, None);
        assert_eq!(title.scene, TitleAttractScene::Nicknames);

        title.update(TitleAttractScene::Nicknames.duration() + 0.1, None);
        assert_eq!(title.scene, TitleAttractScene::Title);
    }

    #[test]
    fn enter_can_start_from_scoring_scene() {
        let mut game = Game::new();
        let _ = game.drain_events();
        game.state
            .title_screen
            .update(TitleAttractScene::Title.duration() + 0.1, None);
        assert_eq!(game.state.title_screen.scene, TitleAttractScene::Scoring);

        game.update_with_input(
            0.0,
            UpdateInput {
                start_requested: true,
                ..UpdateInput::default()
            },
        );

        assert_eq!(game.drain_events(), vec![GameEvent::GameStarted]);
        assert!(game.frame().background.is_some());
    }

    #[test]
    fn scoring_scene_does_not_render_the_start_button() {
        let mut title = TitleScreenState::new();
        title.update(TitleAttractScene::Title.duration() + 0.1, None);

        let mut frame = FrameData::default();
        title.append_renderables(&mut frame);

        assert!(
            frame
                .sprites
                .iter()
                .all(|sprite| sprite.image.width != 120 || sprite.image.height != 60),
            "the title button should not be rendered on the scoring scene"
        );
    }

    #[test]
    fn arcade_mode_keeps_the_original_maze_on_level_two() {
        let state = GameplayState::start_level(2, 5, 0, Vec::new());

        assert_eq!(
            state.maze_spec.layout,
            crate::mazedata::MazeSpec::arcade().layout
        );
    }

    #[test]
    fn gameplay_updates_the_death_animation_while_paused() {
        let mut state = GameplayState::new();
        let before = state.pacman_sprites.current();
        state.pacman.die();

        state.update(0.2, Direction::Stop, false, &[]);

        assert_ne!(before.pixels, state.pacman_sprites.current().pixels);
    }

    #[test]
    fn gameplay_sprites_use_arcade_draw_offset() {
        let state = GameplayState::new();
        let mut frame = FrameData::default();
        state.append_renderables(&mut frame);

        assert_eq!(state.pacman.position().as_tuple(), (216.0, 416.0));
        let pacman_sprite = &frame.sprites[0];
        assert_eq!(pacman_sprite.anchor, SpriteAnchor::TopLeft);
        assert_eq!(
            pacman_sprite.position,
            state.pacman.position()
                - Vector2::new(TILE_WIDTH as f32 / 2.0, TILE_HEIGHT as f32 / 2.0)
        );
    }

    #[test]
    fn gameplay_pacman_sprite_tracks_direction() {
        let mut state = GameplayState::new();
        state.pause.set_paused(false);

        let left_pixels = PacmanSprites::new()
            .update_for_state(0.1, Direction::Left, true)
            .pixels
            .clone();

        state.update(0.1, Direction::Left, false, &[]);
        let mut frame = FrameData::default();
        state.append_renderables(&mut frame);
        assert_eq!(state.pacman.direction(), Direction::Left);
        assert_eq!(state.last_pacman_sprite_direction, Direction::Left);
        assert_eq!(state.pacman_sprites.current().pixels, left_pixels);
        assert_eq!(frame.sprites[0].image.pixels, left_pixels);
    }

    #[test]
    fn frightened_mode_temporarily_allows_red_zone_up_turns() {
        let mut state = GameplayState::new();
        let red_zone = state
            .nodes
            .get_node_from_tiles(12.0, 26.0)
            .expect("red-zone node should exist");

        assert!(
            !state
                .nodes
                .can_travel(red_zone, Direction::Up, EntityKind::Blinky)
        );

        state.ghosts.start_freight();
        state.sync_freight_events(false);

        assert!(
            state
                .nodes
                .can_travel(red_zone, Direction::Up, EntityKind::Blinky)
        );

        state.ghosts.end_freight();
        state.sync_freight_events(true);

        assert!(
            !state
                .nodes
                .can_travel(red_zone, Direction::Up, EntityKind::Blinky)
        );
    }

    #[test]
    fn freight_mode_keeps_red_zone_override_after_ghost_reset() {
        let mut state = GameplayState::new();
        let red_zone = state
            .nodes
            .get_node_from_tiles(12.0, 26.0)
            .expect("red-zone node should exist");

        state.ghosts.start_freight();
        state.sync_freight_events(false);
        state.reset_ghosts_to_start_positions();
        state.sync_freight_events(true);

        assert!(
            state
                .nodes
                .can_travel(red_zone, Direction::Up, EntityKind::Blinky)
        );
    }

    #[test]
    fn freight_mode_keeps_red_zone_override_after_level_rebuild() {
        let mut state = GameplayState::start_level(1, 5, 0, Vec::new());
        let flags = SecretModeFlags {
            easter_egg_active: true,
            easter_egg_force_freight: true,
            autopilot_active: false,
        };
        let red_zone = state
            .nodes
            .get_node_from_tiles(12.0, 26.0)
            .expect("red-zone node should exist");

        state.ghosts.start_freight();
        state.sync_freight_events(false);

        state = GameplayState::start_level(2, 5, 0, Vec::new());
        state.apply_secret_mode_flags(flags);
        state.sync_freight_events(true);

        assert!(
            state
                .nodes
                .can_travel(red_zone, Direction::Up, EntityKind::Blinky)
        );
    }

    #[test]
    fn q_quits_when_secret_mode_is_inactive() {
        let mut game = Game::new();

        game.update_with_input(
            0.0,
            UpdateInput {
                requested_direction: Direction::Stop,
                typed_chars: vec!['Q'],
                ..UpdateInput::default()
            },
        );

        assert!(game.quit_requested());
    }

    #[test]
    fn q_quits_even_when_secret_mode_is_active() {
        let mut game = Game::new();
        start_game(&mut game);

        game.update_with_input(
            0.0,
            UpdateInput {
                requested_direction: Direction::Stop,
                typed_chars: vec!['X', 'Y', 'Z', 'Z', 'Y'],
                ..UpdateInput::default()
            },
        );
        let _ = game.drain_events();

        game.update_with_input(
            0.0,
            UpdateInput {
                requested_direction: Direction::Stop,
                typed_chars: vec!['Q'],
                ..UpdateInput::default()
            },
        );

        assert!(game.quit_requested());
    }

    #[test]
    fn xyzzy_toggles_secret_mode_and_starts_blink_feedback() {
        let mut state = GameplayState::new();

        state.handle_easter_egg_input(&['X', 'Y', 'Z', 'Z', 'Y']);

        assert!(state.easter_egg_active);
        assert_eq!(
            state.easter_egg_blink.toggles_remaining,
            BlinkFeedback::TOGGLES
        );

        state.handle_easter_egg_input(&['X', 'Y', 'Z', 'Z', 'Y']);

        assert!(!state.easter_egg_active);
        assert_eq!(
            state.easter_egg_blink.toggles_remaining,
            BlinkFeedback::TOGGLES
        );
    }

    #[test]
    fn secret_a_toggles_autopilot_and_secret_mode_off_disables_it() {
        let mut state = GameplayState::new();

        state.handle_easter_egg_input(&['X', 'Y', 'Z', 'Z', 'Y']);
        state.handle_easter_egg_input(&['A']);
        assert!(state.easter_egg_autopilot.active());

        state.handle_easter_egg_input(&['A']);
        assert!(!state.easter_egg_autopilot.active());

        state.handle_easter_egg_input(&['A']);
        assert!(state.easter_egg_autopilot.active());

        state.handle_easter_egg_input(&['X', 'Y', 'Z', 'Z', 'Y']);
        assert!(!state.easter_egg_active);
        assert!(!state.easter_egg_autopilot.active());
    }

    #[test]
    fn secret_f_toggles_freight_mode_without_requesting_quit() {
        let mut game = Game::new();
        start_game(&mut game);

        game.update_with_input(
            0.0,
            UpdateInput {
                requested_direction: Direction::Stop,
                typed_chars: vec!['X', 'Y', 'Z', 'Z', 'Y'],
                ..UpdateInput::default()
            },
        );
        assert!(!game.quit_requested());

        game.update_with_input(
            0.0,
            UpdateInput {
                requested_direction: Direction::Stop,
                typed_chars: vec!['F'],
                ..UpdateInput::default()
            },
        );
        assert_eq!(game.drain_events(), vec![GameEvent::FreightModeStarted]);

        let state = game
            .state
            .gameplay
            .as_ref()
            .expect("expected gameplay state");
        assert!(state.easter_egg_force_freight);
        assert!(!game.quit_requested());

        game.update_with_input(
            0.0,
            UpdateInput {
                requested_direction: Direction::Stop,
                typed_chars: vec!['F'],
                ..UpdateInput::default()
            },
        );

        assert_eq!(game.drain_events(), vec![GameEvent::FreightModeEnded]);
    }

    #[test]
    fn freight_events_track_direct_mode_transitions() {
        let mut state = GameplayState::new();

        state.ghosts.start_freight();
        state.sync_freight_events(false);
        assert_eq!(state.drain_events(), vec![GameEvent::FreightModeStarted]);

        state.ghosts.end_freight();
        state.sync_freight_events(true);
        assert_eq!(state.drain_events(), vec![GameEvent::FreightModeEnded]);
    }

    #[test]
    fn secret_t_teleports_pacman_to_the_safest_node() {
        let mut state = GameplayState::new();
        state.easter_egg_active = true;

        state.handle_easter_egg_input(&['t']);

        let expected = state
            .nodes
            .node_ids()
            .filter(|&node_id| {
                Direction::cardinals().into_iter().any(|direction| {
                    state
                        .nodes
                        .can_travel(node_id, direction, EntityKind::Pacman)
                })
            })
            .max_by(|&lhs, &rhs| {
                let lhs_distance = state.minimum_ghost_distance_squared(lhs);
                let rhs_distance = state.minimum_ghost_distance_squared(rhs);
                lhs_distance.total_cmp(&rhs_distance)
            })
            .expect("a safest node should exist");

        assert_eq!(state.pacman.current_node(), expected);
    }

    #[test]
    fn secret_r_sends_ghosts_back_to_their_start_positions() {
        let mut state = GameplayState::new();
        state.easter_egg_active = true;
        let expected_positions: Vec<_> = GameplayState::new()
            .ghosts
            .iter()
            .map(|ghost| ghost.position())
            .collect();

        state.handle_easter_egg_input(&['r']);

        let reset_positions: Vec<_> = state.ghosts.iter().map(|ghost| ghost.position()).collect();
        assert_eq!(reset_positions, expected_positions);
    }

    /// Runs secret autopilot until done.
    fn run_secret_autopilot_until_done(state: &mut GameplayState, step_limit: usize) -> bool {
        let dt = ORIGINAL_FRAME_TIME;
        for _ in 0..step_limit {
            state.update_headless(dt);
            let events = state.drain_events();
            if events.contains(&GameEvent::PacmanDied) {
                return false;
            }
            if events.contains(&GameEvent::LevelCompleted) || state.pellets.is_empty() {
                return true;
            }
        }

        false
    }

    fn single_pellet_reset_state(target: Vector2) -> GameplayState {
        let mut state = GameplayState::new();
        let positions: Vec<_> = state
            .pellets
            .iter()
            .map(|pellet| pellet.position())
            .collect();
        for position in positions {
            if position != target {
                let removed = state.pellets.try_eat(position, 0.0);
                assert!(removed.is_some(), "expected pellet at {position}");
            }
        }

        state.reset_level();
        state.pause.set_paused(false);
        state.easter_egg_active = true;
        state.easter_egg_autopilot.toggle();
        state
    }

    #[test]
    #[ignore = "expensive end-to-end autopilot simulation"]
    fn secret_autopilot_clears_the_level_without_losing_a_life() {
        fastrand::seed(7);

        let mut state = GameplayState::new();
        state.pause.set_paused(false);
        state.easter_egg_active = true;
        state.easter_egg_autopilot.toggle();
        let starting_lives = state.lives;
        if !run_secret_autopilot_until_done(&mut state, 8_000) {
            panic!(
                "autopilot died after eating {} pellets: {:?}",
                state.pellets.num_eaten(),
                state.headless_death_snapshot()
            );
        }
        assert_eq!(state.lives, starting_lives);
    }

    #[test]
    #[ignore = "expensive secret autopilot regression"]
    fn secret_autopilot_handles_top_centre_last_pellet_resets() {
        let failing_positions = [
            Vector2::new(192.0, 80.0),
            Vector2::new(240.0, 80.0),
            Vector2::new(192.0, 96.0),
            Vector2::new(240.0, 96.0),
            Vector2::new(192.0, 112.0),
        ];

        for target in failing_positions {
            fastrand::seed(7);
            let mut state = single_pellet_reset_state(target);
            assert!(
                run_secret_autopilot_until_done(&mut state, 1_200),
                "autopilot failed to clear single pellet at {:?}",
                target.as_tuple()
            );
        }
    }
}
