//! Starts the Pac-Man application and reports top-level failures.

/// Handles main.
fn main() {
    // Branch based on the current runtime condition.
    if let Err(err) = pacman::app::run() {
        eprintln!("pacman: {err:#}");
        std::process::exit(1);
    }
}
