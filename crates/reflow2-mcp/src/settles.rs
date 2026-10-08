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
//! VERSION 2 — THE GENERIC WRITERS (`create_node`, `create_nodes`), since
//! 2026-09-29. Their settling value rides inside a property bag, so no
//! version-1 form can name it:
//!
//! ```json
//! "_meta": { "reflow2/settles": {
//!   "version": 2,
//!   "argument": "props",                   // create_node; "nodes" for create_nodes
//!   "when": "node_settles_intent",
//!   "node_rule": [
//!     {"node_type": "Requirement", "property": "status",   "when": {"not_in": ["proposed"]}},
//!     {"node_type": "Decision",    "property": "status",   "when": {"in": ["accepted", "deferred"]}},
//!     {"node_type": "DesignRule",  "property": "enforced", "when": "present"}
//!   ],
//!   "approver": "approver",                // "nodes[].approver" for create_nodes
//!   "unsigned": "refused"
//! } }
//! ```
//!
//! To evaluate it: take the node(s) the call writes — `{node_type, props}`
//! from the arguments for `create_node`, each item of `nodes` (`props`, alias
//! `properties`) for `create_nodes`. A node settles when `node_rule` has a row
//! for its `node_type` and that row's `when` — one of the version-1 forms,
//! read against `props[property]`; absent or null never settles — holds. The
//! call settles if any node does, and the signature goes on each settling
//! item. reflow2 additionally exempts re-writing a value the stored node
//! already holds (not a NEW settle), which a gateway cannot see; signing such
//! a write anyway is harmless. `node_rule` is `reflow2_core::intent::SETTLING`,
//! served, never a copy. A reader that knows only version 1 must REFUSE a
//! version-2 row rather than guess.
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
    /// A GENERIC writer: the argument holds a node's properties (or a list
    /// of `{node_type, props}` items), and the call settles when the node it
    /// writes would assert settled intent by the core's one table,
    /// `reflow2_core::intent::SETTLING` — served beside the row as
    /// `node_rule`, so a gateway evaluates exactly what reflow2 does.
    NodeSettles,
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
    // A requirement MOVED to another design leaves this one: dropped here, on
    // this owner's word (slice 3, 2026-10-08). A piece or a derived send
    // settles nothing here.
    Settles {
        tool: "send_to_design",
        argument: Some("kind"),
        when: When::In(&["moved"]),
        approver: "approver",
        unsigned: Unsigned::Refused,
    },
    Settles {
        tool: "add_requirement",
        argument: Some("status"),
        when: When::NotIn(LANDING),
        approver: "approver",
        unsigned: Unsigned::Refused,
    },
    // Held to the core table like every status row: a Decision settles at
    // `accepted` or `deferred`; recording one already `rejected` retires an
    // option, it decides nothing. This row read NotIn(proposed) until
    // 2026-09-29 and refused an unsigned `rejected` the gate never asks about.
    Settles {
        tool: "add_decision",
        argument: Some("status"),
        when: When::In(&["accepted", "deferred"]),
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
    // THE GENERIC WRITERS, since 2026-09-29. Their status arrives inside a
    // property bag, so until then no row could name it and they wrote any
    // settle unsigned and unremarked
    // (fact:root-cause-the-settle-rule-guards-the-typed-doors-and-the-generic-writers-go-around-it-2026-09-29).
    // They are constructors, so an unsigned settle is refused, as
    // add_decision's is. import_graph and apply_merge carry their signatures
    // as AUTHORED_BY edges in the document rather than an argument, so they
    // are not rows here: they write and REPORT an unsigned settle
    // (`settled_without_approver`), and the class test drives all four.
    Settles {
        tool: "create_node",
        argument: Some("props"),
        when: When::NodeSettles,
        approver: "approver",
        unsigned: Unsigned::Refused,
    },
    Settles {
        tool: "create_nodes",
        argument: Some("nodes"),
        when: When::NodeSettles,
        approver: "nodes[].approver",
        unsigned: Unsigned::Refused,
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

/// Would a node of `node_type` holding `props` (JSON) assert settled intent,
/// by the core's one table?
pub fn node_settles(node_type: &str, props: Option<&Value>) -> bool {
    let Some(sp) = reflow2_core::intent::settling_property(node_type) else {
        return false;
    };
    let v = props
        .and_then(|p| p.get(sp.property))
        .filter(|v| !v.is_null());
    match (sp.when, v) {
        (_, None) => false,
        (reflow2_core::intent::SettleWhen::Present, Some(_)) => true,
        (reflow2_core::intent::SettleWhen::NotIn(l), Some(v)) => {
            v.as_str().is_none_or(|s| !l.contains(&s))
        }
        (reflow2_core::intent::SettleWhen::In(set), Some(v)) => {
            v.as_str().is_some_and(|s| set.contains(&s))
        }
    }
}

/// The core table as served beside a generic writer's row.
pub fn node_rule() -> Value {
    Value::Array(
        reflow2_core::intent::SETTLING
            .iter()
            .map(|sp| {
                let when = match sp.when {
                    reflow2_core::intent::SettleWhen::Present => json!("present"),
                    reflow2_core::intent::SettleWhen::NotIn(v) => json!({ "not_in": v }),
                    reflow2_core::intent::SettleWhen::In(v) => json!({ "in": v }),
                };
                json!({"node_type": sp.node_type, "property": sp.property, "when": when})
            })
            .collect(),
    )
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
            // Needs the node type beside the value: see settles_args.
            When::NodeSettles => false,
        }
    }

    /// [`Self::settles_value`] for a string argument, as handlers hold it.
    pub fn settles_str(&self, value: Option<&str>) -> bool {
        self.settles_value(value.map(|s| Value::String(s.to_string())).as_ref())
    }

    /// Does a call with these arguments settle? Reads the declared argument
    /// out of the call — the form a gateway uses.
    pub fn settles_args(&self, args: &serde_json::Map<String, Value>) -> bool {
        if self.when == When::NodeSettles {
            let one = |item: &serde_json::Map<String, Value>| {
                let t = item.get("node_type").and_then(Value::as_str).unwrap_or("");
                let props = item.get("props").or_else(|| item.get("properties"));
                node_settles(t, props)
            };
            return match self.argument.and_then(|a| args.get(a)) {
                Some(Value::Array(items)) => items.iter().filter_map(Value::as_object).any(one),
                Some(_) => one(args),
                None => false,
            };
        }
        self.settles_value(self.argument.and_then(|a| args.get(a)))
    }

    /// The declaration as served under `_meta["reflow2/settles"]`.
    pub fn served(&self) -> Value {
        let when = match self.when {
            When::Always => json!("always"),
            When::Present => json!("present"),
            When::NotIn(v) => json!({ "not_in": v }),
            When::In(v) => json!({ "in": v }),
            When::NodeSettles => json!("node_settles_intent"),
        };
        // Version 2 marks the one form a version-1 reader cannot evaluate:
        // `node_settles_intent` reads the node type beside the value, from
        // `node_rule`. A reader that knows only the four version-1 forms must
        // refuse such a row rather than guess (flo2 #106 does).
        let version = if self.when == When::NodeSettles { 2 } else { 1 };
        let mut served = json!({
            "version": version,
            "argument": self.argument,
            "when": when,
            "approver": self.approver,
            "unsigned": match self.unsigned {
                Unsigned::Refused => "refused",
                Unsigned::RecordedWithNote => "recorded_with_note",
            },
        });
        if self.when == When::NodeSettles {
            served["node_rule"] = node_rule();
        }
        served
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

/// For the writers that carry their signatures IN the document rather than as
/// an argument — `import_graph` and `apply_merge`. They restore records, so an
/// unsigned settle is written; it is then NAMED, never silent
/// (fact:root-cause-the-settle-rule-guards-the-typed-doors-and-the-generic-writers-go-around-it-2026-09-29).
/// Read the settling nodes BEFORE the write, then [`SettleWatch::unsigned`]
/// after it.
/// One watched node: its type, id, and what was stored before the write.
type Watched = (
    String,
    String,
    Option<std::collections::HashMap<String, reflow2_core::Value>>,
);

pub struct SettleWatch {
    before: Vec<Watched>,
}

impl SettleWatch {
    pub fn before<'a>(
        g: &reflow2_core::DesignGraph,
        nodes: impl Iterator<Item = (&'a str, &'a str)>,
    ) -> Self {
        let before = nodes
            .filter(|(t, _)| reflow2_core::intent::settling_property(t).is_some())
            .map(|(t, id)| {
                let stored = g.get_node(t, id).ok().flatten().map(|n| n.properties);
                (t.to_string(), id.to_string(), stored)
            })
            .collect();
        Self { before }
    }

    /// Every node the write moved INTO settled intent that carries no
    /// approver signature now.
    pub fn unsigned(&self, g: &reflow2_core::DesignGraph) -> Vec<String> {
        let mut out = Vec::new();
        for (t, id, stored) in &self.before {
            let Some(after) = g.get_node(t, id).ok().flatten() else {
                continue;
            };
            let Some(settle) =
                reflow2_core::intent::newly_settles(t, id, stored.as_ref(), &after.properties)
            else {
                continue;
            };
            if !g.has_approver(id).unwrap_or(false) {
                out.push(settle.describe());
            }
        }
        out
    }
}

/// How many unsigned settles a report names before it gives the count only.
const NAMED_AT_MOST: usize = 50;

/// Attach `settled_without_approver` to a writer's report when there is
/// anything to say; untouched otherwise.
pub fn report_unsigned(reply: &mut Value, unsigned: Vec<String>) {
    if unsigned.is_empty() {
        return;
    }
    let count = unsigned.len();
    let named: Vec<String> = unsigned.into_iter().take(NAMED_AT_MOST).collect();
    if let Some(obj) = reply.as_object_mut() {
        obj.insert(
            "settled_without_approver".into(),
            json!({
                "count": count,
                "nodes": named,
                "note": "These nodes now assert SETTLED intent and carry NOBODY'S NAME: the \
                         document held no AUTHORED_BY role=approver edge for them. They were \
                         written, because this call restores records rather than making a new \
                         decision; where rule:design-intent-moves-only-on-the-owners-word is \
                         enforced they are a red build. Draw authored_by(role='approver') for each \
                         one whose owner's word you have.",
            }),
        );
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

    /// The typed rows that write a node's settling property are held to the
    /// core's one table — two copies of "which value settles" is how the
    /// generic writers went round the rule.
    #[test]
    fn every_status_row_says_what_the_core_table_says() {
        use reflow2_core::intent::{SettleWhen, settling_property};
        let rows = [
            ("add_requirement", "Requirement"),
            ("set_requirement_status", "Requirement"),
            ("add_decision", "Decision"),
            ("set_decision_status", "Decision"),
            ("add_design_rule", "DesignRule"),
        ];
        for (tool, node_type) in rows {
            let core = settling_property(node_type).expect("a settling type").when;
            let row = rule(tool).when;
            let same = match (row, core) {
                (When::NotIn(a), SettleWhen::NotIn(b)) | (When::In(a), SettleWhen::In(b)) => a == b,
                (When::Present, SettleWhen::Present) => true,
                _ => false,
            };
            assert!(
                same,
                "{tool} settles on {row:?}, the core table on {core:?}"
            );
        }
    }

    #[test]
    fn a_generic_writer_reads_the_node_it_writes() {
        let cn = rule("create_node");
        let args = |t: &str, p: Value| {
            json!({"node_type": t, "props": p})
                .as_object()
                .cloned()
                .expect("object")
        };
        assert!(cn.settles_args(&args("Decision", json!({"status": "accepted"}))));
        assert!(!cn.settles_args(&args("Decision", json!({"status": "rejected"}))));
        assert!(cn.settles_args(&args("DesignRule", json!({"enforced": false}))));
        assert!(!cn.settles_args(&args("Capability", json!({"status": "realized"}))));
        let many = rule("create_nodes");
        let batch = json!({"nodes": [
            {"node_type": "Capability", "id": "cap:a", "props": {}},
            {"node_type": "Requirement", "id": "req:a", "props": {"status": "accepted"}}
        ]});
        assert!(many.settles_args(batch.as_object().expect("object")));
    }

    #[test]
    fn a_tool_names_at_most_one_rule() {
        let mut seen = std::collections::BTreeSet::new();
        for s in SETTLES {
            assert!(seen.insert(s.tool), "{} has two rows", s.tool);
        }
    }
}
