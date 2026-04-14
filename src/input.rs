use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

use crate::pacman::Direction;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InputState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    quit: bool,
}

impl InputState {
    pub fn direction(self) -> Direction {
        if self.up {
            Direction::Up
        } else if self.down {
            Direction::Down
        } else if self.left {
            Direction::Left
        } else if self.right {
            Direction::Right
        } else {
            Direction::Stop
        }
    }

    pub fn quit_requested(self) -> bool {
        self.quit
    }
}

#[derive(Debug, Default)]
pub struct InputController {
    state: InputState,
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

    fn handle_key(&mut self, key_event: KeyEvent) {
        let is_pressed = matches!(key_event.kind, KeyEventKind::Press | KeyEventKind::Repeat);
        let is_released = matches!(key_event.kind, KeyEventKind::Release);

        match key_event.code {
            KeyCode::Up | KeyCode::Char('w') | KeyCode::Char('W') => {
                update_button(&mut self.state.up, is_pressed, is_released)
            }
            KeyCode::Down | KeyCode::Char('s') | KeyCode::Char('S') => {
                update_button(&mut self.state.down, is_pressed, is_released)
            }
            KeyCode::Left | KeyCode::Char('a') | KeyCode::Char('A') => {
                update_button(&mut self.state.left, is_pressed, is_released)
            }
            KeyCode::Right | KeyCode::Char('d') | KeyCode::Char('D') => {
                update_button(&mut self.state.right, is_pressed, is_released)
            }
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc if is_pressed => {
                self.state.quit = true;
            }
            _ => {}
        }
    }
}

fn update_button(button: &mut bool, is_pressed: bool, is_released: bool) {
    if is_pressed {
        *button = true;
    } else if is_released {
        *button = false;
    }
}

#[cfg(test)]
mod tests {
    use super::InputState;
    use crate::pacman::Direction;

    #[test]
    fn direction_priority_matches_python_tutorial() {
        let state = InputState {
            up: false,
            down: true,
            left: true,
            right: true,
            quit: false,
        };

        assert_eq!(state.direction(), Direction::Down);
    }

    #[test]
    fn no_keys_means_stop() {
        assert_eq!(InputState::default().direction(), Direction::Stop);
    }
}
