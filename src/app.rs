//! Runs the window application loop, input handling, fixed-timestep updates, and frame presentation.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

use crate::{
    arcade::ORIGINAL_FRAME_TIME,
    audio::AudioManager,
    constants::{SCREEN_HEIGHT, SCREEN_WIDTH},
    game::{Game, UpdateInput},
    input::{InputController, MousePosition},
    render::{RenderTargetSize, Renderer},
    wgpu_graphics::WgpuGraphics,
};

const MAX_DT: f32 = 0.1;

pub fn run() -> Result<()> {
    parse_args(std::env::args().skip(1))?;

    let event_loop = EventLoop::new().context("creating window event loop")?;
    let mut app = PacmanApp::new();
    event_loop
        .run_app(&mut app)
        .context("running window event loop")?;

    if let Some(error) = app.error {
        Err(error)
    } else {
        Ok(())
    }
}

struct PacmanApp {
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    renderer: Option<Renderer>,
    graphics: Option<WgpuGraphics>,
    input: InputController,
    game: Game,
    audio: AudioManager,
    frame_time: f32,
    frame_duration: Duration,
    pending_input: UpdateInput,
    accumulator: f32,
    last_tick: Instant,
    next_frame: Instant,
    error: Option<anyhow::Error>,
}

impl PacmanApp {
    fn new() -> Self {
        let mut game = Game::load();
        let mut audio = AudioManager::new();
        for event in game.drain_events() {
            audio.handle_event(event);
        }

        let frame_time = ORIGINAL_FRAME_TIME;
        let now = Instant::now();

        Self {
            window: None,
            window_id: None,
            renderer: None,
            graphics: None,
            input: InputController::default(),
            game,
            audio,
            frame_time,
            frame_duration: Duration::from_secs_f32(frame_time),
            pending_input: UpdateInput::default(),
            accumulator: 0.0,
            last_tick: now,
            next_frame: now,
            error: None,
        }
    }

    fn initialize_window(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let logical_size = LogicalSize::new(SCREEN_WIDTH as f64, SCREEN_HEIGHT as f64);
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Pac-Man")
                        .with_inner_size(logical_size)
                        .with_min_inner_size(logical_size),
                )
                .context("creating Pac-Man window")?,
        );
        let physical_size = window.inner_size();
        let graphics = pollster::block_on(WgpuGraphics::new(window.clone()))?;
        let renderer = Renderer::new(render_target_size(physical_size));
        let now = Instant::now();

        self.window_id = Some(window.id());
        self.window = Some(window);
        self.graphics = Some(graphics);
        self.renderer = Some(renderer);
        self.last_tick = now;
        self.next_frame = now;

        Ok(())
    }

    fn handle_resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        let size = render_target_size(size);
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(size);
        }
        if let Some(graphics) = &mut self.graphics {
            graphics.resize(size);
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) -> Result<()> {
        let frame_started = Instant::now();
        let update_input = {
            let renderer = self
                .renderer
                .as_ref()
                .context("renderer was not initialized")?;
            collect_update_input(&mut self.input, renderer)
        };

        let dt = self.last_tick.elapsed().as_secs_f32().min(MAX_DT);
        self.last_tick = Instant::now();
        self.accumulator = (self.accumulator + dt).min(self.frame_time * 8.0);

        merge_input(&mut self.pending_input, update_input);

        if advance_game(
            &mut self.game,
            &mut self.pending_input,
            &mut self.accumulator,
            self.frame_time,
        ) {
            event_loop.exit();
            return Ok(());
        }

        {
            let renderer = self
                .renderer
                .as_mut()
                .context("renderer was not initialized")?;
            let graphics = self
                .graphics
                .as_mut()
                .context("wgpu graphics were not initialized")?;
            render_frame(&mut self.game, &mut self.audio, renderer, graphics)?;
        }

        schedule_next_frame(&mut self.next_frame, frame_started, self.frame_duration);
        if self.game.quit_requested() {
            event_loop.exit();
        }

        Ok(())
    }

    fn stop_with_error(&mut self, event_loop: &ActiveEventLoop, error: anyhow::Error) {
        if self.error.is_none() {
            self.error = Some(error);
        }
        event_loop.exit();
    }
}

impl ApplicationHandler for PacmanApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        if let Err(error) = self.initialize_window(event_loop) {
            self.stop_with_error(event_loop, error);
            return;
        }

        if let Some(window) = &self.window {
            window.request_redraw();
        }
        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }

        self.input.handle_window_event(&event);
        if self.input.quit_requested() {
            event_loop.exit();
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => self.handle_resize(size),
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.redraw(event_loop) {
                    self.stop_with_error(event_loop, error);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.error.is_some() {
            event_loop.exit();
            return;
        }

        let Some(window) = &self.window else {
            return;
        };

        let now = Instant::now();
        if now >= self.next_frame {
            window.request_redraw();
            event_loop.set_control_flow(ControlFlow::Poll);
        } else {
            event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame));
        }
    }
}

fn render_target_size(size: PhysicalSize<u32>) -> RenderTargetSize {
    RenderTargetSize::new(size.width, size.height)
}

fn collect_update_input(input: &mut InputController, renderer: &Renderer) -> UpdateInput {
    UpdateInput {
        requested_direction: input.direction(),
        pause_requested: input.take_pause_requested(),
        start_requested: input.take_start_requested(),
        mouse_position: mouse_scene_position(input.mouse_position(), renderer),
        mouse_click_position: mouse_scene_position(input.take_mouse_click(), renderer),
        typed_chars: input.take_typed_chars(),
    }
}

fn mouse_scene_position(
    mouse_position: Option<MousePosition>,
    renderer: &Renderer,
) -> Option<crate::vector::Vector2> {
    mouse_position
        .and_then(|position| renderer.scene_position_for_pixel(position.x(), position.y()))
}

fn advance_game(
    game: &mut Game,
    pending_input: &mut UpdateInput,
    accumulator: &mut f32,
    frame_time: f32,
) -> bool {
    let steps = whole_steps(accumulator, frame_time);
    for step_index in 0..steps {
        let step_input = if step_index == 0 {
            pending_input.clone()
        } else {
            step_input_without_one_shots(pending_input)
        };
        game.update_with_input(frame_time, step_input);
        if game.quit_requested() {
            return true;
        }
    }
    if steps > 0 {
        clear_one_shots(pending_input);
    }
    game.quit_requested()
}

fn whole_steps(accumulator: &mut f32, frame_time: f32) -> usize {
    let steps = (*accumulator / frame_time) as usize;
    *accumulator -= steps as f32 * frame_time;
    steps
}

fn render_frame(
    game: &mut Game,
    audio: &mut AudioManager,
    renderer: &mut Renderer,
    graphics: &mut WgpuGraphics,
) -> Result<()> {
    for event in game.drain_events() {
        audio.handle_event(event);
    }
    let frame = game.frame();
    let image = renderer.render(&frame);
    graphics.draw_frame(image)?;
    Ok(())
}

fn schedule_next_frame(next_frame: &mut Instant, frame_started: Instant, frame_duration: Duration) {
    *next_frame = frame_started + frame_duration;
    while *next_frame <= Instant::now() {
        *next_frame += frame_duration;
    }
}

fn merge_input(pending: &mut UpdateInput, next: UpdateInput) {
    pending.requested_direction = next.requested_direction;
    pending.mouse_position = next.mouse_position;
    pending.pause_requested |= next.pause_requested;
    pending.start_requested |= next.start_requested;
    if next.mouse_click_position.is_some() {
        pending.mouse_click_position = next.mouse_click_position;
    }
    pending.typed_chars.extend(next.typed_chars);
}

fn clear_one_shots(pending: &mut UpdateInput) {
    pending.pause_requested = false;
    pending.start_requested = false;
    pending.mouse_click_position = None;
    pending.typed_chars.clear();
}

fn step_input_without_one_shots(pending: &UpdateInput) -> UpdateInput {
    UpdateInput {
        requested_direction: pending.requested_direction,
        pause_requested: false,
        start_requested: false,
        mouse_position: pending.mouse_position,
        mouse_click_position: None,
        typed_chars: Vec::new(),
    }
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<()> {
    let args: Vec<String> = args.collect();
    if args.is_empty() {
        return Ok(());
    }

    match args.as_slice() {
        [flag] if matches!(flag.as_str(), "-h" | "--help") => {
            print_help();
            std::process::exit(0);
        }
        [arg] => {
            bail!(
                "unexpected argument {arg:?}. This binary only supports the default game launch. \
                 Use `cargo run` or `cargo run -- --help`."
            )
        }
        _ => bail!(
            "unexpected arguments. This binary only supports the default game launch. \
             Use `cargo run` or `cargo run -- --help`."
        ),
    }
}

/// Prints help.
fn print_help() {
    println!(
        "Usage: cargo run [-- --help]

Running `cargo run` launches the game in a wgpu window.

Controls:
  Arrow keys / WASD  Move Pacman
  Space              Pause or unpause during gameplay stages
  Enter              Start from the title screen
  Q or Esc           Quit"
    );
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        advance_game, clear_one_shots, merge_input, mouse_scene_position, parse_args,
        schedule_next_frame, step_input_without_one_shots, whole_steps,
    };
    use crate::{
        arcade::ORIGINAL_FRAME_TIME,
        game::{Game, UpdateInput},
        input::MousePosition,
        pacman::Direction,
        render::{RenderTargetSize, Renderer},
        vector::Vector2,
    };

    #[test]
    fn no_arguments_launch_the_default_game() {
        parse_args(std::iter::empty()).expect("argument parsing should succeed");
    }

    #[test]
    fn explicit_arguments_are_rejected() {
        let error = parse_args(std::iter::once(String::from("legacy-mode")))
            .expect_err("parsing should fail");
        assert!(
            error
                .to_string()
                .contains("This binary only supports the default game launch"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn extra_arguments_are_rejected() {
        let error = parse_args([String::from("legacy-mode"), String::from("extra")].into_iter())
            .expect_err("parsing should fail");
        assert!(
            error
                .to_string()
                .contains("This binary only supports the default game launch"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    /// Merges input keeps latest continuous state and accumulates one shots.
    fn merge_input_keeps_latest_continuous_state_and_accumulates_one_shots() {
        let mut pending = UpdateInput::default();
        merge_input(
            &mut pending,
            UpdateInput {
                requested_direction: Direction::Left,
                pause_requested: true,
                start_requested: false,
                mouse_position: Some(Vector2::new(1.0, 2.0)),
                mouse_click_position: None,
                typed_chars: vec!['x'],
            },
        );
        merge_input(
            &mut pending,
            UpdateInput {
                requested_direction: Direction::Up,
                pause_requested: false,
                start_requested: true,
                mouse_position: Some(Vector2::new(3.0, 4.0)),
                mouse_click_position: Some(Vector2::new(5.0, 6.0)),
                typed_chars: vec!['y'],
            },
        );

        assert_eq!(pending.requested_direction, Direction::Up);
        assert_eq!(pending.mouse_position, Some(Vector2::new(3.0, 4.0)));
        assert_eq!(pending.mouse_click_position, Some(Vector2::new(5.0, 6.0)));
        assert!(pending.pause_requested);
        assert!(pending.start_requested);
        assert_eq!(pending.typed_chars, vec!['x', 'y']);
    }

    #[test]
    fn later_fixed_steps_drop_one_shots_but_keep_continuous_inputs() {
        let pending = UpdateInput {
            requested_direction: Direction::Right,
            pause_requested: true,
            start_requested: true,
            mouse_position: Some(Vector2::new(7.0, 8.0)),
            mouse_click_position: Some(Vector2::new(9.0, 10.0)),
            typed_chars: vec!['z'],
        };

        let step = step_input_without_one_shots(&pending);

        assert_eq!(step.requested_direction, Direction::Right);
        assert_eq!(step.mouse_position, Some(Vector2::new(7.0, 8.0)));
        assert_eq!(step.mouse_click_position, None);
        assert!(step.typed_chars.is_empty());
        assert!(!step.pause_requested);
        assert!(!step.start_requested);
    }

    #[test]
    fn clearing_one_shots_preserves_continuous_inputs() {
        let mut pending = UpdateInput {
            requested_direction: Direction::Down,
            pause_requested: true,
            start_requested: true,
            mouse_position: Some(Vector2::new(1.0, 1.0)),
            mouse_click_position: Some(Vector2::new(2.0, 2.0)),
            typed_chars: vec!['q'],
        };

        clear_one_shots(&mut pending);

        assert_eq!(pending.requested_direction, Direction::Down);
        assert_eq!(pending.mouse_position, Some(Vector2::new(1.0, 1.0)));
        assert_eq!(pending.mouse_click_position, None);
        assert!(pending.typed_chars.is_empty());
        assert!(!pending.pause_requested);
        assert!(!pending.start_requested);
    }

    #[test]
    fn whole_steps_consumes_complete_frames_and_preserves_remainder() {
        let mut accumulator = ORIGINAL_FRAME_TIME * 2.5;

        let steps = whole_steps(&mut accumulator, ORIGINAL_FRAME_TIME);

        assert_eq!(steps, 2);
        assert!((accumulator - ORIGINAL_FRAME_TIME * 0.5).abs() < f32::EPSILON);
    }

    #[test]
    /// Advances game clears one shots after processing steps.
    fn advance_game_clears_one_shots_after_processing_steps() {
        let mut game = Game::new();
        let mut pending = UpdateInput {
            requested_direction: Direction::Left,
            pause_requested: true,
            start_requested: true,
            mouse_position: Some(Vector2::new(4.0, 5.0)),
            mouse_click_position: Some(Vector2::new(6.0, 7.0)),
            typed_chars: vec!['x'],
        };
        let mut accumulator = ORIGINAL_FRAME_TIME * 2.25;

        let quit = advance_game(
            &mut game,
            &mut pending,
            &mut accumulator,
            ORIGINAL_FRAME_TIME,
        );

        assert!(!quit);
        assert_eq!(pending.requested_direction, Direction::Left);
        assert_eq!(pending.mouse_position, Some(Vector2::new(4.0, 5.0)));
        assert!(!pending.pause_requested);
        assert!(!pending.start_requested);
        assert!(pending.mouse_click_position.is_none());
        assert!(pending.typed_chars.is_empty());
        assert!((accumulator - ORIGINAL_FRAME_TIME * 0.25).abs() < f32::EPSILON);
    }

    #[test]
    /// Translates scene position projects window pixels into scene coordinates.
    fn mouse_scene_position_projects_window_pixels_into_scene_coordinates() {
        let renderer = Renderer::new(RenderTargetSize::new(448, 576));

        let scene_position =
            mouse_scene_position(Some(MousePosition::new(216.0, 320.0)), &renderer);

        assert_eq!(scene_position, Some(Vector2::new(216.0, 320.0)));
    }

    #[test]
    /// Translates scene position returns none when there is no mouse position.
    fn mouse_scene_position_returns_none_without_a_mouse_position() {
        let renderer = Renderer::new(RenderTargetSize::new(448, 576));

        let scene_position = mouse_scene_position(None, &renderer);

        assert!(scene_position.is_none());
    }

    #[test]
    fn schedule_next_frame_moves_the_deadline_forward() {
        let mut next_frame = Instant::now()
            .checked_sub(Duration::from_millis(5))
            .expect("instant subtraction should succeed");
        let frame_started = Instant::now();

        schedule_next_frame(&mut next_frame, frame_started, Duration::from_millis(16));

        assert!(next_frame > frame_started);
    }
}
