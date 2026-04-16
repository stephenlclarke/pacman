//! Translates terminal keyboard and mouse events into game-friendly input state.

use std::time::Duration;

use anyhow::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind,
};

use crate::pacman::Direction;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseCell {
    column: u16,
    row: u16,
}

impl MouseCell {
    /// Creates new.
    fn new(column: u16, row: u16) -> Self {
        Self { column, row }
    }

    /// Handles column.
    pub fn column(self) -> u16 {
        self.column
    }

    /// Handles row.
    pub fn row(self) -> u16 {
        self.row
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputState {
    direction: Direction,
    quit: bool,
    mouse_cell: Option<MouseCell>,
}

impl InputState {
    /// Handles direction.
    pub fn direction(self) -> Direction {
        self.direction
    }

    /// Handles requested.
    pub fn quit_requested(self) -> bool {
        self.quit
    }

    /// Translates cell.
    pub fn mouse_cell(self) -> Option<MouseCell> {
        self.mouse_cell
    }
}

#[derive(Debug, Default)]
pub struct InputController {
    state: InputState,
    pause_requested: bool,
    start_requested: bool,
    mouse_click: Option<MouseCell>,
    typed_chars: Vec<char>,
}

impl InputController {
    /// Polls poll.
    pub fn poll(&mut self) -> Result<()> {
        self.poll_for(Duration::ZERO)
    }

    /// Polls for.
    pub fn poll_for(&mut self, timeout: Duration) -> Result<()> {
        // Branch based on the current runtime condition.
        if !event::poll(timeout)? {
            return Ok(());
        }

        self.handle_event(event::read()?);
        // Continue processing while the guard condition remains true.
        while event::poll(Duration::ZERO)? {
            self.handle_event(event::read()?);
        }
        Ok(())
    }

    /// Handles direction.
    pub fn direction(&self) -> Direction {
        self.state.direction()
    }

    /// Handles requested.
    pub fn quit_requested(&self) -> bool {
        self.state.quit_requested()
    }

    /// Handles pause requested.
    pub fn take_pause_requested(&mut self) -> bool {
        std::mem::take(&mut self.pause_requested)
    }

    /// Handles start requested.
    pub fn take_start_requested(&mut self) -> bool {
        std::mem::take(&mut self.start_requested)
    }

    /// Translates cell.
    pub fn mouse_cell(&self) -> Option<MouseCell> {
        self.state.mouse_cell()
    }

    /// Handles mouse click.
    pub fn take_mouse_click(&mut self) -> Option<MouseCell> {
        self.mouse_click.take()
    }

    /// Handles typed chars.
    pub fn take_typed_chars(&mut self) -> Vec<char> {
        std::mem::take(&mut self.typed_chars)
    }

    /// Handles event.
    fn handle_event(&mut self, event: Event) {
        // Select the next behavior based on the current state.
        match event {
            Event::Key(key_event) => self.handle_key(key_event),
            Event::Mouse(mouse_event) => self.handle_mouse(mouse_event),
            Event::Resize(_, _) => {}
            _ => {}
        }
    }

    /// Handles key.
    fn handle_key(&mut self, key_event: KeyEvent) {
        let is_pressed = matches!(key_event.kind, KeyEventKind::Press);

        // Branch based on the current runtime condition.
        if is_pressed
            && let KeyCode::Char(character) = key_event.code
            && character.is_ascii_alphabetic()
        {
            self.typed_chars.push(character.to_ascii_lowercase());
        }

        // Select the next behavior based on the current state.
        match key_event.code {
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') if is_pressed => {
                self.state.direction = Direction::Up
            }
            KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') if is_pressed => {
                self.state.direction = Direction::Down
            }
            KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') if is_pressed => {
                self.state.direction = Direction::Left
            }
            KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') if is_pressed => {
                self.state.direction = Direction::Right
            }
            KeyCode::Char(' ') if is_pressed => {
                self.pause_requested = true;
            }
            KeyCode::Enter if is_pressed => {
                self.start_requested = true;
            }
            KeyCode::Esc if is_pressed => {
                self.state.quit = true;
            }
            _ => {}
        }
    }

    /// Handles mouse.
    fn handle_mouse(&mut self, mouse_event: MouseEvent) {
        let mouse_cell = MouseCell::new(mouse_event.column, mouse_event.row);
        self.state.mouse_cell = Some(mouse_cell);

        // Branch based on the current runtime condition.
        if matches!(mouse_event.kind, MouseEventKind::Down(MouseButton::Left)) {
            self.mouse_click = Some(mouse_cell);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{InputController, InputState, MouseCell};
    use crate::pacman::Direction;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };

    #[test]
    /// Handles direction press replaces the queued turn.
    fn latest_direction_press_replaces_the_queued_turn() {
        let mut input = InputController::default();

        let mut up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        up.kind = KeyEventKind::Press;
        input.handle_key(up);

        let mut left = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        left.kind = KeyEventKind::Press;
        input.handle_key(left);

        assert_eq!(input.direction(), Direction::Left);
    }

    #[test]
    /// Handles keys means stop.
    fn no_keys_means_stop() {
        assert_eq!(InputState::default().direction(), Direction::Stop);
    }

    #[test]
    /// Handles a direction does not clear the queue.
    fn releasing_a_direction_does_not_clear_the_queue() {
        let mut input = InputController::default();

        let mut up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        up.kind = KeyEventKind::Press;
        input.handle_key(up);

        up.kind = KeyEventKind::Release;
        input.handle_key(up);

        assert_eq!(input.direction(), Direction::Up);
    }

    #[test]
    /// Handles events do not override the latest press.
    fn repeat_events_do_not_override_the_latest_press() {
        let mut input = InputController::default();

        let mut up = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        up.kind = KeyEventKind::Press;
        input.handle_key(up);

        let mut left = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
        left.kind = KeyEventKind::Press;
        input.handle_key(left);

        up.kind = KeyEventKind::Repeat;
        input.handle_key(up);

        assert_eq!(input.direction(), Direction::Left);
    }

    #[test]
    /// Handles requests a pause toggle.
    fn spacebar_requests_a_pause_toggle() {
        let mut input = InputController::default();
        let mut key_event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        key_event.kind = KeyEventKind::Press;
        input.handle_key(key_event);

        assert!(input.take_pause_requested());
        assert!(!input.take_pause_requested());
    }

    #[test]
    /// Handles requests a game start.
    fn enter_requests_a_game_start() {
        let mut input = InputController::default();
        let mut key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        key_event.kind = KeyEventKind::Press;
        input.handle_key(key_event);

        assert!(input.take_start_requested());
        assert!(!input.take_start_requested());
    }

    #[test]
    /// Handles mouse down tracks position and click.
    fn left_mouse_down_tracks_position_and_click() {
        let mut input = InputController::default();
        input.handle_mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 12,
            row: 9,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(input.mouse_cell(), Some(MouseCell::new(12, 9)));
        assert_eq!(input.take_mouse_click(), Some(MouseCell::new(12, 9)));
    }

    #[test]
    /// Handles is exposed as a typed character instead of an immediate quit.
    fn q_is_exposed_as_a_typed_character_instead_of_an_immediate_quit() {
        let mut input = InputController::default();
        let mut key_event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        key_event.kind = KeyEventKind::Press;
        input.handle_key(key_event);

        assert_eq!(input.take_typed_chars(), vec!['q']);
        assert!(!input.quit_requested());
    }
}
