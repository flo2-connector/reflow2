### Added

- **An agent can find reflow2's type for a word from mathematics: "lemma", "hypothesis",
  "induction", "counterexample".** Each type now carries an `also_called` list by field, with what each
  mapping means and what it does not. Mathematics is the first field: a theorem, proposition,
  lemma, corollary or conjecture is a Requirement; a hypothesis, axiom or definition is a
  Constraint; a proof strategy or approach is a Decision; a cited result or reference is an
  Artifact; a counterexample (A refutes B) is a CONTRADICTS edge. An inference step, a proof step,
  a rule of inference, a two-column proof's row and the methods of proof (direct proof, modus
  ponens, induction, contraposition, proof by contradiction, construction, exhaustion or double
  counting, case analysis, the probabilistic method) are an Inference, with each premise a
  PREMISE_OF edge and "therefore" a CONCLUDES edge. A lemma held as a
  Requirement whose status is `met` has been SHOWN. It is PROVEN only when every step it rests on
  is confirmed, which reflow2 does not compute yet.
  - `describe_schema` with `node_type` set to a field's word answers the type that holds it:
    `{"node_type": "lemma"}` answers Requirement, as `{"node_type": "Requirement"}` does, plus
    `found_by_term` (the field, the term and what it means) and a `note`. Case and spacing do not
    count. A word for an edge type, such as `counterexample` or `premise`, answers that edge
    whole, with its `drawn_by` tools (for PREMISE_OF and CONCLUDES, `add_inference`, which
    draws them with the step). Until now these calls were refused with "Unknown node type".
  - Every node type's answer carries its `also_called` list, and so does every edge type in the
    full listing.
  - `find_tools` indexes each term against the tools that write its type, so "record a lemma"
    ranks `add_requirement` first, "add a hypothesis" ranks `add_constraint` first and "prove it
    by induction on n" ranks `add_inference` first. Each item a term found carries
    `also_called`, saying which word found it. Its description now says it searches a field's
    words too.

  **Nothing a client already uses changes.** In tools/list only `find_tools`' description
  changes, by the clause that says it searches a field's words; no tool's description or schema
  carries a term. Every type's own name answers as before, a word that is
  neither a type nor a term is refused as before, and a query in reflow2's own words ranks as
  before. The only difference in any existing answer is the added `also_called` key, and, in an
  answer already over its reply budget, the `budget` block's counts, which now include it.
