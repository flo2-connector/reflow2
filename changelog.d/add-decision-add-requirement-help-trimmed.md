### Changed

- **`add_decision` and `add_requirement` cost less context to load.** Their
  argument help and tool descriptions now say only what an agent needs to call
  them right: what each argument is, its values, its default, and when a call
  is refused. `add_requirement` now also says it holds a theorem, lemma,
  conjecture, definition or evidence through `statement_kind`. Dates,
  measurements and the history of why each field was added moved into source
  comments. The `add_decision` listing is 4,282 characters shorter (15,184 to
  10,902) and the `add_requirement` listing 4,988 shorter (14,457 to 9,469);
  `review_relations`, which shares the relation item, is 576 shorter. No
  argument was added, removed, renamed or retyped, and nothing behaves
  differently.
