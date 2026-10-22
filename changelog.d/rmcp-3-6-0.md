### Changed

- **reflow2 is built on rmcp 3.6.0, the official Rust MCP SDK's latest
  release (was 3.5.0).** No tool changes: all 200 tool snapshots match. What a
  client can notice is in the SDK's own transport:
  - Over HTTP, a request with no session that is not `initialize` is now
    answered `400 Bad Request: Session ID is required`, where it was `422`. A
    client that starts with `initialize`, as flo2's gateway does, sees no
    change.
  - On the 2026-07-28 protocol over HTTP, a tool's invalid-params refusal now
    comes back as HTTP 200 with the JSON-RPC error inside, where it was HTTP
    400. A JSON-RPC response posted to that stateless endpoint is refused with
    400, where it was accepted and ignored.
  - On stdio, a single message longer than 16 MiB is now dropped by the SDK,
    with a line in the server's log and no reply. **What to do:** send large
    designs by path (`import_graph`), or over HTTP, whose limit is 4 MiB per
    request and is answered.
  - `--call` is not affected by that bound: the one-shot door runs its call
    over an in-process pipe, and that pipe has none, so `--call export_graph`
    on a large design (reflow2's own is about 37 MB) still answers. With the
    SDK's default it would have waited forever.
