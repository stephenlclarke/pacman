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
    game::{Game, Stage},
    input::InputController,
    kitty::KittyGraphics,
    render::Renderer,
    terminal::{TerminalSession, geometry},
};

const FRAME_TIME: Duration = Duration::from_millis(33);
const MAX_DT: f32 = 0.1;

pub fn run() -> Result<()> {
    let stage = parse_stage(std::env::args().skip(1))?;

    KittyGraphics::ensure_supported()?;

    let mut stdout = stdout();
    let _session = TerminalSession::enter(&mut stdout)?;
    queue!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
    stdout.flush()?;

    let mut terminal_geometry = geometry()?;
    let mut renderer = Renderer::new(terminal_geometry);
    let mut graphics = KittyGraphics::new(terminal_geometry.cols, terminal_geometry.rows);
    let mut input = InputController::default();
    let mut game = Game::new(stage);
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
        game.update(dt, input.direction(), pause_requested);
        let frame = game.frame();
        let image = renderer.render(&frame);
        graphics.draw_frame(&mut stdout, &image)?;
        stdout.flush()?;

        let elapsed = frame_started.elapsed();
        if elapsed < FRAME_TIME {
            std::thread::sleep(FRAME_TIME - elapsed);
        }
    }

    graphics.clear(&mut stdout)?;
    stdout.flush()?;

    Ok(())
}

fn parse_stage(args: impl Iterator<Item = String>) -> Result<Stage> {
    let args: Vec<String> = args.collect();
    if args.is_empty() {
        return Ok(Stage::BasicMovement);
    }

    match args[0].as_str() {
        "blank-screen" => Ok(Stage::BlankScreen),
        "basic-movement" => Ok(Stage::BasicMovement),
        "nodes" => Ok(Stage::Nodes),
        "node-movement-1" => Ok(Stage::NodeMovement1),
        "node-movement-2" => Ok(Stage::NodeMovement2),
        "node-movement-3" | "level1" => Ok(Stage::NodeMovement3),
        "maze-basics" => Ok(Stage::MazeBasics),
        "pacman-maze" => Ok(Stage::PacmanMaze),
        "portals" => Ok(Stage::Portals),
        "pellets" => Ok(Stage::Pellets),
        "eating-pellets" | "level2" => Ok(Stage::EatingPellets),
        "spawn-mode" | "level3" => Ok(Stage::Level3),
        "node-restrictions" | "level4" => Ok(Stage::Level4),
        "animate-ghosts" | "level5" => Ok(Stage::Level5),
        "-h" | "--help" => {
            print_help();
            std::process::exit(0);
        }
        other => {
            bail!(
                "unknown mode {other:?}. Use `blank-screen`, `basic-movement`, `nodes`, \
                 `node-movement-1`, `node-movement-2`, `node-movement-3`, `level1`, \
                 `maze-basics`, `pacman-maze`, `portals`, `pellets`, `eating-pellets`, \
                 `level2`, `spawn-mode`, `level3`, `node-restrictions`, `level4`, or \
                 `animate-ghosts`, `level5`, or `--help`."
            )
        }
    }
}

fn print_help() {
    println!(
        "Usage: cargo run -- [blank-screen|basic-movement]

Modes:
  blank-screen    Render the Start-tab blank screen stage.
  basic-movement  Render the Start-tab basic movement stage.
  nodes           Render the Level 1 Nodes stage.
  node-movement-1 Render Level 1 Node Movement part 1.
  node-movement-2 Render Level 1 Node Movement part 2.
  node-movement-3 Render Level 1 Node Movement part 3.
  level1          Alias for `node-movement-3`.
  maze-basics     Render the Level 2 Maze Basics stage.
  pacman-maze     Render the Level 2 Pacman Maze stage.
  portals         Render the Level 2 Portals stage.
  pellets         Render the Level 2 Pellets stage.
  eating-pellets  Render the Level 2 Eating Pellets stage.
  level2          Alias for `eating-pellets`.
  spawn-mode      Render the final Level 3 Spawn Mode stage.
  level3          Alias for `spawn-mode`.
  node-restrictions Render the final Level 4 Node Restrictions stage.
  level4          Alias for `node-restrictions`.
  animate-ghosts  Render the final Level 5 Animate Ghosts stage.
  level5          Alias for `animate-ghosts`.

Controls:
  Arrow keys / WASD  Move Pacman
  Space              Pause or unpause during Level 4
  Q or Esc           Quit"
    );
}

#[cfg(test)]
mod tests {
    use super::{Stage, parse_stage};

    #[test]
    fn default_stage_is_basic_movement() {
        let stage = parse_stage(std::iter::empty()).expect("stage parsing should succeed");
        assert_eq!(stage, Stage::BasicMovement);
    }

    #[test]
    fn blank_screen_stage_parses() {
        let stage =
            parse_stage(std::iter::once(String::from("blank-screen"))).expect("stage parsing");
        assert_eq!(stage, Stage::BlankScreen);
    }

    #[test]
    fn level1_alias_maps_to_node_movement_part_three() {
        let stage = parse_stage(std::iter::once(String::from("level1"))).expect("stage parsing");
        assert_eq!(stage, Stage::NodeMovement3);
    }

    #[test]
    fn level2_alias_maps_to_eating_pellets() {
        let stage = parse_stage(std::iter::once(String::from("level2"))).expect("stage parsing");
        assert_eq!(stage, Stage::EatingPellets);
    }

    #[test]
    fn level3_alias_maps_to_spawn_mode() {
        let stage = parse_stage(std::iter::once(String::from("level3"))).expect("stage parsing");
        assert_eq!(stage, Stage::Level3);
    }

    #[test]
    fn level4_alias_maps_to_node_restrictions() {
        let stage = parse_stage(std::iter::once(String::from("level4"))).expect("stage parsing");
        assert_eq!(stage, Stage::Level4);
    }

    #[test]
    fn level5_alias_maps_to_animate_ghosts() {
        let stage = parse_stage(std::iter::once(String::from("level5"))).expect("stage parsing");
        assert_eq!(stage, Stage::Level5);
    }
}
