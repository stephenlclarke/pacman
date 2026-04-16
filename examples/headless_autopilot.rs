//! Runs the headless autopilot simulator for regression checks and performance sampling.

use pacman::game::{HeadlessAutopilotStopReason, run_headless_autopilot};
use std::io::Write;

/// Parses arg.
fn parse_arg(args: &[String], index: usize, default: u64) -> u64 {
    args.get(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

/// Handles main.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let start_seed = parse_arg(&args, 1, 0);
    let runs = parse_arg(&args, 2, 10) as usize;
    let max_steps = parse_arg(&args, 3, 500_000) as usize;

    let mut total_levels = 0u64;
    let mut total_pellets = 0u64;
    let mut total_ghosts = 0u64;
    let mut total_fruit = 0u64;
    let mut deaths = 0u64;

    // Iterate through each item in the current collection or range.
    for offset in 0..runs {
        let seed = start_seed + offset as u64;
        let report = run_headless_autopilot(seed, max_steps);
        total_levels += report.levels_cleared as u64;
        total_pellets += report.pellets_eaten as u64;
        total_ghosts += report.ghosts_eaten as u64;
        total_fruit += report.fruit_eaten as u64;
        // Branch based on the current runtime condition.
        if report.stop_reason == HeadlessAutopilotStopReason::PacmanDied {
            deaths += 1;
        }

        println!(
            "seed={seed} stop={:?} levels_cleared={} level_reached={} pellets={} ghosts={} fruit={} score={} steps={}",
            report.stop_reason,
            report.levels_cleared,
            report.level_reached,
            report.pellets_eaten,
            report.ghosts_eaten,
            report.fruit_eaten,
            report.score,
            report.steps,
        );
        // Branch based on the current runtime condition.
        if let Some(snapshot) = &report.death_snapshot {
            println!(
                "death level={} lives={} pellets_remaining={} pacman=({:.1},{:.1}) dir={:?}",
                snapshot.level,
                snapshot.lives,
                snapshot.pellets_remaining,
                snapshot.pacman_position.0,
                snapshot.pacman_position.1,
                snapshot.pacman_direction,
            );
            println!("ghosts={:?}", snapshot.ghosts);
            let sample = snapshot
                .remaining_pellets
                .iter()
                .take(12)
                .copied()
                .collect::<Vec<_>>();
            println!("remaining_pellets_sample={sample:?}");
        }
        let _ = std::io::stdout().flush();
    }

    println!(
        "summary runs={} deaths={} avg_levels_cleared={:.2} avg_pellets={:.2} avg_ghosts={:.2} avg_fruit={:.2}",
        runs,
        deaths,
        total_levels as f64 / runs as f64,
        total_pellets as f64 / runs as f64,
        total_ghosts as f64 / runs as f64,
        total_fruit as f64 / runs as f64,
    );
}
