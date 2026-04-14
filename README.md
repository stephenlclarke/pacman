# pacman

Rust reimplementation of the `Start`, `Level 1`, and `Level 2` sections from
[pacmancode.com](https://pacmancode.com), using Kitty graphics instead of
Pygame.

Implemented scope:

- `vector.py` equivalent in [src/vector.rs](src/vector.rs)
- optional `stack.py` equivalent in [src/stack.rs](src/stack.rs)
- blank screen stage via `cargo run -- blank-screen`
- basic movement stage via `cargo run --` or `cargo run -- basic-movement`
- Level 1 `nodes` via `cargo run -- nodes`
- Level 1 `node-movement-1` via `cargo run -- node-movement-1`
- Level 1 `node-movement-2` via `cargo run -- node-movement-2`
- Level 1 `node-movement-3` via `cargo run -- node-movement-3`
- `cargo run -- level1` as an alias for the final Level 1 state
- Level 2 `maze-basics` via `cargo run -- maze-basics`
- Level 2 `pacman-maze` via `cargo run -- pacman-maze`
- Level 2 `portals` via `cargo run -- portals`
- Level 2 `pellets` via `cargo run -- pellets`
- Level 2 `eating-pellets` via `cargo run -- eating-pellets`
- `cargo run -- level2` as an alias for the final Level 2 state

Run this inside `kitty`, `ghostty`, or another terminal that supports the
Kitty graphics protocol.

Controls:

- Arrow keys or `WASD` to move
- `Q` or `Esc` to quit
