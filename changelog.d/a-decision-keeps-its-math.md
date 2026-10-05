### Changed

- **A decision that rests on numbers is recorded with its math.** Five served skills say so in a
  few lines each: `capture-intent`, `brainstorm`, `detect-and-ask`, `revise-design` and
  `capture-session`. When the answer someone will act on is a cost, a fit, a size, or a yes or no
  that turns on conditions, the decision's rationale names where each input came from. That is the
  person, a cited document, or a measured or reported value, never the agent's memory. If a
  calculator helper is connected, the agent computes there and links the helper's kept record to
  the decision with `documents`. If none is, it shows the arithmetic in the rationale. No helper is
  required.
  - Measured on 2026-10-05 on a hosted design: three chats given such decisions did the arithmetic
    in their heads and kept no computation, even with a calculator connected. The one sent to the
    calculator showed a wrong hidden assumption: a seat taken as 7.5 mm that is really 7.6 mm.
  - `tools/skill_lint.py` pins the new wording in all five skills (the decision-math contract).
  - **What to do:** nothing. The skills are served, so a project gets the new wording with the
    binary.
