### Added

- **Every tool reflow2 serves now carries a `title`, and every writing tool says
  whether it can destroy.** `tools/list` gives each tool MCP's short human name
  beside its snake_case id (`add_requirement` is "Add a Requirement",
  `loop_status` is "What the Loop Owes"), and every tool whose `readOnlyHint` is
  false now states `destructiveHint`: true where a call can delete or overwrite
  what a person made (`delete_node`, `delete_edge`, `apply_heal`, `import_graph`,
  `apply_merge`, `contain_component`, `move_component`), false where it only
  adds, or revises with the prior state preserved. Nothing else changes: no tool
  name, argument, description or reply moved, so a client that read the old list
  sees only the new fields. A gate (`tools/every_tool_carries_a_title.py`) now
  holds the whole surface to it, so a tool added without them fails the build
  (`req:every-served-tool-carries-a-title-and-says-whether-it-destroys`).
