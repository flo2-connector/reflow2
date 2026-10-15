### Added

- **An agent can find reflow2's type for a word from mathematics: "lemma", "hypothesis",
  "counterexample".** Each type now carries an `also_called` list by field, with what each
  mapping means and what it does not. Mathematics is the first field: a theorem, proposition,
  lemma, corollary or conjecture is a Requirement; a hypothesis, axiom or definition is a
  Constraint; a proof strategy or approach is a Decision; a cited result or reference is an
  Artifact; and a counterexample (A refutes B) is a CONTRADICTS edge. A lemma held as a
  Requirement whose status is `met` has been SHOWN. It is PROVEN only when every step it rests on
  is confirmed, which reflow2 does not compute yet.
  - `describe_schema` with `node_type` set to a field's word answers the type that holds it:
    `{"node_type": "lemma"}` answers Requirement, as `{"node_type": "Requirement"}` does, plus
    `found_by_term` (the field, the term and what it means) and a `note`. Case and spacing do not
    count. A word for an edge type, such as `counterexample`, answers that edge whole, with its
    `drawn_by` tools. Until now these calls were refused with "Unknown node type".
  - Every node type's answer carries its `also_called` list, and so does every edge type in the
    full listing.
  - `find_tools` indexes each term against the tools that write its type, so "record a lemma"
    ranks `add_requirement` first and "add a hypothesis" ranks `add_constraint` first. Each
    item a term found carries `also_called`, saying which word found it.

  **Nothing a client already uses changes.** tools/list is byte-identical: no tool's
  description or schema mentions a term. Every type's own name answers as before, a word that is
  neither a type nor a term is refused as before, and a query in reflow2's own words ranks as
  before. The only difference in any existing answer is the added `also_called` key, and, in an
  answer already over its reply budget, the `budget` block's counts, which now include it.
