### Changed

- **The served instructions now name the same first read as the handshake.**
  `get_instructions` said "Start with `open_questions`" at step 0a, while the
  connect-time instructions and the session-start hooks said to read
  `graph_report_markdown` first, so an agent reading both was given two first
  steps. Step 0a now reads `graph_report_markdown` first, names `search_design`
  for the design's own record of where it stands ("current state", "status as
  of"), and then `open_questions`. The VS Code terminal route's "Start of a
  session" begins with the same read. Nothing to do: `open_questions` is
  unchanged.

### Fixed

- **The owner working alone is offered `set_decision_status` again.** The
  settling hint on `add_decision` (proposed) and `replace_text` treated a call
  made for nobody as a call from someone who is not the owner. So a local
  session with no `writes_for`, and every `--call`, was told settling was "a
  question for the owner", even on a design that names no owner. The hint now
  offers `set_decision_status` unless the caller is known (named by
  `writes_for`, or by the gateway that serves the engine for others) and is
  not the owner. On flo2.io the gateway names every caller, so nothing changes
  there: the owner gets `set_decision_status` and other members get
  `ask_question`. On an engine served for others, a call that names nobody
  cannot sign, so it still gets `ask_question`.
