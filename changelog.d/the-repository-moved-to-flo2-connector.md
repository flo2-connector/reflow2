### Changed

- **reflow2's repository moved to `github.com/flo2-connector/reflow2`, and everything that tells
  you where to fetch it, clone it or file an issue now says so.** Anthony moved the repository
  from `github.com/sligara7` to the `flo2-connector` organization on 2026-10-03. GitHub redirects
  the old address, so nothing that used it has broken.
  - The install line is now
    `curl -fsSL https://raw.githubusercontent.com/flo2-connector/reflow2/main/tools/install.sh | sh`.
    It appears in the README, the consumer kit (`getting-started/README.md`, `SETUP.md`,
    `UPDATING.md`), `docs/collaborating.md`, and the `ci-gate` skill's workflow example.
  - `tools/install.sh` downloads from `flo2-connector/reflow2` by default. `REFLOW2_REPO` still
    overrides it.
  - `tools/reflow2_init.py` checks the new address for a newer upstream commit.
    `tools/reflow2_install.py` names the new install line when it finds no binary.
  - The `report-friction` skill searches and files issues in `flo2-connector/reflow2`.
  - `Cargo.toml`'s `repository` and the dependency checker's user agent name the new address.
  - **The container image has not moved yet.** Every image up to and including 0.78.0 is published
    only as `ghcr.io/sligara7/reflow2/reflow2-mcp`, so `getting-started/UPDATING.md` still gives
    that address. The release workflow names the image after the repository it runs in, so the
    next release publishes as `ghcr.io/flo2-connector/reflow2/reflow2-mcp`. That cut moves the
    image address in the docs.
  - **What to do:** nothing is required. If a script, Makefile or CI workflow of yours names
    `sligara7/reflow2`, change it to `flo2-connector/reflow2` when convenient.
