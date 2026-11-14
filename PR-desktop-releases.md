<!-- markdownlint-disable MD013 -->
# ci(release): validate and package native desktop wgpu builds

## Motivation

See [ISSUE-desktop-releases.md](ISSUE-desktop-releases.md). Release builds need native desktop checks and clear downloadable packages, with documentation matching the actual wgpu runtime.

## Implementation

- Confirmed `Cargo.toml`, `src/app.rs`, and `src/wgpu_graphics.rs` connect the game loop to a winit window and wgpu texture presentation; frame composition remains on the CPU.
- Run the native release matrix on pull requests, source branch pushes, manual dispatch, and tags. macOS ARM64 and Windows x86-64 are explicit targets; existing Linux architectures remain supported by the workflow.
- Check formatting, direct renderer dependencies, Rust tests, Clippy, release builds, and help output before packaging.
- Share archive packaging through `scripts/package_release.py`. Include executable and README in macOS/Linux tarballs and the Windows ZIP. Reject invalid labels and missing files; test archive contents and Unix executable permissions, including when archives are created on Windows.
- Publish all archives and a verified SHA256 manifest through a draft release only on version-tag pushes after all native builds pass. Use read-only build permissions, scoped release write permission, concurrency control, and pinned action commits.
- Document downloads, checksum checks, extraction, local packaging, workflow triggers, publication, historical Kitty releases, signing limits, and the independent Homebrew update.

## Validation

- `make ci`: 129 Rust tests pass; two expensive tests stay ignored. Clippy, formatting, and three packaging tests pass.
- `make workflow-lint docs-lint` and `git diff --check`: pass.
- Direct dependency inspection confirms wgpu and winit.
- `make release-package RELEASE_TARGET=aarch64-apple-darwin RELEASE_LABEL=local`: pass. The extracted executable is Mach-O ARM64 and its wgpu help smoke test passes from a temporary directory.
- Windows and Linux native builds are delegated to the updated GitHub Actions matrix; no graphical or audio smoke test has been claimed.

## Compatibility and Remaining Risks

The Windows download changes from a bare `.exe` to a `.zip` containing `pacman.exe` and the README. Windows x86 means x86-64, not 32-bit. Existing Linux release archives remain. This change does not create a tag or replace historical downloads. Windows/Linux native builds need the GitHub-hosted matrix; interactive graphics and audio, distribution signing, and notarization are outside the current validation. No personal access token is required to publish from Actions.
