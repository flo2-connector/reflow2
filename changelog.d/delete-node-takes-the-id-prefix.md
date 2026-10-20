### Changed

- **`delete_node` no longer requires `node_type`.** Omitted, the id's prefix is
  taken as the type you mean (`req:` → Requirement, `dec:` → Decision …), and
  the node is deleted only when the one type holding the id is that type. A
  holder of another type, an id held by two types, or a prefix that names no
  type is refused, nothing is deleted, and the refusal names what the id holds
  and the `node_type` to pass. A `node_type` you pass works exactly as before.
  The reply to a bare-id delete also names the `node_type` it deleted.
