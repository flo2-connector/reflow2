### Added

- **A served skill for designing an argument: `design-an-argument` (`/design-an-argument`).**
  For a line of reasoning that is not a proof — an argument for and against, rival positions on
  principles, objections and replies. It records cases as given and principles as chosen, derives
  each position from a principle, aims each objection at what it attacks (a claim, a premise or
  the step itself), and keeps rival positions as an open dispute: a contradiction each way, or a
  Decision the owner keeps `contested`, never a one-way contradiction that silently refutes one
  side. `find_skills` finds it from "argument", "line of reasoning" and "for and against", and
  `design-a-proof` now points to it instead of excluding the case.

### Fixed

- **An objection aimed at a step is read against that step.** A CONTRADICTS into an Inference
  attacks the step's warrant or rule. It used to be stored and ignored: the step read `holds` and
  `reasoning_report` did not list it. The step's reading now lists it under `objections` and says
  how it stands in `objected`: `defeated` while the objection holds (the step supports nothing;
  its conclusion is left unsupported, never refuted), `contested` while it stands, `answered`
  once a reply answers it. `reasoning_report` walks such objections from the claim the step
  concludes and lists each defeated or contested step under `steps_objected`. **What to do:** if
  you wrote a challenged warrant as an extra premise only to give an objection a target, you can
  now aim the objection at the step itself.
- **A claim held on several lines names every line.** A claim supported by several steps, each
  resting on different assumptions, holds under any one of them. It used to read as holding under
  the first line's assumption alone ("Driver: steering permissible", held by five principles, read
  "conditional on act consequentialism"). Its read-back, foundations and proof status now name
  every line in `holds_under_any_of`. A step citing such a claim inherits only what its own
  cheapest line needs, never another line's assumption. An objection holding only under one
  line's assumption contests a claim that also holds outside it, and no longer refutes it.
