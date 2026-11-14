<!-- markdownlint-disable MD013 -->
# Validate and Publish Desktop wgpu Builds

## Problem

The existing tag-only release workflow could publish an old Kitty source tree, copied a bare Windows executable without user instructions, and did not test either desktop platform before release. Repository documentation did not explain the downloadable archive names, checksums, or publication process.

## Expected Behavior

Validate the native wgpu runtime on macOS ARM64 and Windows x86-64 before publishing downloadable GitHub Release archives. Preserve existing Linux outputs. Include the executable, README, and release-wide checksum manifest, and explain remaining interactive validation and signing limits.

## Scope

Update `.github/workflows/release.yml`, add a shared packaging script and tests, expose local validation targets, and update the README and release guide. No issue number is assigned.
