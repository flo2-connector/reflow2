### Fixed

- **Every one-shot open now says what its version check found.** `reflow2 read`, `--call`,
  `write`, `--export`, `--import` and `--diff` used to drop the verdict the serving modes print, so
  an upgrade, a downgrade and a repair on open went unreported. Each now prints one line on stderr
  when there is something to say. A store a newer reflow2 wrote gets
  *"WARNING — THIS GRAPH WAS LAST WRITTEN BY reflow2 X; you are running Y, which is BEHIND it"*, and
  its newer stamp is no longer rewritten down. MCP replies, and the stdio and served behaviour, are
  unchanged.
- **`graph.meta.json` keeps `previous_reflow2_version` and `last_repair_on_open`**, so an upgrade
  can be confirmed after the process that made it has gone. They are optional keys; older
  binaries ignore them.
- **The installer repairs a `reflow2` command that is a symlink to the binary.** It used to fail
  with a UnicodeDecodeError, even under `--check`. It now replaces the link (never writing through
  it) with its wrapper, and says so.

### Changed

- **`install.sh` keeps the binary it replaces** as `reflow2-mcp.<old version>`, named from its
  `--version`, and prints the path. Only the last one is kept. **To roll back**, copy it over
  `reflow2-mcp`. UPDATING.md now starts with "Before you install: export and commit each design,
  back up `.reflow2/`", documents an install with only the binary (check `head -2
  ~/.local/bin/reflow2` first), and no longer says a downgrade goes unchecked.
- **The hub and impact-check skills say how to carry a blast radius into a member design**,
  through its mirrored surface.
