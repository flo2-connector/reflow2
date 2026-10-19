### Added

- **Through a hub, any tool can be run on one named member with `member`.** A call made to a
  hub's address may pass `member`, the graph_id of one of its members (or of the hub itself), and
  the tool runs on that design alone. This reaches the tools that take no node id, which until now
  answered for the hub or for every design at once: `loop_status`, `reasoning_report` with no
  claim, `detect_gaps` with no scope, `describe_schema`, `find_tools`, `find_skills`, `get_skill`,
  `get_instructions` and `usage_report`. It also reaches `claim_region` and `release_claim`, and it
  keeps a `search_design` or `scan_nodes` to that one design. A hub's `tools/list` declares
  `member` on every tool when the host names the hub's members on the listing request. Every other
  listing is unchanged. **What to do:** nothing; a host serving hubs (flo2.io) sends
  `_meta["reflow2/view_graph_ids"]` on a hub's `tools/list` as it does on its calls.

### Fixed

- **Every id in a reply routed to a hub's member now carries the member's prefix.** Until now
  only ids under keys named `…_id` or `…_ids` did. A reasoning step's `assumes`, `conclusion`,
  `discharges` and `cases`, a report's summary lists, and the ids named in a refusal's words came
  back bare, so they could not be passed back through the hub.
- **A tool the hub's view does not serve no longer writes a member's id into the hub.**
  `add_release {id: "<member>:rel:…"}` through a hub stored a Release in the hub's own design under
  that literal prefixed id, and nothing could delete it through the hub. An id naming a member now
  routes every tool to that member.
