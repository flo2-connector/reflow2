### Added

- **`find_tools` finds relations as well as tools.** Ask it "link an actor to a
  capability" and its reply now carries `relations` beside the tools: the schema's
  edge types that answer, best first (at most three), each marked as a relation,
  with the types it joins, what it means, and the call that draws it — its typed
  tool where one exists (`constrains`, `governed_by` …), and `create_edge` with
  that `edge_type` always. A relation with no typed tool, such as
  `INTERACTS_WITH`, used to be unreachable by describing the job: a chat searched
  six times and found it only through `describe_schema`. The tools' own ranking
  is unchanged, and no tool is added.
- **Every `find_tools` reply says how to ask what joins an X to a Y** (`what_joins`):
  `describe_schema` with `from` and `to` set to the two node types.
