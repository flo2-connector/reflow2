//! WHICH CALLS SETTLE INTENT — declared once, served on the tool, and read by
//! reflow2's own signature checks and by anything that stands between a caller
//! and reflow2.
//!
//! `rule:design-intent-moves-only-on-the-owners-word` says a call that moves
//! intent past its landing status needs the owner's name on it. Until
//! 2026-09-29 that knowledge lived in each handler — `status != "proposed"` in
//! one, `enforced.is_some()` in another, `accepted | deferred` in a third — and
//! nothing a program could read said which argument of which tool settles. So a
//! hosting layer that signs settles on its caller's behalf had to guess, and
//! flo2's gateway guessed "an argument named `status`": `add_design_rule`'s
//! `enforced` went through unsigned and reflow2 refused it (flo2
//! fact:root-cause-the-gateway-reads-settling-from-a-field-named-status-and-lets-an-enforced-rule-through-unsigned-2026-09-29).
//! flo2 #102 contained that with a hand-kept copy of the rules (`SETTLES` in its
//! `identity.ts`), and `collapse_decision` settled a Decision with no approver
//! at all (fact:collapse-decision-settles-a-decision-with-no-approver-and-says-nothing-2026-09-28).
//!
//! THE CURE is this table. Every tool that can settle intent has ONE entry; the
//! handlers compute "does this call settle?" and "what happens when it names
//! nobody?" from it; and [`declare_on`] serves it on the tool itself, under
//! `_meta["reflow2/settles"]`, so a gateway reads the rule instead of copying it.
//! A test (`every_settling_call_declares_what_settles_it`) enumerates the served
//! tools and fails when a tool that takes an approver has no entry, or when a
//! declaration and the tool's behaviour disagree.
//!
//! THE SERVED SHAPE, one object per settling tool:
//!
//! ```json
//! "_meta": { "reflow2/settles": {
//!   "version": 1,
//!   "argument": "status",                 // null when every call settles
//!   "when": { "not_in": ["proposed"] },   // "always" | "present" | {"in": [..]} | {"not_in": [..]}
//!   "approver": "approver",               // where the signature goes; "gaps[].approver" for a batch
//!   "unsigned": "refused"                 // or "recorded_with_note"
//! } }
//! ```
//!
//! `when` reads against the call's arguments: `always` — the call settles by
//! being made; `present` — any value of `argument` settles (either value of
//! `enforced`); `not_in` — a value is given and it is not one of these (the
//! landing status); `in` — the value is one of these. For a batch (`approver`
//! of the form `list[].field`) the rule applies to each item.

use rmcp::ErrorData as McpError;
use serde_json::{Value, json};

/// The key the declaration is served under, inside a tool's `_meta`.
pub const META_KEY: &str = "reflow2/settles";

/// What a settling call does when it names nobody.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unsigned {
    /// Refused before anything is written — the constructors, where the
    /// status can simply be left at its default instead.
    Refused,
    /// Written, and the reply says it carries nobody's name — the setters and
    /// the acknowledgements, which have consumers that must still be able to
    /// act in a design that has modelled no Contributor
    /// (cap:an-acknowledgement-says-whose-judgement-it-was).
    RecordedWithNote,
}

/// Which values of the declared argument settle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    /// Every call settles: the call itself is the owner's act.
    Always,
    /// Any value present settles.
    Present,
    /// A value is given and it is none of these.
    NotIn(&'static [&'static str]),
    /// The value is one of these.
    In(&'static [&'static str]),
}

/// One tool's settling rule.
#[derive(Debug)]
pub struct Settles {
    pub tool: &'static str,
    /// The argument whose value decides; `None` when [`When::Always`].
    pub argument: Option<&'static str>,
    pub when: When,
    /// Where the signature goes: `approver`, or `list[].approver` for a batch.
    pub approver: &'static str,
    pub unsigned: Unsigned,
}

const LANDING: &[&str] = &["proposed"];

/// THE TABLE. One row per tool that can move intent past its landing status.
/// `authored_by` is not here on purpose: with `role: approver` it IS the
/// signature, not a settle that needs one.
pub const SETTLES: &[Settles] = &[
    Settles {
        tool: "add_requirement",
        argument: Some("status"),
        when: When::NotIn(LANDING),
        approver: "approver",
        unsigned: Unsigned::Refused,
    },
    Settles {
        tool: "add_decision",
        argument: Some("status"),
        when: When::NotIn(LANDING),
        approver: "approver",
        unsigned: Unsigned::Refused,
    },
    Settles {
        tool: "add_design_rule",
        argument: Some("enforced"),
        when: When::Present,
        approver: "approver",
        unsigned: Unsigned::Refused,
    },
    Settles {
        tool: "set_requirement_status",
        argument: Some("status"),
        when: When::NotIn(LANDING),
        approver: "approver",
        unsigned: Unsigned::RecordedWithNote,
    },
    // Settling a question and setting one aside are the owner's word;
    // superseding and rejecting retire rather than decide.
    Settles {
        tool: "set_decision_status",
        argument: Some("status"),
        when: When::In(&["accepted", "deferred"]),
        approver: "approver",
        unsigned: Unsigned::RecordedWithNote,
    },
    // Choosing a fork's winner moves the Decision to `accepted`.
    Settles {
        tool: "collapse_decision",
        argument: None,
        when: When::Always,
        approver: "approver",
        unsigned: Unsigned::RecordedWithNote,
    },
    // An acknowledgement mints an accepted Decision.
    Settles {
        tool: "acknowledge_gap",
        argument: None,
        when: When::Always,
        approver: "approver",
        unsigned: Unsigned::RecordedWithNote,
    },
    Settles {
        tool: "acknowledge_defect",
        argument: None,
        when: When::Always,
        approver: "approver",
        unsigned: Unsigned::RecordedWithNote,
    },
    Settles {
        tool: "acknowledge_gaps",
        argument: None,
        when: When::Always,
        approver: "gaps[].approver",
        unsigned: Unsigned::RecordedWithNote,
    },
];

/// The rule for `tool`, if it can settle intent.
pub fn declared(tool: &str) -> Option<&'static Settles> {
    SETTLES.iter().find(|s| s.tool == tool)
}

/// The rule for `tool`, which the caller knows is in the table. A handler
/// asking for a tool the table does not hold is a programming error the
/// enumeration test exists to make impossible to ship.
pub fn rule(tool: &str) -> &'static Settles {
    declared(tool).unwrap_or_else(|| panic!("`{tool}` settles intent and has no row in SETTLES"))
}

impl Settles {
    /// Does a call whose declared argument holds `value` settle?
    /// `None` means the argument was not passed.
    pub fn settles_value(&self, value: Option<&Value>) -> bool {
        let value = value.filter(|v| !v.is_null());
        match self.when {
            When::Always => true,
            When::Present => value.is_some(),
            When::NotIn(landing) => value.is_some_and(|v| match v.as_str() {
                Some(s) => !landing.contains(&s),
                None => true,
            }),
            When::In(set) => value
                .and_then(Value::as_str)
                .is_some_and(|s| set.contains(&s)),
        }
    }

    /// [`Self::settles_value`] for a string argument, as handlers hold it.
    pub fn settles_str(&self, value: Option<&str>) -> bool {
        self.settles_value(value.map(|s| Value::String(s.to_string())).as_ref())
    }

    /// Does a call with these arguments settle? Reads the declared argument
    /// out of the call — the form a gateway uses.
    pub fn settles_args(&self, args: &serde_json::Map<String, Value>) -> bool {
        self.settles_value(self.argument.and_then(|a| args.get(a)))
    }

    /// The declaration as served under `_meta["reflow2/settles"]`.
    pub fn served(&self) -> Value {
        let when = match self.when {
            When::Always => json!("always"),
            When::Present => json!("present"),
            When::NotIn(v) => json!({ "not_in": v }),
            When::In(v) => json!({ "in": v }),
        };
        json!({
            "version": 1,
            "argument": self.argument,
            "when": when,
            "approver": self.approver,
            "unsigned": match self.unsigned {
                Unsigned::Refused => "refused",
                Unsigned::RecordedWithNote => "recorded_with_note",
            },
        })
    }
}

/// Serve every declaration on its tool. Tools the table does not name are
/// returned untouched.
pub fn declare_on(tools: Vec<rmcp::model::Tool>) -> Vec<rmcp::model::Tool> {
    tools
        .into_iter()
        .map(|mut t| {
            if let Some(s) = declared(&t.name) {
                let mut meta = t.meta.take().unwrap_or_default();
                meta.0.insert(META_KEY.to_string(), s.served());
                t.meta = Some(meta);
            }
            t
        })
        .collect()
}

/// The sentence a setter's reply carries when a settling call names nobody.
pub const NOBODYS_NAME: &str = "This status is settled intent and carries NOBODY'S NAME: no \
     `approver` was passed, so no AUTHORED_BY role=approver edge was drawn. Where \
     rule:design-intent-moves-only-on-the-owners-word is enforced that is a red build. Re-issue \
     with `approver` (the Contributor whose word this is), or draw authored_by(role='approver') \
     yourself.";

/// THE ONE GATE every settle path goes through. `settles` is the table's
/// answer for this call. Returns `Ok(None)` when nothing needs saying,
/// `Ok(Some(note))` when the declared policy is to record and say so, and the
/// refusal when the declared policy is to refuse — BEFORE anything is written.
pub fn gate(
    tool: &str,
    settles: bool,
    approver: Option<&str>,
    what: &str,
) -> Result<Option<String>, McpError> {
    if !settles || approver.is_some() {
        return Ok(None);
    }
    match rule(tool).unsigned {
        Unsigned::Refused => Err(McpError::invalid_params(
            format!(
                "`{tool}` will not record {what} with nobody's name on it: pass `approver` — \
                 the Contributor whose word this is — in the same call, and the signature is \
                 drawn as AUTHORED_BY role=approver. Or omit the status and let it land at the \
                 default; an agent may draft and recommend without limit, but moving intent \
                 past its landing status is the owner's act \
                 (rule:design-intent-moves-only-on-the-owners-word). Which arguments settle \
                 this tool is declared on it, under _meta[\"{META_KEY}\"]. Nothing was written."
            ),
            None,
        )),
        Unsigned::RecordedWithNote => Ok(Some(NOBODYS_NAME.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_names_its_argument_exactly_when_it_needs_one() {
        for s in SETTLES {
            assert_eq!(
                s.argument.is_none(),
                s.when == When::Always,
                "{}: an argument is named iff the rule reads one",
                s.tool
            );
        }
    }

    #[test]
    fn the_rules_read_the_arguments_the_way_the_handlers_did() {
        let st = rule("add_requirement");
        assert!(!st.settles_str(None));
        assert!(!st.settles_str(Some("proposed")));
        assert!(st.settles_str(Some("accepted")));
        let dr = rule("add_design_rule");
        assert!(!dr.settles_value(None));
        assert!(
            dr.settles_value(Some(&json!(false))),
            "either value of `enforced` settles"
        );
        let ds = rule("set_decision_status");
        assert!(ds.settles_str(Some("deferred")));
        assert!(
            !ds.settles_str(Some("rejected")),
            "rejecting retires, it does not decide"
        );
        assert!(rule("collapse_decision").settles_value(None));
    }

    #[test]
    fn a_tool_names_at_most_one_rule() {
        let mut seen = std::collections::BTreeSet::new();
        for s in SETTLES {
            assert!(seen.insert(s.tool), "{} has two rows", s.tool);
        }
    }
}
