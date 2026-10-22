### Added

- **One composed check runs over a hub and several of its designs, through
  the `--call` door too.** `compose_and_analyse` takes `other_graph_ids`
  (several designs at once) or `tracked_designs: true` (every design this one
  pins). They are composed with this design in ONE combined graph, each under
  its own graph_id, and reflow2's ordinary checks run once over the whole. Each
  finding comes back under the designs it names: `by_design` for one,
  `by_pair` (`"a×b"`) for two, `across` for more, `of_the_whole` for a finding
  that names no node. So a finding between two members shows even when neither
  is the design calling. On a design served alone, each design is read from
  its pin's committed export; on a server that holds many designs, from its
  live store, inside the set the host says you may read; through a hub's
  address, `tracked_designs` composes every member. Every design that could
  not be read is named in `not_composed` with the same `state` a tracked
  search gives it (`missing`, `unreadable`, `not_reached`, `nowhere`,
  `unreachable`). Nothing is written to any design or export, and the reply
  is bounded by `budget_chars` (default 30,000), sampling only the finding
  lists and keeping every count. A call naming one other design (`design` or
  `other_graph_id`) answers exactly as before.
- **The hub skill says when to run the composed check**: before work that
  several of a hub's designs share.
