### Added

- **A lite tool profile: a session can be listed 45 tools instead of 200.** A
  host that loads every tool's description paid about 140,000 tokens a session
  for reflow2's list, while the 40 most-used tools carried 97.4% of 30,439
  counted calls. A lite session is listed those tools, four more, and
  `use_another_tool`; every other tool is still served, one call away. **To
  turn it on:** start the server with `--profile lite`, or set
  `REFLOW2_PROFILE=lite` (with `--shared` or `--remote` the client asks its
  server for lite itself). Over HTTP, a session can choose it for itself by
  connecting to the MCP endpoint URL with `?profile=lite`, such as
  `http://127.0.0.1:7000/?profile=lite` or `/g/<graph_id>/?profile=lite`. The
  profile is fixed when a session connects and never changes during it. A
  value that is not `full` or `lite` is refused at startup, or at the
  handshake for a URL. **`full` is the default**: a session that asks for
  nothing is listed every tool, as before, plus `use_another_tool`.
- **`use_another_tool` calls any reflow2 tool by its name.** Give the tool's
  name in `tool` and its arguments in `arguments`. The reply is that tool's own
  reply, and a refusal is that tool's own refusal. This is how a lite session
  reaches a tool it is not listed. It is served in both profiles, so there is
  one tool surface. Call `find_tools` first to get the name.
- **`find_tools` gives each tool it finds its whole input schema**
  (`input_schema`, exactly as `tools/list` serves it), so a tool found there
  can be called without a second read. Schemas are added best match first, up
  to about 20,000 characters. The top match always gets its schema, and any
  match left without one is named under `schemas_left_out` (`describe_schema`
  with `tool` reads it). In a lite session it still searches every tool, not
  only the listed ones.
