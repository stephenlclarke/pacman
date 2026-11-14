<!-- markdownlint-disable MD013 -->
# Building and Releasing Pacman

## Renderer and Supported Downloads

The runtime creates a `winit` window and sends scene commands to `src/gpu_scene.rs` for GPU composition from cached textures and primitive instances. `src/wgpu_graphics.rs` presents the completed GPU texture. Game logic and asset preparation remain on the CPU; the full software renderer is used for media generation and GPU parity tests. `Cargo.toml` and `cargo tree --locked --depth 1` show the direct wgpu and winit dependencies.

The Release workflow builds these native targets:

| Runner | Rust target | Archive |
| --- | --- | --- |
| `macos-15` (ARM64) | `aarch64-apple-darwin` | `pacman-<tag>-aarch64-apple-darwin.tar.gz` |
| `windows-2025` (x86-64) | `x86_64-pc-windows-msvc` | `pacman-<tag>-x86_64-pc-windows-msvc.zip` |
| `ubuntu-24.04` | `x86_64-unknown-linux-gnu` | `pacman-<tag>-x86_64-unknown-linux-gnu.tar.gz` |
| `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | `pacman-<tag>-aarch64-unknown-linux-gnu.tar.gz` |

GitHub documents these runner architectures in its [hosted runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners). Windows x86 means x86-64 in this repository. There is no 32-bit Windows or Intel macOS release target.

Each archive contains `pacman` or `pacman.exe` plus `README.md`. Graphics, audio, and arcade defaults are embedded. A graphical desktop and suitable graphics drivers are required to play. The build checks do not open a window or verify interactive graphics/audio. macOS and Windows binaries do not have distribution signing or notarization; this workflow does not create installers or app bundles.

## Workflow Triggers and Checks

`.github/workflows/release.yml` runs on pull requests, pushes to `main` and `wgpu`, version-tag pushes matching `v*`, and manual dispatch. Manual dispatch tests the selected ref and uploads Actions artifacts; it does not publish a release. A ref without the wgpu/winit dependencies or wgpu help output fails the checks.

Each native runner checks formatting, release packaging tests, direct renderer dependencies, Rust tests, Clippy, the optimized binary build, and its `--help` output. Linux installs the ALSA development package before building. Checks use `Cargo.lock`. Keep `wgpu-hal` and `gpu-allocator` on their shared `windows` 0.62.0 interface version; resolving the allocator against cpal's older interface breaks Direct3D compilation. Audio dependencies can use additional Windows interfaces independently. The two expensive autopilot tests and one native GPU parity test remain ignored by default; `make gpu-test` explicitly runs the GPU readback comparison on a machine with a working adapter. Workflow action dependencies are pinned to commit SHAs.

Every successful build uploads an Actions artifact retained for 14 days. Tags use the tag in the archive name; other runs use the commit SHA. Only a version-tag push runs the publishing job, and all four build jobs must succeed first. That job checks the four expected archives, writes and verifies SHA256 checksums, creates a draft GitHub Release, uploads all archives and the manifest, and publishes it. Tags with a suffix such as `-rc.1` are marked as prereleases. Ordinary build jobs have read-only repository access; only the publishing job has `contents: write`. The built-in `GITHUB_TOKEN` supplies that permission; no personal token is needed.

## Local Validation

From the repository root:

```sh
make ci
make workflow-lint
make docs-lint
make gpu-test
make release-package RELEASE_TARGET=aarch64-apple-darwin RELEASE_LABEL=local
```

Use stable Rust, Python 3, `actionlint`, and `markdownlint`. `release-package` builds the selected target and calls the same Python packaging script as CI. Its archive appears in ignored `dist/`. The packager needs an existing target-specific release binary and rejects missing files, unsupported targets, and unsafe archive labels. Its tests cover all four archive layouts and executable permissions in Unix archives.

On a Windows machine, the equivalent commands are:

```powershell
cargo test --locked --all-targets --target x86_64-pc-windows-msvc
cargo clippy --locked --all-targets --target x86_64-pc-windows-msvc -- -D warnings
cargo build --locked --release --bin pacman --target x86_64-pc-windows-msvc
.\target\x86_64-pc-windows-msvc\release\pacman.exe --help
python scripts/package_release.py --target x86_64-pc-windows-msvc --label local
```

## Publish a New Version

1. Land the wgpu runtime and this workflow through the repository's pull-request process. Select the tested GPU-composition commit rather than an unrelated source ref.
2. Select an unused version. Update `Cargo.toml` and the package entry in `Cargo.lock` together, and update documentation that names the released version.
3. Run local checks and confirm that all native workflow jobs pass for the exact commit to release.
4. Create an annotated tag such as `v1.1.0` on that validated commit and push that tag. Use `v1.1.0-rc.1` for a prerelease. Do not move or reuse an existing tag.
5. Check the Release workflow and confirm that GitHub Releases contains all four archives and `pacman-<tag>-SHA256SUMS.txt`. Download and compare archive hashes before announcing the release.
6. Update the Homebrew tap separately if it should follow the new release. Its existing `1.0.0_1` formula pins the published wgpu commit, and its HEAD source follows `wgpu`; a GitHub Release does not automatically update the tap.

The workflow uses a draft so incomplete uploads are not published. If publication fails after draft creation, inspect the failed job and remove the incomplete draft before rerunning the workflow. The workflow deliberately refuses to overwrite an existing release. Never delete a published release or retag a version to recover an incomplete draft.

User download, checksum, extraction, and launch instructions are in the [README](../README.md#download-a-binary-from-github). Editing or manually running this workflow does not publish a new version.
