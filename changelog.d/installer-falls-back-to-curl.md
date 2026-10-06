### Fixed

- **The installer no longer gives up when the GitHub CLI is installed but not signed in.**
  `tools/install.sh` used `gh` whenever it was on your PATH, and a signed-out `gh` ended the
  install, though the release is public and plain `curl` works. It now tries `gh` first and,
  when `gh` fails for any reason, downloads with `curl` and says so. **What to do:** nothing.
  Re-run the one-line install if it stopped at "could not download … (gh)".

### Changed

- **The VS Code guide says the never-exported warning shipped.**
  `docs/using-reflow2-in-vscode-without-mcp.md` still listed it as open after v0.80.0 delivered it.
  The guide now marks idea 14 and limitation 15 as shipped in v0.80.0.
