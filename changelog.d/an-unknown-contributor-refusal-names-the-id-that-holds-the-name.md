### Fixed

- **A refusal for a Contributor given by display name now names the id that holds that
  name.** `add_decision` (and every tool that takes an `approver`, the defect and gap
  acknowledgements, `authored_by`, `owned_by`, `ask_question`, `claim_region` and the other
  who-edges) refused `"Pat Doe"` with "no Contributor ... check the id", although a
  Contributor `who:pat` named Pat Doe was in the design. Some of those refusals even said to
  create the person, which would have made a second record of them. The refusal stays,
  because an approval must name one person for certain, but it now says "`Pat Doe` is the
  display name of Contributor `who:pat`: pass its id". When several Contributors share the
  name, it names them all. **What to do:** nothing; pass the id the refusal names.
