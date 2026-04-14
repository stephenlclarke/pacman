fn main() {
    if let Err(err) = pacman::app::run() {
        eprintln!("pacman: {err:#}");
        std::process::exit(1);
    }
}
