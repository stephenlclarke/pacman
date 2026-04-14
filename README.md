# pacman

Rust reimplementation of the `Start` tab from [pacmancode.com](https://pacmancode.com),
using Kitty graphics instead of Pygame.

Implemented scope:

- `vector.py` equivalent in [src/vector.rs](src/vector.rs)
- optional `stack.py` equivalent in [src/stack.rs](src/stack.rs)
- blank screen stage via `cargo run -- blank-screen`
- basic movement stage via `cargo run --` or `cargo run -- basic-movement`

Run this inside `kitty`, `ghostty`, or another terminal that supports the
Kitty graphics protocol.

Controls:

- Arrow keys or `WASD` to move
- `Q` or `Esc` to quit
