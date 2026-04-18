# PacMan

[![Quality Gate Status](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_pacman&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_pacman)
[![Bugs](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_pacman&metric=bugs)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_pacman)
[![Code Smells](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_pacman&metric=code_smells)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_pacman)
[![Coverage](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_pacman&metric=coverage)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_pacman)
[![Duplicated Lines (%)](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_pacman&metric=duplicated_lines_density)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_pacman)
[![Lines of Code](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_pacman&metric=ncloc)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_pacman)
[![Reliability Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_pacman&metric=reliability_rating)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_pacman)
[![Security Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_pacman&metric=security_rating)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_pacman)
[![Technical Debt](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_pacman&metric=sqale_index)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_pacman)
[![Maintainability Rating](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_pacman&metric=sqale_rating)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_pacman)
[![Vulnerabilities](https://sonarcloud.io/api/project_badges/measure?project=stephenlclarke_pacman&metric=vulnerabilities)](https://sonarcloud.io/summary/new_code?id=stephenlclarke_pacman)
![Repo Visitors](https://visitor-badge.laobi.icu/badge?page_id=stephenlclarke.pacman)

---

This is a rust implmentation of Pacman rendered with Kitty graphics.

![PacMan](docs/pacman.png)

<!-- markdownlint-disable MD033 -->
<p align="center">
  <img src="docs/start-sequence.gif" alt="Start Sequence" />
</p>
<!-- markdownlint-enable MD033 -->

Run targets:

- `cargo run`
- `make run`
- `cargo test`
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `make ci`
- `make coverage`
- `make sq-ci`
- `make sq`
- `cargo run --example generate_start_sequence_gif`
- `cargo run --example headless_autopilot`

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
- The top score now persists between runs in `~/.xyzzy/pacman/high_scores.txt`;
  set `PACMAN_DATA_DIR` to redirect that file for local experiments or tests.
- If `pacman` is not found after installation, ensure `~/.cargo/bin` is on your
  `PATH`.

## SonarQube

- `make sq-ci` generates the Cobertura coverage report used by the SonarCloud
  workflow in CI.
- `make sq` runs the same coverage step locally and then invokes
  `sonar-scanner`.
- Local SonarQube scans require `cargo-llvm-cov`, `sonar-scanner`, and a
  `SONAR_TOKEN` environment variable.

## XYZZY Mode

After starting the game, type `X`, `Y`, `Z`, `Z`, `Y` to toggle `XYZZY` mode on
or off. Pacman blinks three times when the mode changes.

Letter-key controls accept either upper- or lower-case input.

Extra keys while `xyzzy` mode is active:

- `A`: toggle autopilot. This will route Pacman around the maze to clear pellets,
  pick up fruit, chase frightened ghosts when it is worthwhile, and delay power
  pellets until they are useful. Autopilot turns off automatically when the
  level has no pellets left.
- `F`: toggle forced freight mode for the ghosts.
- `R`: reset all ghosts back to their starting positions.
- `T`: teleport Pacman to the safest valid node on the map.

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

## Customisation

Arcade defaults ship in `assets/arcade/`. Copy the files you want to change
into `~/.xyzzy/pacman/` to override them locally. Key/value files are merged on
top of the embedded defaults, so you only need to include the keys you want to
change. Layout files are replaced whole. If `PACMAN_DATA_DIR` is set, the game
reads these files from that directory instead.

### `arcade-rules.txt`

`difficulty_entries`
Default:

```text
3,1,1,0,2,0;4,1,2,1,3,0;4,1,3,2,4,1;4,2,3,2,5,1;5,0,3,2,6,2;
5,1,3,3,3,2;5,2,3,3,6,2;5,2,3,3,6,2;5,0,3,4,7,2;5,1,3,4,3,2;
5,2,3,4,6,2;5,2,3,5,7,2;5,0,3,5,7,2;5,2,3,5,5,2;5,1,3,6,7,2;
5,2,3,6,7,2;5,2,3,6,8,2;5,2,3,6,7,2;5,2,3,7,8,2;5,2,3,7,8,2;
6,2,3,7,8,2
```

Meaning: per-level lookup table selecting movement, phase, dot-release,
frightened-time, and timer groups.

`personal_dot_groups`
Default: `20,30,70;0,30,60;0,0,50;0,0,0`
Meaning: Pinky, Inky, and Clyde personal dot-release thresholds by group.

`elroy_dot_groups`
Default: `20,10;30,15;40,20;50,25;60,30;80,40;100,50;120,60`
Meaning: Blinky’s Cruise Elroy pellet thresholds by group.

`frightened_timer_ticks`
Default: `960,840,720,600,480,360,240,120,1`
Meaning: frightened-mode durations in original 120 Hz timer ticks.

`release_timer_frames`
Default: `240,240,180`
Meaning: ghost-house inactivity release timers in video frames.

`move_pattern_groups`
Default:

```text
0x552a552a,0x55555555,0x552a552a,0x524aa594,0x25252525,0x22222222,0x01010101;
0x524aa594,0xaa2a5555,0x552a552a,0x524aa594,0x92242549,0x48242291,0x01010101;
0x552a552a,0x55555555,0xaa2a5555,0x552a552a,0x524aa594,0x48242291,0x21444408;
0x55555555,0xd56ad56a,0xaa6a55d5,0x55555555,0xaa2a5555,0x92249224,0x22222222;
0xd56ad56a,0xd65aadb5,0xd65aadb5,0xd56ad56a,0xaa6a55d5,0x92242549,0x48242291;
0x6d6d6d6d,0x6d6d6d6d,0xb66d6ddb,0x6d6d6d6d,0xd65aadb5,0x25252525,0x92249224;
0xd56ad56a,0xd56ad56a,0xb66d6ddb,0x6d6d6d6d,0xd65aadb5,0x48242291,0x92249224
```

Meaning: 32-step movement bit patterns for Pac-Man, ghosts, tunnels,
frightened mode, and Elroy speeds.

`phase_change_frames`
Default:

```text
600,1800,2400,3600,4200,6000,6420;0,0,0,0,0,0,0;600,2100,2520,4020,4440,5640,5940;
420,1620,2040,3240,3540,4740,5040;420,1620,2040,3240,3540,65534,65535;
300,1500,1800,3000,3300,65534,65535;300,1500,1800,3000,3300,65534,65535
```

Meaning: scatter/chase schedule tables in video frames by difficulty group.

`fruit_points`
Default:
`100,300,500,500,700,700,1000,1000,2000,2000,3000,3000,5000,5000,5000,5000,5000,5000,5000,5000,5000`
Meaning: bonus-fruit score values by level.

`fruit_reset_timer`
Default: `138`
Meaning: encoded arcade timer byte controlling fruit lifetime on the maze.

`fruit_release_dots`
Default: `70,170`
Meaning: dot counts that spawn the first and second fruits.

`global_release_dots`
Default: `7,17,32`
Meaning: global dot thresholds that release Pinky, Inky, and Clyde when the
timer path is active.

### `maze-metadata.txt`

`portal_pair`
Default: `0,17|27,17`
Meaning: left/right tunnel pair used for wraparound travel.

`home_offset`
Default: `11.5,14.0`
Meaning: top-left offset of the ghost house within maze tile coordinates.

`home_connect_left`
Default: `12,14`
Meaning: left entrance tile that connects the ghost house to the maze graph.

`home_connect_right`
Default: `15,14`
Meaning: right entrance tile that connects the ghost house to the maze graph.

`blinky_start_pixels`
Default: `216,224`
Meaning: Blinky’s sprite start position in original screen pixels.

`pinky_start_pixels`
Default: `216,272`
Meaning: Pinky’s sprite start position in original screen pixels.

`inky_start_pixels`
Default: `184,272`
Meaning: Inky’s sprite start position in original screen pixels.

`clyde_start_pixels`
Default: `248,272`
Meaning: Clyde’s sprite start position in original screen pixels.

`pacman_start`
Default: `15,26`
Meaning: Pac-Man’s starting tile coordinate.

`fruit_start`
Default: `15,23`
Meaning: fruit spawn tile coordinate.

`pacman_start_pixels`
Default: `216,416`
Meaning: Pac-Man’s sprite start position in original screen pixels.

`fruit_start_pixels`
Default: `216,320`
Meaning: fruit sprite position in original screen pixels.

`ghost_deny_up`
Default: `12,14;15,14;12,26;15,26`
Meaning: tile positions where upward ghost turns are blocked to match arcade
pathing.

### `maze-logic.txt`

This file is a whole-layout override rather than a partial key/value merge. If
`~/.xyzzy/pacman/maze-logic.txt` exists, it fully replaces the embedded maze
tile map. The default file is the 36-row Midway maze layout shipped in
`assets/arcade/maze-logic.txt`.

## Platform Support

Sound effects and music are embedded in the binary and played in-process using
`rodio` on top of `cpal`. That removes the previous dependency on a
platform-specific command such as `/usr/bin/afplay` and makes the audio path
portable across macOS and Linux.

macOS is still the only platform that has been actively validated. Most of the
rendering and terminal code is already Unix-oriented, and the audio layer no
longer needs a separate Linux backend, but Linux support has not been tested
end to end yet.

To finish Linux support, the remaining work is:

- Verify Kitty graphics protocol support and terminal pixel sizing on Linux
  terminals such as Kitty and Ghostty, since rendering depends on a compatible
  terminal and `ioctl(TIOCGWINSZ)` reporting usable pixel dimensions.
- Run the game on real Linux machines or CI runners to confirm that `rodio` can
  open the default audio device cleanly on ALSA, PulseAudio, or PipeWire-backed
  setups.
- Add Linux-specific install notes for terminal choice, audio stack quirks, and
  any distro packages needed for building or running the app.
- Expand the test and release matrix so Linux builds and smoke tests are kept
  healthy going forward.
