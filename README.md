# PacMan

This is a rust implmentation of Pacman rendered with Kitty graphics.

![PacMan](docs/pacman.png)

![Start Sequence](docs/start-sequence.gif)

Run targets:

- `cargo run`

Run this inside `kitty`, `ghostty`, `warp` or another terminal that supports the
Kitty graphics protocol.

## Install

Install directly from git with Cargo:

- `cargo install --git https://github.com/stephenlclarke/pacman pacman`

`cargo install` builds with Cargo's release profile by default. Do not pass
`--debug` unless you explicitly want a slower debug build.

After installation, run the game with:

- `pacman`

Notes:

- Run it inside `kitty`, `ghostty`, `warp`, or another terminal that supports
  the Kitty graphics protocol.
- Download Ghostty: <https://ghostty.org/download>
- Download Warp: <https://www.warp.dev/download>
- If `pacman` is not found after installation, ensure `~/.cargo/bin` is on your
  `PATH`.

## XYZZY Mode

After starting the game, type `x`, `y`, `z`, `z`, `y` to toggle `xyzzy` mode on
or off. Pacman blinks three times when the mode changes.

Extra keys while `xyzzy` mode is active:

- `a`: toggle autopilot. This will route Pacman around the maze to clear pellets,
  pick up fruit, chase frightened ghosts when it is worthwhile, and delay power
  pellets until they are useful. Autopilot turns off automatically when the
  level has no pellets left.
- `f`: toggle forced freight mode for the ghosts.
- `t`: teleport Pacman to the safest valid node on the map.
- `r`: reset all ghosts back to their starting positions.

## ROM References

These online references have been useful while translating the Midway Pac-Man
ROMs into native Rust rather than emulating the Z80 code directly:

- [pacmancode.com](https://pacmancode.com): original lesson sequence this repo
  started from before the arcade-ROM translation work.
- [Midway Pacman ROMS](https://www.retrostic.com/roms/mame/pac-man-40808):
  Original Midway Arcade Pacman ROMS
- [Pacman hardware](https://www.walkofmind.com/programming/pie/hardware.htm):
  CPU/video memory map, palette PROM layout, sprite registers, and screen
  rotation details.
- [Pacman character definitions](https://walkofmind.com/programming/pie/char_defs.htm):
  character ROM byte layout and rotated tile decoding details.
- [Characters, sprites and colours](https://pacmanc.blogspot.com/2024/05/characters-sprites-and-colours.html):
  practical notes on character ranges, sprite ranges, maze wall characters,
  tunnel and ghost-house color markers, and fruit/icon character tables.
- [Pac-Man Emulation Guide](https://www.lomont.org/software/games/pacman/PacmanEmulation.pdf):
  hardware-oriented reference for palettes, video layout, sprite ordering, and
  general ROM structure.
