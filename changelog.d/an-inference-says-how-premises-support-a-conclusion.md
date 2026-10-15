### Added

- **A design can now say how its premises support a conclusion, and reflow2 checks that the
  conclusion follows.** A new record, an `Inference`, sits between the premises and the one
  conclusion they support, and holds the rule: `all` (every premise must hold), `any` (one is
  enough, for redundant support) or `k_of_n` with its `k` (a quorum). A named rule such as modus
  ponens is `all` with `rule_name: modus ponens`, and `warrant` says why, in words a person can
  challenge. `add_inference` writes the step in one call: the rule, the premises, the conclusion
  and those optional fields. It refuses a bad rule or `k` with the legal values, a premise or
  conclusion that does not exist, and a circle, before anything is written. Premises link in with
  the new `PREMISE_OF` edge, and the conclusion links out with `CONCLUDES`.
  reflow2 then reads each premise's truth from what it already computes. A requirement holds when
  it is delivered or `met`, and fails when `dropped`. A check holds when `passing`, a decision
  when `accepted`, and a constraint when nothing violates it. Any other record is `unknown`, never
  guessed. reflow2 evaluates the rule over holds, fails and unknown, and the result shows in four
  reads that already exist:
  - **delivery:** a requirement a step concludes is delivered when the step holds;
  - **every blast radius** (`propagate_change`, `propagate_from`): a new `inferences` list names
    each step a changed premise reaches as `broken`, `weakened` (it still stands on its other
    premises) or `unknown`;
  - **`get_node`** on a step: its verdict, as `inference_reading`;
  - **`detect_gaps`:** a new finding, `conclusion_does_not_follow`, for a requirement marked
    `met` (or a check `passing`, a decision `accepted`, a capability `verified`) that its steps
    do not support. The status is reported, never changed.

  It checks the form, never the content: a false premise passes.
  **Nothing changes for a design that records no `Inference`.** Delivery, every blast radius,
  `get_node`, the gaps and the defects read exactly as before. One addition reaches every
  design: `derived_report` lists one more declared relation, `follows`, with a count of 0.
  **What to do to see the new tool:** a session started before the upgrade has the old tool
  list, so restart it, or reconnect with `/mcp` once the server it reaches runs this release (a
  local shared daemon needs `--stop-shared` first, as AGENTS.md says). A design hosted on
  flo2.io serves the new tool only after flo2.io runs this release and flo2 regenerates its list
  of offered tools: both its doors offer only the tools on that list. Its MCP endpoint also
  declares that its tool list never changes, so after that a Claude Code session on a hosted
  design must reconnect (`/mcp`) to see `add_inference`.
  **Upgrade note:** the schema gains one node type and two edge types. A store this release has
  opened is stamped with them, and an older reflow2 then refuses to open it, as for every
  vocabulary addition before it.
