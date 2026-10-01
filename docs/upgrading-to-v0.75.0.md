# Upgrading to v0.75.0

🛑 **Upgrade every seat together, and read the first section before you upgrade a server that
other people or machines reach.** This release moves the schema stamp and changes four things
that callers depend on:

- who may sign on a server that others reach;
- the shape of the settles declaration;
- the shape of every write's reply;
- four removed flow and capability arguments.

| | v0.74.0 | v0.75.0 |
| --- | --- | --- |
| Node types | 28 | 28 |
| Edge types | 65 | **66** (+ `ACTS_FOR`) |
| `Constraint.composition` | — | **new enum**: `sum`, `path` |
| `Question` | — | new optional `asked_of`, `answered_by`, `answered_at`, `batch`, `batch_position` |
| `AUTHORED_BY` | — | new optional `authored_via`, `reviewed_via`, `approved_via` |
| `_meta["reflow2/settles"]` | not served | version 1 **and version 2** |
| A write's reply | the whole stored node | **a receipt** (`echo: "node"` for the old reply) |

## 1 · 🛑 A server that others reach refuses approvals until you say who is calling (#636)

This is the change most likely to stop someone's work. It applies when reflow2 is **served for
others**: `--registry-root`, or `--http-allow-host` naming a host that is not loopback. Local use
is unchanged: stdio, `--shared`, and `--http` answering loopback only.

On such an engine, a signature is now checked where the store writes it. The check covers every
approval, and every move of a status into settled intent. It sits at the store, so it applies to
every writer (`create_edge`, `create_edges`, `draw_edges`, `acknowledge_gaps`, `import_graph`, the
typed helpers), and no tool can go around it.

- **AN ENGINE BEHIND A GATEWAY MUST DECLARE IT, OR IT REFUSES EVERY APPROVAL.** Start it with
  `--http-trusted-gateway <NAME>`, or set `REFLOW2_TRUSTED_GATEWAY=<NAME>`. The gateway must name
  the signed-in person's Contributor on every `tools/call` in `_meta["reflow2/writes_for"]`,
  overwriting anything the client sent. That name is the caller. An `AUTHORED_BY` that names anyone
  else, as author or approver, is refused and nothing is written. Deleting someone else's
  `AUTHORED_BY` is refused too.
  **flo2.io: set `REFLOW2_TRUSTED_GATEWAY=flo2.io` on the engine container in the same deploy that
  moves it to 0.75.0.** Without it, the hosted engine refuses every approval.
- **AN EXPOSED `--http` SERVER WITH NO GATEWAY BECOMES READ-AND-PROPOSE-ONLY.** Reads and proposals
  work, and every approval is refused, naming the flag. **This affects any team server run as a
  plain `--http` engine on a shared network.** There is no way to sign through it until reflow2
  verifies a Bearer token (the OAuth / OIDC sign-in half of #616 fix 4), which is **not in this
  release**. Until then, the choices are:
  - put a gateway that authenticates callers in front of it, and declare that gateway;
  - or keep the server local (loopback, stdio or `--shared`), where nothing changed.
- The handshake says which mode an engine is in, and so does the operator's startup banner.

## 2 · `_meta["reflow2/settles"]` has a version-2 form: readers must learn it (#635)

Every tool that can settle intent declares what settles it under `_meta["reflow2/settles"]`, on
its `tools/list` entry. v0.75.0 serves the declaration for the first time. The tools declare one of
two forms:

- **Version 1** (the nine typed settling tools): `{version: 1, argument, when, approver, unsigned}`.
- **Version 2** (the generic writers `create_node` and `create_nodes`):
  - the settling value rides inside a property bag, so `when` is `"node_settles_intent"`;
  - `node_rule` lists `{node_type, property, when}`, and each `when` is a version-1 form.

**A consumer that reads this declaration (a gateway that signs on its caller's behalf, as flo2
does) must learn version 2, and must REFUSE a row whose version it does not know.** A reader that
treats a version-2 row as version 1 will judge the generic writers wrongly. The exact shape, and
how to evaluate it, is in `crates/reflow2-mcp/src/settles.rs`.

## 3 · A write replies with a receipt, not the whole node (#630)

Every tool the served surface marks as a write now replies with a receipt:

- the node's or edge's id and type;
- each stored value of at most 200 characters, echoed as stored;
- each longer value, given by size under `elided`;
- a revise's replaced fields, with `prior_chars`, `after_chars` and `prior_in` (the snapshot that
  keeps the prior value);
- every warning, note, drawn edge and removal report, unchanged.

A prior value that nothing else holds (`fields_at_risk`) is still echoed in full.

**If a script or agent read fields back out of a write's reply, pass `echo: "node"`.** That returns
the reply exactly as before: the whole stored node and every prior value. Any other `echo` value is
refused before the tool runs. Read tools are unchanged and do not take `echo`.

## 4 · New tools, and four removed arguments

- **`draw_edges`** is the bulk form of every typed edge helper. Each item names the helper and
  carries that helper's own arguments. The item runs the helper's own code, with its checks and its
  refusal words. The batch is all or nothing, and `check_only` writes nothing. `create_edges` stays
  the GENERIC bulk form and runs no typed helper's checks.
- **`derived_report`** is read-only. It counts each of reflow2's 23 declared derived relations over
  the design, through the code path the declaration names, with example ids. The declarations are in
  `schema/derived/relations.yaml`.

**Removed arguments (#622).** A caller still sending one of these is refused by name ("unknown
field"). Remove the argument:

- `add_flow` no longer takes `entry_point` / `exit_point`;
- `add_capability` no longer takes `is_entry_point` / `is_exit_point`.

A Flow's entry and exit are now computed from its step order. `flow_report` returns
`entry_points` and `exit_points` in place of `entry_point` and `exit_point`.

## 5 · The schema changes, and what an older binary does

- **`ACTS_FOR`** (edge types 65 → 66) records the agent a write or an approval went through, beside
  the person it was for. `AUTHORED_BY` carries the agent in `authored_via` / `reviewed_via` /
  `approved_via`. A session names its agent with `writes_for`'s `acting_agent`, or a request names
  it in `_meta["reflow2/acting_agent"]`. The agent must be an existing `automated_agent`
  Contributor. It is attribution only and never signs anything.
- **`composition` on a Constraint** is `sum` or `path`. A budget's verdict follows what it declares.
  An undeclared budget keeps its verdict on the sum, and `budget_report` says which total it judged.
- **A Question records whom it was put to and who answered.** It has `asked_of`, `answered_by`,
  `answered_at`, `batch` and `batch_position`. `answer_question` takes `answered_by`, `answered_at`,
  `record` and `note`. `gaps_to_prompts` and `open_questions` take `asked_of`.

**What an older binary does.** A v0.75.0 binary re-stamps every store it opens with 66 edge types.
**After that, a v0.74.0 or older binary refuses to open that store**: its schema names an edge type
the older one has never heard of. It also refuses to import a v0.75.0 export without
`--accept-newer`.

## What you do

1. **Before upgrading a server that others reach**, decide its mode (section 1). Behind a gateway,
   declare it in the same change. With no gateway, expect read-and-propose-only.
2. **Update every binary that opens the design**: every machine, every harness, the container.
   The installer's update does it; a hand-built copy needs rebuilding. On a box that serves
   `target/release/reflow2-mcp`, stop the shared daemons afterwards, so every session respawns on
   the new build.
3. **Update any code that does one of these** before the upgrade reaches it:
   - consumes `_meta["reflow2/settles"]` (section 2);
   - reads fields out of write replies (section 3);
   - passes the removed flow or capability arguments (section 4).
4. **Nothing to migrate by hand.** A v0.75.0 binary opens an older graph. The first open or import
   repairs, once, every relation the store keeps twice (#622):
   - it draws each missing `HAS_TEMPORAL_FACT` copy;
   - it turns a finding's non-subject `HAS_TEMPORAL_FACT` into `ABOUT_ENTITY`;
   - it removes the retired entry and exit values.

   It reports each repair, never silently. Expect the next export to change accordingly.

The full list of changes, including the fixes, is in [CHANGELOG.md](../CHANGELOG.md) under 0.75.0.
