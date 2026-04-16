//! Provides a reusable pause controller for player pauses and scripted timed pauses.

#[derive(Clone, Debug)]
pub struct PauseController<T> {
    paused: bool,
    timer: f32,
    pause_time: Option<f32>,
    after_pause: Option<T>,
}

impl<T: Copy> PauseController<T> {
    /// Creates new.
    pub fn new(paused: bool) -> Self {
        Self {
            paused,
            timer: 0.0,
            pause_time: None,
            after_pause: None,
        }
    }

    /// Handles paused.
    pub fn paused(&self) -> bool {
        self.paused
    }

    /// Handles timed.
    pub fn is_timed(&self) -> bool {
        self.pause_time.is_some()
    }

    /// Toggles toggle.
    pub fn toggle(&mut self) -> bool {
        self.timer = 0.0;
        self.pause_time = None;
        self.after_pause = None;
        self.paused = !self.paused;
        self.paused
    }

    /// Sets paused.
    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
        self.timer = 0.0;
        self.pause_time = None;
        self.after_pause = None;
    }

    /// Starts timed pause.
    pub fn start_timed_pause(&mut self, pause_time: f32, after_pause: T) {
        self.paused = true;
        self.timer = 0.0;
        self.pause_time = Some(pause_time);
        self.after_pause = Some(after_pause);
    }

    /// Updates update.
    pub fn update(&mut self, dt: f32) -> Option<T> {
        let pause_time = self.pause_time?;

        self.timer += dt;
        // Branch based on the current runtime condition.
        if self.timer < pause_time {
            return None;
        }

        self.paused = false;
        self.timer = 0.0;
        self.pause_time = None;
        self.after_pause.take()
    }
}

#[cfg(test)]
mod tests {
    use super::PauseController;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Action {
        Resume,
    }

    #[test]
    /// Handles pause flips the state.
    fn player_pause_flips_the_state() {
        let mut pause = PauseController::<Action>::new(true);

        assert!(!pause.toggle());
        assert!(pause.toggle());
    }

    #[test]
    /// Handles pause returns its followup action.
    fn timed_pause_returns_its_followup_action() {
        let mut pause = PauseController::new(false);
        pause.start_timed_pause(1.0, Action::Resume);

        assert_eq!(pause.update(0.5), None);
        assert_eq!(pause.update(0.6), Some(Action::Resume));
        assert!(!pause.paused());
    }

    #[test]
    /// Handles pause state is visible to callers.
    fn timed_pause_state_is_visible_to_callers() {
        let mut pause = PauseController::new(false);
        assert!(!pause.is_timed());

        pause.start_timed_pause(1.0, Action::Resume);

        assert!(pause.is_timed());
    }
}
