### Changed

- **The repo now names the Rust it is built and checked with: 1.100.0, in
  `rust-toolchain.toml`.** CI and the release builds install that version, not
  whatever `stable` is on the day. A step fails if the compiler that ran is not
  the pinned one, or if a workflow asks for `stable` again. A compiler upgrade
  is now a commit that changes one line. On 2026-08-24, and again on
  2026-11-13 when Rust 1.100 came out, a new stable release made every open
  pull request fail on code nobody had changed. The oldest Rust the crates
  build on is still `rust-version` (1.98) in `Cargo.toml`, and the monthly
  floor job now really builds at it: the toolchain file would otherwise have
  overridden it. **What to do:** building from source, rustup installs
  1.100.0 the first time you run `cargo` here. Nothing else changes.

### Fixed

- **Eleven server tests no longer fail to start the server when cargo builds
  tests in its new location.** Cargo 1.100 builds a test program under
  `target/<profile>/build/` instead of beside the server binary in
  `target/<profile>/deps`. These tests worked the server's path out from
  their own, so 55 of them failed with "spawn reflow2-mcp: No such file or
  directory". They now take the path cargo gives (`CARGO_BIN_EXE_reflow2-mcp`),
  as the other 28 test files already did. **What to do:** nothing.
