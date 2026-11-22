### Added

- **`import_graph` can CHECK a document and write nothing: `check_only: true`.**
  The whole import runs exactly as a real one would, with every other option as
  given and under the caller's own signing policy, and is then discarded
  whether or not it found faults. The design, its identity and its committed
  export are left exactly as they were. The reply cannot be read as a write: it
  carries `checked_only: true`, `written: false` and a `note` saying in
  plain words that this was a check, with `would_be_refused`, `faults` (every
  item the same import would be refused for, in the same words the refused
  import uses) and `would_write: {nodes, edges}`. It carries no `nodes_written`
  or `edges_written`. A check that finds faults is a normal reply, not an
  error. On an empty store it names the identity the import would adopt in
  `would_adopt_identity` and adopts nothing. Through `--call`, a check writes
  no export and says so in one line on stderr. Nothing to do: the option is off
  unless asked for.
- **`import_graph` can be ADD-ONLY: `add_only: true`.** A node whose id the
  design already holds, under any type, is refused, and so is an edge it
  already holds (same type, same two ends). Each is listed with every other
  fault, and nothing is written. New nodes are written, and so is a new edge
  between nodes the design holds, or between one it holds and a new one.
  Nothing to do: off unless asked for.
- **`import_graph` can take PROPOSALS ONLY: `proposals_only: true`.** The
  document may propose and may not settle. Refused, each listed with every
  other fault: a node that would arrive settled (an accepted requirement or
  decision, a rule with `enforced` stated, an increment closed without
  arriving); a node the design holds whose settling property the document
  would change at all, back out of a settled value included; an `AUTHORED_BY`
  edge it would create or change that carries the `approver` role. What counts
  as settled is read off the one table every settle is judged by, and is
  judged on what would be stored, after `merge` and the schema's defaults.
  Nothing to do: off unless asked for.

The three options combine with each other and with `merge`. For a Rust caller,
`ImportOptions` gains `check_only`, `add_only` and `proposals_only` (all false
by `Default`), and `ImportReport` gains `checked_only`, `faults`, `would_write`
and `would_adopt_identity`: **what to do** is build `ImportOptions` with
`..Default::default()` if you write it as a literal.
