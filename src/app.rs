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
    audio::AudioManager,
    game::{Game, UpdateInput},
    input::InputController,
    kitty::KittyGraphics,
    render::Renderer,
    terminal::{TerminalSession, geometry},
};

const FRAME_TIME: Duration = Duration::from_millis(33);
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

        let pause_requested = input.take_pause_requested();
        let start_requested = input.take_start_requested();
        let typed_chars = input.take_typed_chars();
        let mouse_position = input.mouse_cell().and_then(|mouse_cell| {
            renderer.scene_position_for_terminal_cell(
                terminal_geometry,
                mouse_cell.column(),
                mouse_cell.row(),
            )
        });
        let mouse_click_position = input.take_mouse_click().and_then(|mouse_cell| {
            renderer.scene_position_for_terminal_cell(
                terminal_geometry,
                mouse_cell.column(),
                mouse_cell.row(),
            )
        });
        game.update_with_input(
            dt,
            UpdateInput {
                requested_direction: input.direction(),
                pause_requested,
                start_requested,
                mouse_position,
                mouse_click_position,
                typed_chars,
            },
        );
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
        if elapsed < FRAME_TIME {
            input.poll_for(FRAME_TIME - elapsed)?;
            if input.quit_requested() {
                break;
            }
        }
    }

    graphics.clear(&mut stdout)?;
    stdout.flush()?;

    Ok(())
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
                "unexpected launch mode {arg:?}. This branch only supports the final target. \
                 Use `cargo run` or `cargo run -- --help`."
            )
        }
        _ => bail!(
            "unexpected arguments. This branch only supports the final target. \
             Use `cargo run` or `cargo run -- --help`."
        ),
    }
}

fn print_help() {
    println!(
        "Usage: cargo run [-- --help]

Running `cargo run` launches the final Level 7 target.

This branch no longer exposes per-lesson launch modes.

Controls:
  Arrow keys / WASD  Move Pacman
  Space              Pause or unpause during gameplay stages
  Enter              Start from the Level 7 title screen
  Q or Esc           Quit"
    );
}

#[cfg(test)]
mod tests {
    use super::parse_args;

    #[test]
    fn no_arguments_launch_the_default_game() {
        parse_args(std::iter::empty()).expect("argument parsing should succeed");
    }

    #[test]
    fn explicit_launch_modes_are_rejected() {
        let error =
            parse_args(std::iter::once(String::from("level7"))).expect_err("parsing should fail");
        assert!(
            error
                .to_string()
                .contains("This branch only supports the final target"),
            "unexpected error: {error:#}"
        );
    }

    #[test]
    fn extra_arguments_are_rejected() {
        let error = parse_args([String::from("level7"), String::from("extra")].into_iter())
            .expect_err("parsing should fail");
        assert!(
            error
                .to_string()
                .contains("This branch only supports the final target"),
            "unexpected error: {error:#}"
        );
    }
}
