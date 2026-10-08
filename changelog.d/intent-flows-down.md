### Added

- **A requirement or decision can be sent to another design, a parent's to its part or a customer's
  to a supplier, without copying it.** Two new tools, one for each side. In the receiving design,
  `receive_from_design` holds the requirement at `proposed`, attributed to the sender, with where it
  came from. In the sending design, `send_to_design` records the send. `kind` says how it travels:
  - `moved`: the requirement belongs wholly to the receiver and leaves the sender. A reference to
    its new home replaces it, and its ending is kept as history. It needs the sending owner's
    `approver`, and is refused while a capability in the sender still satisfies it.
  - `piece`: the requirement stays with the sender, and the receiver holds its own part, linked
    with `DECOMPOSES` across the designs.
  - `derived`: a decision stays with the sender, and the receiver holds the requirements it forces,
    `GOVERNED_BY` that decision.

  What lands is binding only once the receiver's owner accepts it. `upstream_status` lists what was
  `received` and `sent`, and `loop_status` names what waits under `received_waiting`. A ripple in
  the sender carries down to what the part received. **What to do:** nothing. The `/hub` skill
  offers this when a requirement of the whole belongs to a part.

### Changed

- **`DECOMPOSES` can reach a cross-design reference**, so a part's piece can decompose its parent's
  requirement. This is additive. **What to do:** nothing. An older reflow2 refuses such an edge if
  a design carrying one is imported into it.
