### Added

- **A conclusion now shows the axioms it ultimately rests on.** When a node is concluded by at
  least one Inference, `get_node` includes a `foundations` field beside `inference_reading`: the
  list of terminal premises (nodes not concluded by any Inference) that the conclusion's inference
  chain rests on, each with its id, type, truth value, and why. The walk follows every
  alternative derivation and takes the union, so a theorem supported two ways shows all the
  axioms both paths need. The field carries an honest limit: "These foundations show what this
  conclusion follows from. They never say the axioms are right: in an abstract design they are
  chosen principles or values, and they may be contested by design."
  When a node is not concluded by any Inference the field is omitted (no foundations to show, and
  an absent field is "not a conclusion", not "rests on nothing"). No gap or finding fires.
  A new declared derived relation, `foundations`, lists every node in the design that is a
  foundation (an axiom) of at least one conclusion: the design-wide view.
  **Nothing changes for a design that records no Inference.** The field is absent, the derived
  relation counts 0, and every other read is unchanged.
