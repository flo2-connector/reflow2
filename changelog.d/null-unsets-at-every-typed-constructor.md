### Fixed

- **A `null` now unsets a field at every typed constructor, not only at
  `create_node`.** The Shop sample trial sent `limit: null` to `add_constraint`
  to clear a numeric limit, and the limit stayed: the typed request read an
  explicit `null` and a missing field as the same thing, so the merge kept the
  stored value. The 2026-09-07 rule ("a null unsets", v0.51.0) had been built
  into `create_node`'s props only. Now every `add_*` constructor, `plan_epoch`
  and `record_finding` follow it on a revise: a `null` on an optional property
  removes it (a property with a schema default reads its default again, as at
  `create_node`) and the reply names it under `unset`; a `null` on a required
  property is refused by name and nothing is written; a field left out keeps
  its stored value, as before. Two kinds of field are never unset by a null,
  and the call is refused by name with nothing written: a `status` (the
  refusal names the `set_*_status` tool that moves it), and a field the schema
  keeps as additive history (`distinct_from`). A `null` on a parameter that is
  not a stored property (an edge to draw, `approver`) still means the same as
  leaving it out.
- **Behaviour change for MCP clients: a `null` is no longer the same as
  leaving a field out.** A client that sends `null` for every field it leaves
  empty will now CLEAR those fields when it revises a node, or be refused on a
  `status` or history field. **What to do:** send only the fields you mean to
  change; send `null` only to clear one.
