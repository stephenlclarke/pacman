<!-- markdownlint-disable MD013 -->
# feat(render): compose Pacman scenes on the GPU

## Motivation

See [ISSUE-gpu-composition.md](ISSUE-gpu-composition.md). Live rendering should use the GPU for composition instead of uploading a CPU-rasterized full frame.

## Implementation

- Add `gpu_scene.rs` and WGSL shaders for cached sprite textures and instanced primitive quads. Preserve draw order, nearest texel selection, integer coverage, and encoded-color alpha blending, with sRGB conversion for presentation.
- Cache GPU textures by image identity with weak ownership; unchanged images upload once and expired images leave the cache. Reuse the instance buffer and grow it as needed.
- Send `FrameData` directly from the application to the GPU. Replace the live software renderer with viewport dimensions for mouse mapping; retain the software renderer for media generation and the GPU oracle.
- Share the existing viewport transform rather than duplicating scaling logic.
- Align `gpu-allocator` and `wgpu-hal` on Windows interface version 0.62.0 in `Cargo.lock` to fix native Direct3D compilation. Keep cpal's separate version. Update maze loops for the CI compiler's lint rules without changing graph construction.
- Document the GPU/CPU responsibilities, current source branch, older Homebrew pin, test commands, and release workflow.

## Validation

The native GPU parity test passes on Apple M5 Pro with the Metal backend. It compares title, gameplay, and primitive/alpha scenes at four window sizes, with at most one byte of rounding difference per color channel. It also checks repeated-frame identity, texture reuse, and expiry. Local Rust CI (129 passing, three ignored by default), packaging tests, workflow lint, and documentation lint pass. An actual native window process stayed running for five seconds without reported render errors and was then stopped. The desktop Actions matrix is rerun for the final commit.

## Compatibility and Limits

Game logic, PNG decoding, text image preparation, and draw-command construction remain on the CPU. The software reference is kept for testing and media generation. This is not distribution signing or audio/interactive play validation. The published Homebrew formula remains pinned to its earlier native-window build until updated separately. No release tag or historical binary is replaced by this code change.
