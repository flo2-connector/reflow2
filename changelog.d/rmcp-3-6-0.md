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
  - The SDK's new 16 MiB limit on a single incoming message is lifted, so
    nothing changes there: a session on stdio, and the one-shot `--call`
    door, answer a message of any length, as before (rmcp 3.6.0 would drop a
    longer one with no reply). HTTP keeps its 4 MiB limit per request, which
    is answered `413` and is unchanged. Tip: a large design still travels
    best by path (`import_graph`).
