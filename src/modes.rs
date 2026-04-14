#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GhostMode {
    Scatter,
    Chase,
    Freight,
    Spawn,
}

#[derive(Clone, Debug)]
struct MainMode {
    timer: f32,
    mode: GhostMode,
    time: f32,
}

#[derive(Clone, Debug)]
pub struct ModeController {
    timer: f32,
    time: Option<f32>,
    main_mode: MainMode,
    current: GhostMode,
}

impl MainMode {
    fn new() -> Self {
        let mut mode = Self {
            timer: 0.0,
            mode: GhostMode::Scatter,
            time: 7.0,
        };
        mode.scatter();
        mode
    }

    fn update(&mut self, dt: f32) {
        self.timer += dt;
        if self.timer >= self.time {
            match self.mode {
                GhostMode::Scatter => self.chase(),
                GhostMode::Chase => self.scatter(),
                GhostMode::Freight | GhostMode::Spawn => {}
            }
        }
    }

    fn scatter(&mut self) {
        self.mode = GhostMode::Scatter;
        self.time = 7.0;
        self.timer = 0.0;
    }

    fn chase(&mut self) {
        self.mode = GhostMode::Chase;
        self.time = 20.0;
        self.timer = 0.0;
    }
}

impl ModeController {
    pub fn new() -> Self {
        let main_mode = MainMode::new();
        let current = main_mode.mode;
        Self {
            timer: 0.0,
            time: None,
            main_mode,
            current,
        }
    }

    pub fn current(&self) -> GhostMode {
        self.current
    }

    pub fn update(&mut self, dt: f32, at_spawn_node: bool) -> bool {
        let mut reset_to_normal = false;

        self.main_mode.update(dt);
        match self.current {
            GhostMode::Freight => {
                self.timer += dt;
                if self.time.is_some_and(|time| self.timer >= time) {
                    self.time = None;
                    self.current = self.main_mode.mode;
                    reset_to_normal = true;
                }
            }
            GhostMode::Scatter | GhostMode::Chase => {
                self.current = self.main_mode.mode;
            }
            GhostMode::Spawn => {
                if at_spawn_node {
                    self.current = self.main_mode.mode;
                    reset_to_normal = true;
                }
            }
        }

        reset_to_normal
    }

    pub fn set_spawn_mode(&mut self) {
        if self.current == GhostMode::Freight {
            self.current = GhostMode::Spawn;
        }
    }

    pub fn set_freight_mode(&mut self) {
        match self.current {
            GhostMode::Scatter | GhostMode::Chase => {
                self.timer = 0.0;
                self.time = Some(7.0);
                self.current = GhostMode::Freight;
            }
            GhostMode::Freight => {
                self.timer = 0.0;
            }
            GhostMode::Spawn => {}
        }
    }

    pub fn clear_freight_mode(&mut self) -> bool {
        if self.current != GhostMode::Freight {
            return false;
        }

        self.timer = 0.0;
        self.time = None;
        self.current = self.main_mode.mode;
        true
    }

    pub fn freight_remaining(&self) -> Option<f32> {
        (self.current == GhostMode::Freight)
            .then(|| (self.time.unwrap_or(0.0) - self.timer).max(0.0))
    }
}

impl Default for ModeController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{GhostMode, ModeController};

    #[test]
    fn main_mode_toggles_from_scatter_to_chase() {
        let mut controller = ModeController::new();
        assert_eq!(controller.current(), GhostMode::Scatter);

        controller.update(7.1, false);

        assert_eq!(controller.current(), GhostMode::Chase);
    }

    #[test]
    fn freight_mode_times_out_back_to_main_mode() {
        let mut controller = ModeController::new();
        controller.set_freight_mode();
        assert_eq!(controller.current(), GhostMode::Freight);

        let reset = controller.update(7.1, false);

        assert!(reset);
        assert_eq!(controller.current(), GhostMode::Chase);
    }

    #[test]
    fn spawn_mode_returns_to_main_mode_at_spawn_node() {
        let mut controller = ModeController::new();
        controller.set_freight_mode();
        controller.set_spawn_mode();

        let reset = controller.update(0.1, true);

        assert!(reset);
        assert_eq!(controller.current(), GhostMode::Scatter);
    }
}
