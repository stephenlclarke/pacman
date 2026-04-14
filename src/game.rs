use std::sync::Arc;

use crate::{
    actors::{EntityKind, GhostKind},
    autopilot::AutoPilot,
    constants::{
        BUTTON_CLICK, BUTTON_COLOR, BUTTON_HOVER, SCREEN_HEIGHT, SCREEN_WIDTH, TILE_HEIGHT,
        TILE_WIDTH, WHITE, YELLOW,
    },
    fruit::Fruit,
    ghosts::GhostGroup,
    mazedata::MazeSpec,
    modes::GhostMode,
    nodes::NodeGroup,
    pacman::{Direction, NodePacman},
    pause::PauseController,
    pellets::{PelletGroup, PelletKind},
    render::{FrameData, RenderedImage, Sprite, SpriteAnchor},
    sprites::{FruitSprites, GhostSprites, LifeSprites, MazeSprites, PacmanSprites},
    text::{StatusText, TextGroup, rasterize_text_image},
    vector::Vector2,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Level6Action {
    ShowEntities,
    ResetLevel,
    NextLevel,
    RestartGame,
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
    state: Level7State,
    quit_requested: bool,
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
    title_image: Arc<RenderedImage>,
    button: TitleButtonState,
}

#[derive(Clone, Debug)]
struct Level7State {
    title_screen: TitleScreenState,
    gameplay: Option<Level6State>,
    events: Vec<GameEvent>,
}

impl Game {
    pub fn new() -> Self {
        Self {
            state: Level7State::new(),
            quit_requested: false,
        }
    }

    pub fn update_with_input(&mut self, dt: f32, input: UpdateInput) {
        let q_pressed = input.typed_chars.contains(&'q');
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
    let mut pacman = NodePacman::new(pacman_start, &nodes);
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

impl Level6State {
    const FRUIT_THRESHOLDS: [usize; 2] = [50, 140];
    const FLASH_TIME: f32 = 0.2;
    const EASTER_EGG_CODE: [char; 5] = ['x', 'y', 'z', 'z', 'y'];

    fn new() -> Self {
        Self::start_level(1, 5, 0, Vec::new())
    }

    fn start_level(level: u32, lives: u32, score: u32, fruit_captured: Vec<usize>) -> Self {
        let maze_spec = MazeSpec::for_level(level, true);
        let (nodes, pacman, pellets, ghosts) = build_gameplay_level(maze_spec);
        let maze_sprites = MazeSprites::from_layout(maze_spec.layout, maze_spec.rotation);
        let background_norm = maze_sprites.construct_background(level);
        let background_flash = maze_sprites.construct_flash_background();

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
        }
    }

    fn update(
        &mut self,
        dt: f32,
        requested_direction: Direction,
        pause_requested: bool,
        typed_chars: &[char],
    ) {
        let mut requested_direction = requested_direction;
        let freight_was_active = self.ghosts.has_freight_mode();
        self.handle_easter_egg_input(typed_chars);
        self.easter_egg_blink.update(dt);
        self.text_group.update(dt);
        self.pellets.update(dt);

        if !self.pause.paused() {
            self.sustain_secret_freight_mode();
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
                if self.easter_egg_autopilot.active() {
                    requested_direction = self.easter_egg_autopilot.choose_direction(
                        &self.nodes,
                        &self.pacman,
                        &self.pellets,
                        &self.ghosts,
                        self.fruit.as_ref(),
                    );
                }
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
            if self.flash_timer >= Self::FLASH_TIME {
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

        self.sync_freight_events(freight_was_active);
    }

    fn drain_events(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }

    fn handle_easter_egg_input(&mut self, typed_chars: &[char]) {
        for &character in typed_chars {
            let is_secret_code_input = self.update_easter_egg_sequence(character);

            if !self.easter_egg_active {
                continue;
            }

            match character {
                'a' => {
                    self.easter_egg_autopilot.toggle();
                }
                'f' => self.toggle_secret_freight_mode(),
                't' => self.teleport_pacman_to_safest_node(),
                'r' if !is_secret_code_input => self.reset_ghosts_to_start_positions(),
                _ => {}
            }
        }
    }

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

    fn toggle_easter_egg_mode(&mut self) {
        self.easter_egg_active = !self.easter_egg_active;
        self.easter_egg_sequence_index = 0;
        self.easter_egg_blink.start();
        if !self.easter_egg_active {
            self.easter_egg_autopilot.disable();
            self.disable_secret_freight_mode();
        }
    }

    fn toggle_secret_freight_mode(&mut self) {
        if self.easter_egg_force_freight {
            self.disable_secret_freight_mode();
            return;
        }

        self.easter_egg_force_freight = true;
        self.ghosts.start_freight();
    }

    fn disable_secret_freight_mode(&mut self) {
        if !self.easter_egg_force_freight {
            return;
        }

        self.easter_egg_force_freight = false;
        self.ghosts.end_freight();
    }

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

    fn minimum_ghost_distance_squared(&self, node_id: usize) -> f32 {
        let position = self.nodes.position(node_id);
        self.ghosts
            .iter()
            .map(|ghost| (position - ghost.position()).magnitude_squared())
            .fold(f32::INFINITY, f32::min)
    }

    fn reset_ghosts_to_start_positions(&mut self) {
        self.ghosts.reset(&self.nodes);
        self.restore_ghost_access_rules();
        if self.easter_egg_force_freight {
            self.ghosts.start_freight();
        }
    }

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
    }

    fn secret_mode_flags(&self) -> (bool, bool) {
        (self.easter_egg_active, self.easter_egg_force_freight)
    }

    fn apply_secret_mode_flags(&mut self, easter_egg_active: bool, easter_egg_force_freight: bool) {
        self.easter_egg_active = easter_egg_active;
        self.easter_egg_force_freight = easter_egg_force_freight;
        self.easter_egg_sequence_index = 0;
        if self.easter_egg_force_freight {
            self.ghosts.start_freight();
        }
    }

    fn sync_freight_events(&mut self, freight_was_active: bool) {
        let freight_is_active = self.ghosts.has_freight_mode();
        match (freight_was_active, freight_is_active) {
            (false, true) => self.events.push(GameEvent::FreightModeStarted),
            (true, false) => self.events.push(GameEvent::FreightModeEnded),
            _ => {}
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
            self.easter_egg_autopilot.disable();
            self.events.push(GameEvent::LevelCompleted);
            self.flash_background = true;
            self.flash_timer = 0.0;
            self.background = self.background_norm.clone();
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
                    self.level.saturating_sub(1),
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

    fn handle_after_pause(&mut self, action: Level6Action) {
        match action {
            Level6Action::ShowEntities => {
                self.text_group.hide_status();
                self.show_entities();
            }
            Level6Action::ResetLevel => self.reset_level(),
            Level6Action::NextLevel => {
                let (easter_egg_active, easter_egg_force_freight) = self.secret_mode_flags();
                *self = Self::start_level(
                    self.level + 1,
                    self.lives,
                    self.score,
                    self.fruit_captured.clone(),
                );
                self.apply_secret_mode_flags(easter_egg_active, easter_egg_force_freight);
            }
            Level6Action::RestartGame => {
                self.return_to_title_requested = true;
            }
        }
    }

    fn reset_level(&mut self) {
        self.pause.set_paused(true);
        self.pacman.reset(&self.nodes);
        self.pacman_sprites.reset();
        self.ghosts.reset(&self.nodes);
        self.restore_ghost_access_rules();
        if self.easter_egg_force_freight {
            self.ghosts.start_freight();
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
        if self.pacman.visible() && self.easter_egg_blink.visible() {
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

impl BlinkFeedback {
    const TOGGLES: u8 = 6;
    const INTERVAL: f32 = 0.1;

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
    fn new() -> Self {
        Self {
            title_image: rasterize_text_image("PACMAN", YELLOW, 64.0),
            button: TitleButtonState::new(
                SCREEN_WIDTH as f32 / 2.0 - 60.0,
                SCREEN_HEIGHT as f32 / 2.0 - 30.0,
                120,
                60,
                "START",
            ),
        }
    }

    fn update(&mut self, mouse_position: Option<Vector2>) {
        self.button.set_mouse_position(mouse_position);
    }

    fn start_requested(
        &mut self,
        start_requested: bool,
        mouse_click_position: Option<Vector2>,
    ) -> bool {
        if start_requested {
            return true;
        }

        if mouse_click_position.is_some_and(|position| self.button.contains(position)) {
            self.button.pressed = true;
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
        self.button.append_renderables(frame);
    }
}

impl Level7State {
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

        self.title_screen.update(input.mouse_position);
        let should_start = self
            .title_screen
            .start_requested(input.start_requested, input.mouse_click_position);

        if input
            .mouse_click_position
            .is_some_and(|position| self.title_screen.button.contains(position))
        {
            self.events.push(GameEvent::ButtonClicked);
        }

        if should_start {
            self.gameplay = Some(Level6State::new());
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
    use super::{BlinkFeedback, Game, GameEvent, Level6State, UpdateInput, sprite_draw_position};
    use crate::{
        actors::EntityKind,
        constants::{SCREEN_HEIGHT, SCREEN_WIDTH, TILE_HEIGHT, TILE_WIDTH},
        pacman::Direction,
        render::{FrameData, SpriteAnchor},
        vector::Vector2,
    };

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
    fn level7_title_screen_emits_an_entered_event() {
        let mut game = Game::new();

        assert_eq!(game.drain_events(), vec![GameEvent::TitleScreenEntered]);
    }

    #[test]
    fn level7_enter_starts_the_gameplay_screen() {
        let mut game = Game::new();
        start_game(&mut game);

        assert!(game.frame().background.is_some());
    }

    #[test]
    fn level7_button_click_starts_the_gameplay_screen() {
        let mut game = Game::new();
        let _ = game.drain_events();
        let button_center = Vector2::new(SCREEN_WIDTH as f32 / 2.0, SCREEN_HEIGHT as f32 / 2.0);

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
    fn level7_button_click_uses_the_click_position() {
        let mut game = Game::new();
        let _ = game.drain_events();
        let button_center = Vector2::new(SCREEN_WIDTH as f32 / 2.0, SCREEN_HEIGHT as f32 / 2.0);

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
    fn level7_gameplay_renders_background_and_sprites() {
        let mut game = Game::new();
        start_game(&mut game);

        let frame = game.frame();
        assert!(frame.background.is_some());
        assert!(frame.sprites.len() >= 5);
    }

    #[test]
    fn level7_uses_the_second_maze_on_level_two() {
        let state = Level6State::start_level(2, 5, 0, Vec::new());

        assert_eq!(state.maze_spec.name, "maze2");
    }

    #[test]
    fn level7_updates_the_death_animation_while_paused() {
        let mut state = Level6State::new();
        let before = state.pacman_sprites.current();
        state.pacman.die();

        state.update(0.2, Direction::Stop, false, &[]);

        assert_ne!(before.pixels, state.pacman_sprites.current().pixels);
    }

    #[test]
    fn gameplay_sprite_positions_match_tutorial_entity_offset() {
        let state = Level6State::new();
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
    }

    #[test]
    fn q_quits_when_secret_mode_is_inactive() {
        let mut game = Game::new();

        game.update_with_input(
            0.0,
            UpdateInput {
                requested_direction: Direction::Stop,
                typed_chars: vec!['q'],
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
                typed_chars: vec!['x', 'y', 'z', 'z', 'y'],
                ..UpdateInput::default()
            },
        );
        let _ = game.drain_events();

        game.update_with_input(
            0.0,
            UpdateInput {
                requested_direction: Direction::Stop,
                typed_chars: vec!['q'],
                ..UpdateInput::default()
            },
        );

        assert!(game.quit_requested());
    }

    #[test]
    fn xyzzy_toggles_secret_mode_and_starts_blink_feedback() {
        let mut state = Level6State::new();

        state.handle_easter_egg_input(&['x', 'y', 'z', 'z', 'y']);

        assert!(state.easter_egg_active);
        assert_eq!(
            state.easter_egg_blink.toggles_remaining,
            BlinkFeedback::TOGGLES
        );

        state.handle_easter_egg_input(&['x', 'y', 'z', 'z', 'y']);

        assert!(!state.easter_egg_active);
        assert_eq!(
            state.easter_egg_blink.toggles_remaining,
            BlinkFeedback::TOGGLES
        );
    }

    #[test]
    fn secret_a_toggles_autopilot_and_secret_mode_off_disables_it() {
        let mut state = Level6State::new();

        state.handle_easter_egg_input(&['x', 'y', 'z', 'z', 'y']);
        state.handle_easter_egg_input(&['a']);
        assert!(state.easter_egg_autopilot.active());

        state.handle_easter_egg_input(&['a']);
        assert!(!state.easter_egg_autopilot.active());

        state.handle_easter_egg_input(&['a']);
        assert!(state.easter_egg_autopilot.active());

        state.handle_easter_egg_input(&['x', 'y', 'z', 'z', 'y']);
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
                typed_chars: vec!['x', 'y', 'z', 'z', 'y'],
                ..UpdateInput::default()
            },
        );
        assert!(!game.quit_requested());

        game.update_with_input(
            0.0,
            UpdateInput {
                requested_direction: Direction::Stop,
                typed_chars: vec!['f'],
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
                typed_chars: vec!['f'],
                ..UpdateInput::default()
            },
        );

        assert_eq!(game.drain_events(), vec![GameEvent::FreightModeEnded]);
    }

    #[test]
    fn freight_events_track_direct_mode_transitions() {
        let mut state = Level6State::new();

        state.ghosts.start_freight();
        state.sync_freight_events(false);
        assert_eq!(state.drain_events(), vec![GameEvent::FreightModeStarted]);

        state.ghosts.end_freight();
        state.sync_freight_events(true);
        assert_eq!(state.drain_events(), vec![GameEvent::FreightModeEnded]);
    }

    #[test]
    fn secret_t_teleports_pacman_to_the_safest_node() {
        let mut state = Level6State::new();
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
        let mut state = Level6State::new();
        state.easter_egg_active = true;
        let expected_positions: Vec<_> = Level6State::new()
            .ghosts
            .iter()
            .map(|ghost| ghost.position())
            .collect();

        state.handle_easter_egg_input(&['r']);

        let reset_positions: Vec<_> = state.ghosts.iter().map(|ghost| ghost.position()).collect();
        assert_eq!(reset_positions, expected_positions);
    }
}
