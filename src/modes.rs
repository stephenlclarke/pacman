//! Manages ghost mode timing, frightened-state lifetime, and mode transitions.

use crate::arcade::{chase_durations, level_spec, scatter_durations};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GhostMode {
    Scatter,
    Chase,
    Freight,
    Spawn,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ModeUpdate {
    pub returned_to_normal: bool,
    pub reversed: bool,
}

#[derive(Clone, Copy, Debug)]
enum PhaseKind {
    Scatter,
    Chase,
}

#[derive(Clone, Copy, Debug)]
struct MainPhase {
    kind: PhaseKind,
    duration: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct ModeController {
    phases: Vec<MainPhase>,
    phase_index: usize,
    phase_timer: f32,
    fright_timer: f32,
    fright_time: f32,
    current: GhostMode,
}

impl ModeController {
    /// Creates new.
    pub fn new(level: u32) -> Self {
        let scatters = scatter_durations(level);
        let chases = chase_durations(level);
        let phases = vec![
            MainPhase {
                kind: PhaseKind::Scatter,
                duration: Some(scatters[0]),
            },
            MainPhase {
                kind: PhaseKind::Chase,
                duration: chases[0],
            },
            MainPhase {
                kind: PhaseKind::Scatter,
                duration: Some(scatters[1]),
            },
            MainPhase {
                kind: PhaseKind::Chase,
                duration: chases[1],
            },
            MainPhase {
                kind: PhaseKind::Scatter,
                duration: Some(scatters[2]),
            },
            MainPhase {
                kind: PhaseKind::Chase,
                duration: chases[2],
            },
            MainPhase {
                kind: PhaseKind::Scatter,
                duration: Some(scatters[3]),
            },
            MainPhase {
                kind: PhaseKind::Chase,
                duration: chases[3],
            },
        ];
        let spec = level_spec(level);
        Self {
            phases,
            phase_index: 0,
            phase_timer: 0.0,
            fright_timer: 0.0,
            fright_time: spec.frightened_time,
            current: GhostMode::Scatter,
        }
    }

    /// Handles current.
    pub fn current(&self) -> GhostMode {
        self.current
    }

    /// Updates update.
    pub fn update(&mut self, dt: f32, at_spawn_node: bool) -> ModeUpdate {
        let mut update = ModeUpdate::default();

        // Branch based on the current runtime condition.
        if self.current == GhostMode::Spawn {
            // Branch based on the current runtime condition.
            if at_spawn_node {
                self.current = self.main_mode();
                update.returned_to_normal = true;
            }
            return update;
        }

        // Branch based on the current runtime condition.
        if self.current == GhostMode::Freight {
            self.fright_timer += dt;
            // Branch based on the current runtime condition.
            if self.fright_timer >= self.fright_time {
                self.current = self.main_mode();
                self.fright_timer = 0.0;
                update.returned_to_normal = true;
            }
            return update;
        }

        self.phase_timer += dt;
        // Keep looping until a break condition exits the block.
        loop {
            let Some(duration) = self.phases[self.phase_index].duration else {
                break;
            };
            // Branch based on the current runtime condition.
            if self.phase_timer < duration {
                break;
            }
            self.phase_timer -= duration;
            // Branch based on the current runtime condition.
            if self.phase_index + 1 >= self.phases.len() {
                break;
            }
            self.phase_index += 1;
            self.current = self.main_mode();
            update.reversed = true;
        }
        self.current = self.main_mode();
        update
    }

    /// Sets spawn mode.
    pub fn set_spawn_mode(&mut self) {
        // Branch based on the current runtime condition.
        if self.current == GhostMode::Freight {
            self.current = GhostMode::Spawn;
            self.fright_timer = 0.0;
        }
    }

    /// Sets freight mode.
    pub fn set_freight_mode(&mut self) -> bool {
        let reversed = matches!(
            self.current,
            GhostMode::Scatter | GhostMode::Chase | GhostMode::Freight
        );
        // Branch based on the current runtime condition.
        if self.fright_time <= 0.0 {
            return reversed;
        }

        // Select the next behavior based on the current state.
        match self.current {
            GhostMode::Scatter | GhostMode::Chase | GhostMode::Freight => {
                self.current = GhostMode::Freight;
                self.fright_timer = 0.0;
            }
            GhostMode::Spawn => {}
        }
        reversed
    }

    /// Clears freight mode.
    pub fn clear_freight_mode(&mut self) -> bool {
        // Branch based on the current runtime condition.
        if self.current != GhostMode::Freight {
            return false;
        }

        self.fright_timer = 0.0;
        self.current = self.main_mode();
        true
    }

    /// Handles remaining.
    pub fn fright_remaining(&self) -> Option<f32> {
        (self.current == GhostMode::Freight)
            .then(|| (self.fright_time - self.fright_timer).max(0.0))
    }

    /// Handles total duration.
    pub fn fright_total_duration(&self) -> Option<f32> {
        (self.current == GhostMode::Freight).then_some(self.fright_time)
    }

    /// Handles mode.
    fn main_mode(&self) -> GhostMode {
        // Select the next behavior based on the current state.
        match self.phases[self.phase_index].kind {
            PhaseKind::Scatter => GhostMode::Scatter,
            PhaseKind::Chase => GhostMode::Chase,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GhostMode, ModeController};

    #[test]
    /// Handles mode toggles from scatter to chase.
    fn main_mode_toggles_from_scatter_to_chase() {
        let mut controller = ModeController::new(1);
        assert_eq!(controller.current(), GhostMode::Scatter);

        let update = controller.update(7.1, false);

        assert!(update.reversed);
        assert_eq!(controller.current(), GhostMode::Chase);
    }

    #[test]
    /// Handles mode times out back to main mode.
    fn freight_mode_times_out_back_to_main_mode() {
        let mut controller = ModeController::new(1);
        assert!(controller.set_freight_mode());
        assert_eq!(controller.current(), GhostMode::Freight);

        let update = controller.update(6.1, false);

        assert!(update.returned_to_normal);
        assert_eq!(controller.current(), GhostMode::Scatter);
    }

    #[test]
    /// Handles mode pauses the main mode timer.
    fn freight_mode_pauses_the_main_mode_timer() {
        let mut controller = ModeController::new(1);

        let update = controller.update(3.0, false);
        assert!(!update.reversed);
        assert_eq!(controller.current(), GhostMode::Scatter);

        assert!(controller.set_freight_mode());
        let update = controller.update(3.0, false);
        assert!(!update.reversed);
        assert_eq!(controller.current(), GhostMode::Freight);

        assert!(controller.clear_freight_mode());
        let update = controller.update(3.8, false);
        assert!(!update.reversed);
        assert_eq!(controller.current(), GhostMode::Scatter);

        let update = controller.update(0.2, false);
        assert!(update.reversed);
        assert_eq!(controller.current(), GhostMode::Chase);
    }

    #[test]
    /// Handles mode returns to main mode at spawn node.
    fn spawn_mode_returns_to_main_mode_at_spawn_node() {
        let mut controller = ModeController::new(1);
        assert!(controller.set_freight_mode());
        controller.set_spawn_mode();

        let update = controller.update(0.1, true);

        assert!(update.returned_to_normal);
        assert_eq!(controller.current(), GhostMode::Scatter);
    }
}
