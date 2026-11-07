### Added

- **A route that was tried and fails is a record of its own, read back with the
  line of reasoning it belongs to** (req:a-failed-route-is-a-record-of-its-own-read-back-with-the-reasoning,
  Anthony 2026-10-30). Working a hole in a proof leaves approaches that did not
  go through; until now they could only be prose on a finding, which nobody
  could find or count, or DEPENDS_ON edges to the failing steps, which put the
  finding in every what-if's blast radius (the heat rung, 2026-10-22). Record
  one with `add_requirement` and `statement_kind: failed_route`: `statement`
  says what was tried, `route_to` names the claim it was a route to (the hole),
  `fails_at` says the step at which it fails (and `fails_at_step` names that
  step or statement when the design holds it), `fails_because` says why, and
  `route_rests_on` names what it rests on.
  - **A route can say what would make it go through.** `would_hold_if` holds
    the natural condition under which the route WOULD hold ("if f is
    Boolean-valued", "with exp(d) in place of poly(d)"), so a dead end for one
    claim is kept as a possible result for another
    (req:a-failed-route-records-the-natural-condition-under-which-it-would-hold).
    It is read back with the route and moves no reading: the route still fails
    for its claim, and the condition is neither a hypothesis of that claim nor
    a result shown.
  - **It supports nothing and refutes nothing.** It reads `failed_route`, never
    `open`, `stated` or `unproven`, and its truth never holds. It is never a
    premise, an assumption or the conclusion of a step, and never an objection:
    `add_inference` and the store refuse each link, and `add_requirement`
    refuses to turn a record that already takes part in a step into one. No
    claim reads proven, refuted, contested or open because of it, and it is
    never a gap, a debt or an unsupported claim. On the committed Riemann map,
    every gap, defect, debt count and claim reading is the same with a failed
    route recorded as without it.
  - **No walk crosses it.** Its references are fields, not edges, so a what-if
    on the step it fails at, or on anything it rests on, never reaches it.
  - **Where it is read.** `reasoning_report` reads each route whole under the
    claim it was a route to (that claim's new `failed_routes`) and tallies them
    in the report's new `failed_routes`: how many, how many name both the step
    and the reason (`with_step_and_reason`, what working a hole is scored on),
    which do not, and in full any whose claim the read did not walk. `get_node`
    on a route adds `failed_route`, the record read whole with each id it names
    resolved. `search_design` finds it by what was tried, where it fails and
    why, and a hit on a route carries `route_to`.
  - **The design-a-proof skill says how** (section 5a): how to record a failed
    route, never to draw DEPENDS_ON to the step that fails, and how to record a
    turn in the search (a Decision whose `decision` names the route left and
    the route taken, with CAUSES from the source or result that forced it).
  - **Additive only.** A design that records no failed route reads exactly as
    before: no new field appears in any reply, and every existing call answers
    as it did. The tool list gains six optional `add_requirement` fields, each
    refused on any statement that is not a failed route. It works the same on
    every door — the MCP server, `reflow2 write add_requirement` / `--call`,
    and a design hosted on flo2.io once flo2.io runs this release; flo2.io's
    own copy of the proof skills says nothing of it until flo2 updates it.
