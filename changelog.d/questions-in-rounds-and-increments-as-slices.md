### Changed

- **detect-and-ask, genesis and design-an-argument put their questions to the
  person in rounds.** A round holds every question that is ready now, meaning
  its prerequisites are settled. Each question is numbered and worded so "yes"
  accepts the recommended answer, so "yes", or "yes to all but Q3", is a whole
  reply. A question that depends on one still open waits for a later round. A
  fact the agent can look up (in the design, the files, a document the person
  gave) is looked up while the round goes out, and never asked. detect-and-ask
  still orders the questions inside a round by the severity `detect_gaps` gives
  them; dependency decides which round a question is in. How one question is put
  has not changed: the recommendation first, what would change it, options to
  pick from with their consequences, the person's own language and their own
  field's words. For a person who is not in the chat, each round is one `batch`
  of `gaps_to_prompts` or `ask_question`. detect-and-ask also counted its
  obligations as six and listed seven; it says seven now. The technique is
  adapted from the grilling skill of mattpocock/skills (MIT) at 4ae2317.
  **What to do:** nothing; the served skills carry it.
- **plan-increments says how to cut an increment, not only how to order one.**
  Each increment is a vertical slice that can be shown working on its own, end
  to end: in software, one request answered from the screen to the stored
  record; in a building, one room finished and inspected; in a proof, one case
  carried to its conclusion. Restructuring that makes the work easier goes
  first, as its own increment. A change that reaches everything (a part every
  drawing cites, a unit every interface carries) takes three increments: add
  the new beside the old, move each user over, then remove the old. Before the
  plan is recorded, the agent asks two things: is each increment the right
  size, and is each dependency between increments real? The technique is
  adapted from the to-tickets skill of mattpocock/skills (MIT) at 4ae2317.
  **What to do:** nothing; the served skill carries it.
- **root-cause builds one repeatable check that shows the failure before any
  cause is weighed.** In code it is a command that goes red on the exact
  symptom; in a building model, a query; in numbers, a calculation record that
  re-runs; in a physical piece, a test procedure written down before it is run.
  The check is cut down until every piece left matters, measurements in steps ④
  and ⑤ answer to it, and it is kept so it shows the fix; step ⑦'s failing test
  is the code case of it. It is fast and runs with nobody at the controls where
  the design allows, and is never required to where it cannot. When no check
  can be built, the agent says so and step ⑨ says the cause rests on a failure
  nobody could show again. The steps keep their numbers. The technique is
  adapted from the diagnosing-bugs skill of mattpocock/skills (MIT) at 4ae2317.
  **What to do:** nothing; the served skill carries it.
- `skill_lint.py` checks that each of the five skills states its new rule.
