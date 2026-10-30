### Added

- **Behind a trusted gateway, the gateway can name who owns a design, and the settling cue
  follows it.** A request's `_meta["reflow2/design_owners"]` (a list of Contributor ids) is the
  host's word on who owns the design that one call is on, read the way `reflow2/writes_for` names
  the caller. The cue after `add_decision` and `replace_text` then offers `set_decision_status` to
  an owner it names and `ask_question` to anyone else, on a design that records no `OWNED_BY` at
  all. Nothing is written into the design from it, and it grants nothing: who may sign is still
  the caller alone. Until now the engine knew an owner only from `OWNED_BY`, which flo2.io never
  writes, so on flo2.io a member a design was shared with was still offered the settling call.
  **What to do:** a gateway in front of reflow2 sends the key on each call; nothing else changes.
  Honoured only under `--http-trusted-gateway`; ignored on a local engine, behind a bearer gate,
  and on a call a hub's view routes to a member (those owners are the hub's).
- **Behind a trusted gateway, each caller gets its own seat in the usage ledger.** A request's
  `_meta["reflow2/seat"]` (a handle of at most 128 printable characters, never a person's name)
  is written on that call's ledger line in place of the engine session's seat, so `usage_report`
  follows each cue per caller. A gateway that carries everyone over one engine session per design,
  as flo2.io does, had every caller on one seat, and a cue offered to the owner read as followed
  by a member's call. **What to do:** a gateway sends a stable seat per caller on each call.
  Honoured only under `--http-trusted-gateway`; `claim_region` still takes its seat as an
  argument.
