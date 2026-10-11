### Added

- **A hub's address serves a view over its members, when the host names them.** A registry
  server behind a trusted gateway or a bearer gate (flo2.io's shape) now reads
  `_meta["reflow2/view_graph_ids"]` — the hub's members the caller may read — on a call to the
  hub's own design, and serves that address as a view. A call naming an id as
  `"<member graph_id>:<id>"` runs on that member alone, its prefix stripped and its reply's ids
  prefixed back, a write included, credited to the caller. A read naming no member runs on the hub
  and then on each readable member in turn and answers `{"view", "by_design", "unreachable"}`;
  `search_design` answers one merged list, every design's hits ranked together. A write naming no
  member writes the hub, exactly as before. Exports, releases, claims, seats, served text and the
  cross-design tools act on the hub alone and say so in `view_note`. A call naming ids in two
  members is refused before anything runs (a relation across designs is a typed link record:
  `review_relations` with `other_design` and `other_id`), and a member outside
  `reflow2/readable_graph_ids` is refused naming the id and nothing it holds. No tool or argument
  was added, and a call without the key is answered byte for byte as before. **What a host must
  do:** send `reflow2/view_graph_ids` (the hub's members ∩ the caller's readable set) together
  with `reflow2/readable_graph_ids` on every call to a hub, reads included.
