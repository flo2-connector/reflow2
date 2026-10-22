### Added

- **One search reaches a hub and every design it pins, through the `--call`
  door too.** `search_design` takes `tracked_designs: true`: on a design that
  pins others (`external_dependency` with a `graph_id`), each pinned design's
  committed export is read into memory, read-only, and searched with the same
  query. The reply is one ranked list in the shape a hub's address answers
  with: each hit names its `design`, a pinned design's `node_id` carries its
  graph_id as a prefix (`<graph_id>:<id>`), and `designs_searched`,
  `found_by_design`, `hits_withheld` and `read_from` say what was searched and
  where from. Every pinned design it did not search is named in `not_searched`
  with a `state` (`missing`, `unreadable`, `not_reached` for a design at an
  address, `nowhere`) and why. Nothing is written to any export, no git is run
  and nothing is fetched. Without the argument the reply is unchanged. Through
  a hub's address on a server the view already searches every member, and the
  reply says the argument adds nothing there.
- **A read can be aimed at a pinned design's export.** On a design that pins
  others, `get_node` (and any tool that changes nothing) given an id with a
  pinned design's prefix, or `member: "<graph_id>"`, runs on that design's
  committed export alone, so a hit from a tracked search opens. A write aimed
  at one is refused, and nothing is written.
- **The served instructions say: when you tell someone the design holds
  nothing about something, say where you looked.** The hub skill's search step
  names the tracked search and asks you to open and cite another design's
  record on the same subject rather than write a second copy.
