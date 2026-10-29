### Added

- **Every cue reflow2 gives that names a next call now has an id, and `usage_report` counts whether the agent acted on it.**
  A cue is a sentence in a reply's `loop_hint`, `hint`, `next`, `nudge` or
  `next_step` that names a tool or skill. Its id is `<field>/<named call>`
  (`loop_hint/set_decision_status`, `next/loop_status`), carried in the reply's
  `_meta["reflow2/cues"]` and written on the call's line in the usage ledger.
  `usage_report`'s tally gains a `cues` block: per cue, how often it was
  offered, how often the same session made that call within its next five
  calls or at all, and which tools offered it. Only reflow2's own words are
  kept, never the sentence, a node id or an argument. In the recall pilot 4% of
  cues were followed within five calls; this is how the next measurement is
  read. Nothing to do: the field is additive, and a ledger written by an
  earlier release reads as before.
- **An agent is told, before its first call, which read says where the design stands.**
  The server's connect-time instructions and the kit's session-start hooks
  (Claude Code and the VS Code terminal route) now name `graph_report_markdown`
  as the first read (a few KB: what the design holds, what is delivered, the
  top open gaps), and `search_design` for the design's own record of where
  things stand ("current state"). The connect-time offer is counted like any
  other cue (`instructions/graph_report_markdown`). The hook text replaces
  "call open_questions" as the first step; `open_questions` is unchanged.
- **A search hit on a record with no name carries an `excerpt` of its text.**
  A nameless Fragment or Question used to come back with `name: ""` and nothing
  else. It now carries up to 160 characters of its title, statement, question,
  summary or description. `name` is unchanged, and a named hit gets no excerpt.

### Changed

- **A Decision's "settle it" hint goes to whoever can give the owner's word.**
  `add_decision` and `replace_text` used to tell every caller that
  `set_decision_status` was the next step. When the call is made for nobody who
  owns the design (no `writes_for`, or the design is `OWNED_BY` someone else),
  the hint now says the settlement is a question for the owner and names
  `ask_question`. A call made for the owner, or for a named person on a design
  that names no owner, gets the hint as before.
