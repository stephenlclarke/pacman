//! Translates window keyboard and mouse events into game-friendly input state.

use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    keyboard::{Key, NamedKey},
};

use crate::pacman::Direction;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MousePosition {
    x: f32,
    y: f32,
}

impl MousePosition {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn x(self) -> f32 {
        self.x
    }

    pub fn y(self) -> f32 {
        self.y
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InputState {
    direction: Direction,
    quit: bool,
    mouse_position: Option<MousePosition>,
}

impl InputState {
    pub fn direction(self) -> Direction {
        self.direction
    }

    pub fn quit_requested(self) -> bool {
        self.quit
    }

    pub fn mouse_position(self) -> Option<MousePosition> {
        self.mouse_position
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputKey {
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Character(char),
    Enter,
    Escape,
}

#[derive(Debug, Default)]
pub struct InputController {
    state: InputState,
    pause_requested: bool,
    start_requested: bool,
    mouse_click: Option<MousePosition>,
    typed_chars: Vec<char>,
}

impl InputController {
    pub fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                let is_pressed = matches!(event.state, ElementState::Pressed) && !event.repeat;
                if let Some(key) = key_from_logical_key(event.logical_key.as_ref()) {
                    self.handle_key(key, is_pressed);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_mouse_move(MousePosition::new(position.x as f32, position.y as f32));
            }
            WindowEvent::CursorLeft { .. } => {
                self.state.mouse_position = None;
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state: ElementState::Pressed,
                ..
            } => {
                self.handle_left_mouse_down();
            }
            _ => {}
        }
    }

    pub fn direction(&self) -> Direction {
        self.state.direction()
    }

    pub fn quit_requested(&self) -> bool {
        self.state.quit_requested()
    }

    pub fn take_pause_requested(&mut self) -> bool {
        std::mem::take(&mut self.pause_requested)
    }

    pub fn take_start_requested(&mut self) -> bool {
        std::mem::take(&mut self.start_requested)
    }

    pub fn mouse_position(&self) -> Option<MousePosition> {
        self.state.mouse_position()
    }

    pub fn take_mouse_click(&mut self) -> Option<MousePosition> {
        self.mouse_click.take()
    }

    pub fn take_typed_chars(&mut self) -> Vec<char> {
        std::mem::take(&mut self.typed_chars)
    }

    fn handle_key(&mut self, key: InputKey, is_pressed: bool) {
        if is_pressed
            && let InputKey::Character(character) = key
            && character.is_ascii_alphabetic()
        {
            self.typed_chars.push(character.to_ascii_lowercase());
        }

        match key {
            InputKey::ArrowUp | InputKey::Character('w') | InputKey::Character('W')
                if is_pressed =>
            {
                self.state.direction = Direction::Up;
            }
            InputKey::ArrowDown | InputKey::Character('s') | InputKey::Character('S')
                if is_pressed =>
            {
                self.state.direction = Direction::Down;
            }
            InputKey::ArrowLeft | InputKey::Character('a') | InputKey::Character('A')
                if is_pressed =>
            {
                self.state.direction = Direction::Left;
            }
            InputKey::ArrowRight | InputKey::Character('d') | InputKey::Character('D')
                if is_pressed =>
            {
                self.state.direction = Direction::Right;
            }
            InputKey::Character(' ') if is_pressed => {
                self.pause_requested = true;
            }
            InputKey::Enter if is_pressed => {
                self.start_requested = true;
            }
            InputKey::Escape if is_pressed => {
                self.state.quit = true;
            }
            _ => {}
        }
    }

    fn handle_mouse_move(&mut self, position: MousePosition) {
        self.state.mouse_position = Some(position);
    }

    fn handle_left_mouse_down(&mut self) {
        self.mouse_click = self.state.mouse_position;
    }
}

fn key_from_logical_key(key: Key<&str>) -> Option<InputKey> {
    match key {
        Key::Named(NamedKey::ArrowUp) => Some(InputKey::ArrowUp),
        Key::Named(NamedKey::ArrowDown) => Some(InputKey::ArrowDown),
        Key::Named(NamedKey::ArrowLeft) => Some(InputKey::ArrowLeft),
        Key::Named(NamedKey::ArrowRight) => Some(InputKey::ArrowRight),
        Key::Named(NamedKey::Enter) => Some(InputKey::Enter),
        Key::Named(NamedKey::Escape) => Some(InputKey::Escape),
        Key::Named(NamedKey::Space) => Some(InputKey::Character(' ')),
        Key::Character(value) => single_character(value).map(InputKey::Character),
        _ => None,
    }
}

fn single_character(value: &str) -> Option<char> {
    let mut characters = value.chars();
    let character = characters.next()?;
    if characters.next().is_some() {
        return None;
    }
    Some(character)
}

#[cfg(test)]
mod tests {
    use super::{InputController, InputKey, InputState, MousePosition, key_from_logical_key};
    use crate::pacman::Direction;
    use winit::keyboard::{Key, NamedKey};

    #[test]
    fn latest_direction_press_replaces_the_queued_turn() {
        let mut input = InputController::default();

        input.handle_key(InputKey::ArrowUp, true);
        input.handle_key(InputKey::ArrowLeft, true);

        assert_eq!(input.direction(), Direction::Left);
    }

    #[test]
    fn no_keys_means_stop() {
        assert_eq!(InputState::default().direction(), Direction::Stop);
    }

    #[test]
    fn releasing_a_direction_does_not_clear_the_queue() {
        let mut input = InputController::default();

        input.handle_key(InputKey::ArrowUp, true);
        input.handle_key(InputKey::ArrowUp, false);

        assert_eq!(input.direction(), Direction::Up);
    }

    #[test]
    fn repeat_events_do_not_override_the_latest_press() {
        let mut input = InputController::default();

        input.handle_key(InputKey::ArrowUp, true);
        input.handle_key(InputKey::ArrowLeft, true);
        input.handle_key(InputKey::ArrowUp, false);

        assert_eq!(input.direction(), Direction::Left);
    }

    #[test]
    fn spacebar_requests_a_pause_toggle() {
        let mut input = InputController::default();

        input.handle_key(InputKey::Character(' '), true);

        assert!(input.take_pause_requested());
        assert!(!input.take_pause_requested());
    }

    #[test]
    fn enter_requests_a_game_start() {
        let mut input = InputController::default();

        input.handle_key(InputKey::Enter, true);

        assert!(input.take_start_requested());
        assert!(!input.take_start_requested());
    }

    #[test]
    fn left_mouse_down_tracks_position_and_click() {
        let mut input = InputController::default();
        let position = MousePosition::new(12.0, 9.0);

        input.handle_mouse_move(position);
        input.handle_left_mouse_down();

        assert_eq!(input.mouse_position(), Some(position));
        assert_eq!(input.take_mouse_click(), Some(position));
    }

    #[test]
    fn q_is_exposed_as_a_typed_character_instead_of_an_immediate_quit() {
        let mut input = InputController::default();

        input.handle_key(InputKey::Character('Q'), true);

        assert_eq!(input.take_typed_chars(), vec!['q']);
        assert!(!input.quit_requested());
    }

    #[test]
    fn uppercase_direction_keys_match_lowercase_mappings() {
        let mut input = InputController::default();

        input.handle_key(InputKey::Character('W'), true);
        assert_eq!(input.direction(), Direction::Up);

        input.handle_key(InputKey::Character('D'), true);
        assert_eq!(input.direction(), Direction::Right);
    }

    #[test]
    fn winit_named_keys_map_to_game_keys() {
        assert_eq!(
            key_from_logical_key(Key::Named(NamedKey::ArrowUp)),
            Some(InputKey::ArrowUp)
        );
        assert_eq!(
            key_from_logical_key(Key::Named(NamedKey::Escape)),
            Some(InputKey::Escape)
        );
    }

    #[test]
    fn winit_character_keys_keep_single_ascii_characters() {
        assert_eq!(
            key_from_logical_key(Key::Character("W")),
            Some(InputKey::Character('W'))
        );
        assert_eq!(key_from_logical_key(Key::Character("XY")), None);
    }
}
