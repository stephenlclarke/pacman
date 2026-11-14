<!-- markdownlint-disable MD013 -->
# Pacman wgpu validation

## Motivation

The requested Kitty-to-wgpu port is already present in commit `78dc7c5`. Its documentation needs a clear explanation of the native window, controls, rendering architecture, and platform limits. The current Rust toolchain also rejects seven constant-size chunk iterators during the warnings-as-errors check.

## Expected behavior

Pacman launches through `winit` and presents frames through `wgpu` without requiring Kitty protocol support. Repository build, test, formatting, lint, and documentation checks pass.

## Scope

Clarify the README and replace constant-size chunk iterators with typed array slices without changing pixel conversion or rendering behavior. No GitHub issue number has been assigned.
