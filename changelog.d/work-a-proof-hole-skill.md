### Added

- **A served skill works one hole in a proof: `work-a-proof-hole`
  (`/work-a-proof-hole`)** (req:a-served-skill-works-a-proof-hole-refute-first-with-each-candidate-a-path-in-the-graph).
  It runs root-cause's refute-first steps on a proof's open step or lemma. It
  states the hole as an observation, then searches the design and the given
  sources for precedent, including the failed routes already recorded toward
  it. It generates at least three candidate routes, each with a stated reason,
  and registers each as an alternative under the hole's decision point
  (`register_alternative`), with no more than three open at once. Before a
  route is worked, it writes into the decision point what would refute it, and
  it works the routes cheapest refuter first. A route that fails becomes a
  `failed_route` record with where and why it fails, and a change of route is
  a decision with `CAUSES` from the source or result that forced it. It never
  lets a step read proven on the agent's word, and leaves settling the decision
  point to the owner. Each session ends with what is closed, what is open, the
  failed routes counted from `reasoning_report`, and the next route, written
  into the design so that a fresh agent can start from it.
  - **It adds no record type and no tool.** It composes what this release
    serves: `failed_route` (above), the fork tools, `review_relations` and
    design-a-proof's statements and steps. design-a-proof points to it from its
    section on failed routes.
  - **The cap and the order are discipline, not enforcement.** Nothing refuses a
    fourth open route or a route worked before its refuter is written; the
    record shows both, for whoever judges the session. reflow2 does not compute
    the most efficient route over a proof's graph.
  - **In your setup.** The skill is served by `get_skill` wherever reflow2
    serves skills: over MCP, through `reflow2 read get_skill` / `--call`, and on
    flo2.io once flo2.io runs this release. Every call it names works the same
    through `--call`, `failed_route` included.
