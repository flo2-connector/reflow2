### Changed

- **A proof's read-back says each premise's reason once.** `reasoning_report`
  re-rendered every premise's whole reason in every step that took it up, and
  said the "cited, unverified … Register the source" advisory once per
  citation, per row: on the heat-equation rung's theorem a page asked at 60,000
  reached the client at 63,998 characters and was refused. Each premise row's
  `why` now holds its reason's leading clause ("cited, unverified", "concluded
  by inf:… holds"); the full reason is said once per page in `premise_whys`,
  keyed by the premise's id (or in its own row, when the premise is a claim on
  the page), and an advisory is said once in `advisories`, naming every id it
  applies to. The words of every claim, each step's rule and warrant and each
  premise's name are unchanged. On the committed heat-equation fixture the
  whole walk went from 133,198 to 114,072 characters, and a 60,000 page holds
  20 whole claims instead of 18. **What to do:** read a premise's full reason
  from `premise_whys` by its id.
- **A reply asked at N through a hub's address arrives at N or less.** The hub
  prefixed every member id and added its `view_note` after the member had
  held its reply to the budget (+5,154 characters on that page). A read that
  takes `budget_chars` is now held with the hub's additions counted: when it
  arrives over, the hub asks the member again at a budget lowered by the
  overshoot, and the `view_note` says so. A write is never asked twice.
- **`get_skill` takes `budget_chars` (default 30,000).** It attached every
  lesson the design holds for the step with no bound: `root-cause` answered
  54,054 characters on reflow2's own design. The body always comes whole; the
  lessons are ranked newest first and withheld whole from the end, and
  `lessons` then says how many were withheld (`shown`, `total`) and names each
  by id in `withheld_ids`. **What to do:** read a withheld lesson with
  `get_node`, or ask with a larger `budget_chars` if your client has the room.
- **`propagate_from` takes `budget_chars` (default 30,000).** `max_depth`
  bounds the walk, not the reply: the steps a change breaks are followed to
  the end of their reasoning, and at `max_depth` 2 the radius from one cited
  lemma was 41,073 characters. Over budget, whole rows are withheld from the
  end of `inferences`, `impacted`, `direct_ring`, `risk_crossings` and
  `across_designs`; `budget.lists_shortened` names each list with `shown`,
  `total` and the ids `withheld`, and every count stays.
- **`--call` and `--describe` print compact JSON, and a client that reads only
  the text block gets compact JSON there.** Both were pretty-printed, so a
  reply held to 30,000 printed 35,087 characters (and one held to 60,000
  printed 73,497). What is printed is now exactly what the budget measured.
  **What to do:** pipe the output to `jq` to read it laid out.
