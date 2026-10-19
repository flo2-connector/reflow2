### Fixed

- **A writing `--call` that keeps no export now says how to name one in
  `.reflow2.toml`.** The line said only "no --export-to was given, and no MCP
  configuration … names one", although the lookup has read the project's
  `.reflow2.toml` `[export] path` since 2026-10-06 (#691): the one record every later write,
  the Stop hook, `reflow2 init` and the gate read, and the one a store outside
  any repository has. The line is now built from the lookup's own list of
  sources, that file first (`<project>/.reflow2.toml` as
  `[export] path = "<FILE>"`), then `--export-to <FILE>` for one call, then the
  MCP configurations, and a test fails if a source is added without it.
