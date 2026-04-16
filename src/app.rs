//! Runs the terminal application loop, input polling, fixed-timestep updates, and final frame presentation.

use std::{
    io::{Write, stdout},
    time::{Duration, Instant},
};

use anyhow::{Result, bail};
use crossterm::{
    cursor::MoveTo,
    queue,
    terminal::{Clear, ClearType},
};

use crate::{
    arcade::ORIGINAL_FRAME_TIME,
    audio::AudioManager,
    game::{Game, UpdateInput},
    input::{InputController, MouseCell},
    kitty::KittyGraphics,
    render::Renderer,
    terminal::{TerminalSession, geometry},
};

const MAX_DT: f32 = 0.1;

/// Runs run.
pub fn run() -> Result<()> {
    parse_args(std::env::args().skip(1))?;

    KittyGraphics::ensure_supported()?;

    let mut stdout = stdout();
    let _session = TerminalSession::enter(&mut stdout)?;
    queue!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    stdout.flush()?;

    let mut terminal_geometry = geometry()?;
    let mut renderer = Renderer::new(terminal_geometry);
    let mut graphics = KittyGraphics::new(terminal_geometry.cols, terminal_geometry.rows);
    let mut input = InputController::default();
    let mut game = Game::new();
    let mut audio = AudioManager::new();
    // Iterate through each item in the current collection or range.
    for event in game.drain_events() {
        audio.handle_event(event);
    }
    let frame_time = ORIGINAL_FRAME_TIME;
    let frame_duration = Duration::from_secs_f32(frame_time);
    let mut pending_input = UpdateInput::default();
    let mut accumulator = 0.0f32;
    let mut last_tick = Instant::now();

    // Keep looping until a break condition exits the block.
    loop {
        let frame_started = Instant::now();
        sync_terminal_geometry(&mut terminal_geometry, &mut renderer, &mut graphics)?;

        // Branch based on the current runtime condition.
        if poll_input(&mut input)? {
            break;
        }

        let dt = last_tick.elapsed().as_secs_f32().min(MAX_DT);
        last_tick = Instant::now();
        accumulator = (accumulator + dt).min(frame_time * 8.0);

        merge_input(
            &mut pending_input,
            collect_update_input(&mut input, &renderer, terminal_geometry),
        );

        // Branch based on the current runtime condition.
        if advance_game(&mut game, &mut pending_input, &mut accumulator, frame_time) {
            break;
        }

        render_frame(
            &mut game,
            &mut audio,
            &mut renderer,
            &mut graphics,
            &mut stdout,
        )?;

        // Branch based on the current runtime condition.
        if wait_for_next_frame(&mut input, frame_started, frame_duration)? {
            break;
        }
    }

    graphics.clear(&mut stdout)?;
    stdout.flush()?;

    Ok(())
}

/// Synchronizes terminal geometry.
fn sync_terminal_geometry(
    terminal_geometry: &mut crate::terminal::TerminalGeometry,
    renderer: &mut Renderer,
    graphics: &mut KittyGraphics,
) -> Result<()> {
    let latest_geometry = geometry()?;
    // Branch based on the current runtime condition.
    if latest_geometry != *terminal_geometry {
        *terminal_geometry = latest_geometry;
        renderer.resize(*terminal_geometry);
        graphics.resize(terminal_geometry.cols, terminal_geometry.rows);
    }
    Ok(())
}

/// Polls input.
fn poll_input(input: &mut InputController) -> Result<bool> {
    input.poll()?;
    Ok(input.quit_requested())
}

/// Collects update input.
fn collect_update_input(
    input: &mut InputController,
    renderer: &Renderer,
    terminal_geometry: crate::terminal::TerminalGeometry,
) -> UpdateInput {
    UpdateInput {
        requested_direction: input.direction(),
        pause_requested: input.take_pause_requested(),
        start_requested: input.take_start_requested(),
        mouse_position: mouse_scene_position(input.mouse_cell(), renderer, terminal_geometry),
        mouse_click_position: mouse_scene_position(
            input.take_mouse_click(),
            renderer,
            terminal_geometry,
        ),
        typed_chars: input.take_typed_chars(),
    }
}

/// Translates scene position.
fn mouse_scene_position(
    mouse_cell: Option<MouseCell>,
    renderer: &Renderer,
    terminal_geometry: crate::terminal::TerminalGeometry,
) -> Option<crate::vector::Vector2> {
    mouse_cell.and_then(|mouse_cell| {
        renderer.scene_position_for_terminal_cell(
            terminal_geometry,
            mouse_cell.column(),
            mouse_cell.row(),
        )
    })
}

/// Advances game.
fn advance_game(
    game: &mut Game,
    pending_input: &mut UpdateInput,
    accumulator: &mut f32,
    frame_time: f32,
) -> bool {
    let steps = whole_steps(accumulator, frame_time);
    // Iterate through each item in the current collection or range.
    for step_index in 0..steps {
        let step_input = if step_index == 0 {
            pending_input.clone()
        } else {
            step_input_without_one_shots(pending_input)
        };
        game.update_with_input(frame_time, step_input);
        // Branch based on the current runtime condition.
        if game.quit_requested() {
            return true;
        }
    }
    // Branch based on the current runtime condition.
    if steps > 0 {
        clear_one_shots(pending_input);
    }
    game.quit_requested()
}

/// Handles steps.
fn whole_steps(accumulator: &mut f32, frame_time: f32) -> usize {
    let steps = (*accumulator / frame_time) as usize;
    *accumulator -= steps as f32 * frame_time;
    steps
}

/// Renders frame.
fn render_frame(
    game: &mut Game,
    audio: &mut AudioManager,
    renderer: &mut Renderer,
    graphics: &mut KittyGraphics,
    stdout: &mut std::io::Stdout,
) -> Result<()> {
    // Iterate through each item in the current collection or range.
    for event in game.drain_events() {
        audio.handle_event(event);
    }
    let frame = game.frame();
    let image = renderer.render(&frame);
    graphics.draw_frame(stdout, image)?;
    stdout.flush()?;
    Ok(())
}

/// Waits for for next frame.
fn wait_for_next_frame(
    input: &mut InputController,
    frame_started: Instant,
    frame_duration: Duration,
) -> Result<bool> {
    let elapsed = frame_started.elapsed();
    // Branch based on the current runtime condition.
    if elapsed < frame_duration {
        input.poll_for(frame_duration - elapsed)?;
    }
    Ok(input.quit_requested())
}

/// Merges input.
fn merge_input(pending: &mut UpdateInput, next: UpdateInput) {
    pending.requested_direction = next.requested_direction;
    pending.mouse_position = next.mouse_position;
    pending.pause_requested |= next.pause_requested;
    pending.start_requested |= next.start_requested;
    // Branch based on the current runtime condition.
    if next.mouse_click_position.is_some() {
        pending.mouse_click_position = next.mouse_click_position;
    }
    pending.typed_chars.extend(next.typed_chars);
}

/// Clears one shots.
fn clear_one_shots(pending: &mut UpdateInput) {
    pending.pause_requested = false;
    pending.start_requested = false;
    pending.mouse_click_position = None;
    pending.typed_chars.clear();
}

/// Handles input without one shots.
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

/// Parses args.
fn parse_args(args: impl Iterator<Item = String>) -> Result<()> {
    let args: Vec<String> = args.collect();
    // Branch based on the current runtime condition.
    if args.is_empty() {
        return Ok(());
    }

    // Select the next behavior based on the current state.
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

Running `cargo run` launches the game.

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
        step_input_without_one_shots, wait_for_next_frame, whole_steps,
    };
    use crate::{
        arcade::ORIGINAL_FRAME_TIME,
        game::{Game, UpdateInput},
        input::InputController,
        pacman::Direction,
        render::Renderer,
        terminal::TerminalGeometry,
        vector::Vector2,
    };

    #[test]
    /// Handles arguments launch the default game.
    fn no_arguments_launch_the_default_game() {
        parse_args(std::iter::empty()).expect("argument parsing should succeed");
    }

    #[test]
    /// Handles arguments are rejected.
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
    /// Handles arguments are rejected.
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
    /// Handles fixed steps drop one shots but keep continuous inputs.
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
    /// Handles one shots preserves continuous inputs.
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
    /// Handles steps consumes complete frames and preserves remainder.
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
    /// Translates scene position projects terminal cells into scene coordinates.
    fn mouse_scene_position_projects_terminal_cells_into_scene_coordinates() {
        let geometry = TerminalGeometry {
            cols: 80,
            rows: 30,
            pixel_width: 0,
            pixel_height: 0,
        };
        let renderer = Renderer::new(geometry);

        let scene_position = mouse_scene_position(
            Some(crate::input::MouseCell::default()),
            &renderer,
            geometry,
        );

        assert!(scene_position.is_some());
    }

    #[test]
    /// Translates scene position returns none for empty terminal geometry.
    fn mouse_scene_position_returns_none_for_empty_terminal_geometry() {
        let geometry = TerminalGeometry {
            cols: 0,
            rows: 0,
            pixel_width: 0,
            pixel_height: 0,
        };
        let renderer = Renderer::new(geometry);

        let scene_position = mouse_scene_position(
            Some(crate::input::MouseCell::default()),
            &renderer,
            geometry,
        );

        assert!(scene_position.is_none());
    }

    #[test]
    /// Waits for for next frame returns current quit state when frame is elapsed.
    fn wait_for_next_frame_returns_current_quit_state_when_frame_is_elapsed() {
        let mut input = InputController::default();
        let frame_started = Instant::now()
            .checked_sub(Duration::from_millis(5))
            .expect("instant subtraction should succeed");

        let quit = wait_for_next_frame(&mut input, frame_started, Duration::ZERO)
            .expect("waiting should succeed");

        assert!(!quit);
    }
}
