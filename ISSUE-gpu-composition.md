<!-- markdownlint-disable MD013 -->
# Move Live Pacman Composition onto the GPU

## Motivation

The original wgpu port uploaded a software-rendered full frame every redraw. The user requested that the GPU perform the composition itself, while retaining the visuals and the desktop release work.

## Expected Behavior

Draw cached maze, sprite, and text textures with GPU quads and rasterize shapes in WGSL. Reuse unchanged textures, release expired dynamic textures, and preserve scaling, color, transparency, clipping, and draw order. Keep game logic and asset preparation on the CPU, and retain software rendering for media and parity checks.

## Related Work

See [ISSUE-desktop-releases.md](ISSUE-desktop-releases.md). Native CI also exposed a Windows Direct3D interface mismatch in the lockfile and newer Clippy checks in maze traversal; those must be fixed for desktop builds to succeed.
