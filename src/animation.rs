//! Provides simple frame-based animation helpers used by sprites and attract-sequence visuals.

#[derive(Clone, Debug)]
pub struct Animator<T: Clone> {
    frames: Vec<T>,
    current_frame: usize,
    speed: f32,
    looped: bool,
    dt: f32,
    finished: bool,
}

impl<T: Clone> Animator<T> {
    pub fn new(frames: Vec<T>, speed: f32, looped: bool) -> Self {
        assert!(!frames.is_empty(), "animator requires at least one frame");

        Self {
            frames,
            current_frame: 0,
            speed,
            looped,
            dt: 0.0,
            finished: false,
        }
    }

    /// Resets reset.
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.dt = 0.0;
        self.finished = false;
    }

    pub fn update(&mut self, dt: f32) -> T {
        if !self.finished {
            self.advance(dt);
        }

        if self.current_frame >= self.frames.len() {
            if self.looped {
                self.current_frame = 0;
            } else {
                self.finished = true;
                self.current_frame = self.frames.len() - 1;
            }
        }

        self.frames[self.current_frame].clone()
    }

    /// Advances advance.
    fn advance(&mut self, dt: f32) {
        self.dt += dt;
        let frame_time = 1.0 / self.speed.max(0.0001);
        if self.dt >= frame_time {
            self.current_frame += 1;
            self.dt = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Animator;

    #[test]
    fn looping_animator_wraps_to_the_start() {
        let mut animator = Animator::new(vec![1, 2], 20.0, true);

        assert_eq!(animator.update(0.0), 1);
        assert_eq!(animator.update(0.1), 2);
        assert_eq!(animator.update(0.1), 1);
    }

    #[test]
    /// Resets returns to the first frame.
    fn reset_returns_to_the_first_frame() {
        let mut animator = Animator::new(vec![1, 2, 3], 20.0, true);
        let _ = animator.update(0.2);

        animator.reset();

        assert_eq!(animator.update(0.0), 1);
    }
}
