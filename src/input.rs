use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use crate::pacman::Direction;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputState {
    direction: Direction,
    quit: bool,
}

impl InputState {
    pub fn direction(self) -> Direction {
        self.direction
    }

    pub fn quit_requested(self) -> bool {
        self.quit
    }
}

#[derive(Debug, Default)]
pub struct InputController {
    state: InputState,
    pause_requested: bool,
}

impl InputController {
    pub fn poll(&mut self) -> Result<()> {
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(key_event) => self.handle_key(key_event),
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        Ok(())
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

    fn handle_key(&mut self, key_event: KeyEvent) {
        let is_pressed = matches!(key_event.kind, KeyEventKind::Press);

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
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc if is_pressed => {
                self.state.quit = true;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{InputController, InputState};
    use crate::pacman::Direction;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    #[test]
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
    fn no_keys_means_stop() {
        assert_eq!(InputState::default().direction(), Direction::Stop);
    }

    #[test]
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
    fn spacebar_requests_a_pause_toggle() {
        let mut input = InputController::default();
        let mut key_event = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        key_event.kind = KeyEventKind::Press;
        input.handle_key(key_event);

        assert!(input.take_pause_requested());
        assert!(!input.take_pause_requested());
    }
}
