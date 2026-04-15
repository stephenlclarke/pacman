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
    input::InputController,
    kitty::KittyGraphics,
    render::Renderer,
    terminal::{TerminalSession, geometry},
};

const MAX_DT: f32 = 0.1;

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
    for event in game.drain_events() {
        audio.handle_event(event);
    }
    let frame_time = ORIGINAL_FRAME_TIME;
    let frame_duration = Duration::from_secs_f32(frame_time);
    let mut pending_input = UpdateInput::default();
    let mut accumulator = 0.0f32;
    let mut last_tick = Instant::now();

    loop {
        let frame_started = Instant::now();
        let latest_geometry = geometry()?;
        if latest_geometry != terminal_geometry {
            terminal_geometry = latest_geometry;
            renderer.resize(terminal_geometry);
            graphics.resize(terminal_geometry.cols, terminal_geometry.rows);
        }

        input.poll()?;
        if input.quit_requested() {
            break;
        }

        let dt = last_tick.elapsed().as_secs_f32().min(MAX_DT);
        last_tick = Instant::now();
        accumulator = (accumulator + dt).min(frame_time * 8.0);

        merge_input(
            &mut pending_input,
            UpdateInput {
                requested_direction: input.direction(),
                pause_requested: input.take_pause_requested(),
                start_requested: input.take_start_requested(),
                mouse_position: input.mouse_cell().and_then(|mouse_cell| {
                    renderer.scene_position_for_terminal_cell(
                        terminal_geometry,
                        mouse_cell.column(),
                        mouse_cell.row(),
                    )
                }),
                mouse_click_position: input.take_mouse_click().and_then(|mouse_cell| {
                    renderer.scene_position_for_terminal_cell(
                        terminal_geometry,
                        mouse_cell.column(),
                        mouse_cell.row(),
                    )
                }),
                typed_chars: input.take_typed_chars(),
            },
        );

        let mut first_step = true;
        while accumulator >= frame_time {
            let step_input = if first_step {
                pending_input.clone()
            } else {
                step_input_without_one_shots(&pending_input)
            };
            game.update_with_input(frame_time, step_input);
            accumulator -= frame_time;
            first_step = false;
            if game.quit_requested() {
                break;
            }
        }
        if !first_step {
            clear_one_shots(&mut pending_input);
        }
        if game.quit_requested() {
            break;
        }

        for event in game.drain_events() {
            audio.handle_event(event);
        }
        let frame = game.frame();
        let image = renderer.render(&frame);
        graphics.draw_frame(&mut stdout, image)?;
        stdout.flush()?;

        let elapsed = frame_started.elapsed();
        if elapsed < frame_duration {
            input.poll_for(frame_duration - elapsed)?;
            if input.quit_requested() {
                break;
            }
        }
    }

    graphics.clear(&mut stdout)?;
    stdout.flush()?;

    Ok(())
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
    use super::{clear_one_shots, merge_input, parse_args, step_input_without_one_shots};
    use crate::{game::UpdateInput, pacman::Direction, vector::Vector2};

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
}
