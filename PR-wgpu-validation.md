<!-- markdownlint-disable MD013 -->
# docs(render): document and validate the Pacman wgpu port

## Motivation

The existing wgpu implementation needed explicit native-window instructions and validation against the current Rust toolchain. See `ISSUE-wgpu-validation.md`.

## Implementation

- README: document wgpu/winit, controls, Homebrew install/upgrade commands, the wgpu Cargo source branch, build requirements, software frame composition, GPU presentation, Defender reference, and remaining platform checks.
- `src/app.rs`: show the installed `pacman` command and Cargo launch in help.
- `src/render.rs`, `src/sprites.rs`, `src/text.rs`: use typed array chunks for RGBA, RGB, and grayscale-alpha buffers to satisfy the current Clippy check while preserving behavior.
- The renderer port itself already exists in commit `78dc7c5`; this branch builds on it rather than duplicating it.

## Validation

- `make ci`: formatting, all-target tests, and warnings-as-errors Clippy pass; 129 tests pass and two expensive autopilot tests and the later native GPU parity test remain ignored by default.
- `cargo build --locked`: pass on macOS.
- Markdown lint and whitespace checks: pass; README media paths exist and documented arcade defaults match the embedded files.
- Homebrew formula version `1.0.0_1` was published in tap commit `788ce62`; source installation, formula test, installed help, style, and strict online audit pass on macOS.
- Native launch stays running without reported errors. UI automation could not bind to the standalone process, so live visuals and keyboard interaction were not verified; the test process was stopped afterward.

## Compatibility and remaining risks

The original wgpu port used CPU composition and wgpu texture presentation; the [GPU composition follow-up](PR-gpu-composition.md) replaces that live path; Defender's current sprite atlas renderer is more extensive. Linux/Windows graphical and audio smoke tests remain unverified. No SonarCloud scan or pull request has been published. Remote native CI is now being used for the desktop release and GPU composition follow-ups. The follow-up [desktop release change](PR-desktop-releases.md) adds native build checks and release packaging; it does not retroactively validate Windows/Linux interactive play.
