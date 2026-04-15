use std::{fs::File, path::PathBuf};

use anyhow::{Context, Result};
use gif::{Encoder, Frame, Repeat};
use pacman::{
    constants::{SCREEN_HEIGHT, SCREEN_WIDTH},
    game::{Game, UpdateInput},
    pacman::Direction,
    render::Renderer,
    terminal::TerminalGeometry,
};

const FRAME_DT: f32 = 0.25;
const FRAME_DELAY_CS: u16 = 25;
const ATTRACT_CYCLE_SECONDS: f32 = 6.0 + 8.5 + 7.5;
const RETURN_TO_TITLE_SECONDS: f32 = 1.0;
const READY_SECONDS: f32 = 1.5;

fn main() -> Result<()> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("docs/start-sequence.gif"));

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating parent directory for {}", output.display()))?;
    }

    let geometry = TerminalGeometry {
        cols: 80,
        rows: 36,
        pixel_width: SCREEN_WIDTH as u16,
        pixel_height: SCREEN_HEIGHT as u16,
    };
    let mut renderer = Renderer::new(geometry);
    let mut game = Game::new();
    let _ = game.drain_events();

    let file = File::create(&output)
        .with_context(|| format!("creating gif output {}", output.display()))?;
    let mut encoder = Encoder::new(file, SCREEN_WIDTH as u16, SCREEN_HEIGHT as u16, &[])
        .with_context(|| format!("creating gif encoder for {}", output.display()))?;
    encoder
        .set_repeat(Repeat::Infinite)
        .context("setting gif repeat mode")?;

    capture_frame(&mut encoder, &mut renderer, &game, 40)?;

    advance_for(
        &mut encoder,
        &mut renderer,
        &mut game,
        ATTRACT_CYCLE_SECONDS,
        UpdateInput::default(),
    )?;
    advance_for(
        &mut encoder,
        &mut renderer,
        &mut game,
        RETURN_TO_TITLE_SECONDS,
        UpdateInput::default(),
    )?;

    game.update_with_input(
        FRAME_DT,
        UpdateInput {
            requested_direction: Direction::Stop,
            start_requested: true,
            ..UpdateInput::default()
        },
    );
    capture_frame(&mut encoder, &mut renderer, &game, FRAME_DELAY_CS)?;

    advance_for(
        &mut encoder,
        &mut renderer,
        &mut game,
        READY_SECONDS,
        UpdateInput::default(),
    )?;

    println!("wrote {}", output.display());
    Ok(())
}

fn advance_for(
    encoder: &mut Encoder<File>,
    renderer: &mut Renderer,
    game: &mut Game,
    duration: f32,
    input: UpdateInput,
) -> Result<()> {
    let mut remaining = duration;
    while remaining > 0.0 {
        game.update_with_input(FRAME_DT, input.clone());
        capture_frame(encoder, renderer, game, FRAME_DELAY_CS)?;
        remaining -= FRAME_DT;
    }
    Ok(())
}

fn capture_frame(
    encoder: &mut Encoder<File>,
    renderer: &mut Renderer,
    game: &Game,
    delay_cs: u16,
) -> Result<()> {
    let frame = game.frame();
    let image = renderer.render(&frame);
    let mut pixels = image.pixels.clone();
    let mut gif_frame =
        Frame::from_rgba_speed(image.width as u16, image.height as u16, &mut pixels, 10);
    gif_frame.delay = delay_cs;
    encoder
        .write_frame(&gif_frame)
        .context("writing gif frame")?;
    Ok(())
}
