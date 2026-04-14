# PacMan

![PacMan](docs/pacman.png)

This is a rust implmentation of [pacmancode.com](https://pacmancode.com),
rendered with Kitty graphics.

Run targets:

- `cargo run`

Run this inside `kitty`, `ghostty`, `warp` or another terminal that supports the
Kitty graphics protocol.

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
