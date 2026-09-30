//! WHICH STORED VALUES SETTLE INTENT — one predicate every writer asks.
//!
//! `rule:design-intent-moves-only-on-the-owners-word` (ENFORCED) says a node
//! that asserts intent reaches a settled state only on the owner's word. Until
//! 2026-09-29 that rule was enforced at each typed DOOR — the constructors and
//! setters since 2026-09-06, the settles table (`reflow2-mcp` `settles.rs`)
//! since #628 — and never at the STATE CHANGE every writer makes. So the
//! doors nobody wrote a row for went round it: `create_node`, `create_nodes`,
//! `import_graph` and `apply_merge` wrote an accepted Decision, an accepted
//! Requirement or an enforced DesignRule with nobody's name and no note
//! (fact:root-cause-the-settle-rule-guards-the-typed-doors-and-the-generic-writers-go-around-it-2026-09-29).
//!
//! This table is the one answer to "does this stored value settle intent?".
//! The typed rows in `settles.rs` are held to it by a test, the generic
//! writers ask [`newly_settles`] before they write, and
//! `tools/check_intent_authority.py` is held to it by a test that runs its
//! `settles_intent` over the same cases.

use std::collections::HashMap;

use crate::DynoError;
use crate::foundation::core::Value;
use crate::graph::{DesignGraph, authored_roles};
use crate::nodes::{edge, node};

/// Which values of a settling property settle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettleWhen {
    /// A value is given and it is none of these (the landing status).
    NotIn(&'static [&'static str]),
    /// The value is one of these.
    In(&'static [&'static str]),
    /// Any value present settles — either value of `enforced` is the owner's
    /// choice about consequence.
    Present,
}

impl SettleWhen {
    /// Does this stored value settle? `None` / `Null` means absent.
    pub fn settles(&self, value: Option<&Value>) -> bool {
        let value = value.filter(|v| !v.is_null());
        match self {
            SettleWhen::Present => value.is_some(),
            SettleWhen::NotIn(landing) => value.is_some_and(|v| match v.as_str() {
                Some(s) => !landing.contains(&s),
                None => true,
            }),
            SettleWhen::In(set) => value
                .and_then(Value::as_str)
                .is_some_and(|s| set.contains(&s)),
        }
    }
}

/// The one property of a node type whose value settles intent.
#[derive(Debug)]
pub struct SettlingProperty {
    pub node_type: &'static str,
    pub property: &'static str,
    pub when: SettleWhen,
}

/// THE TABLE — the rule's three cases, as `check_intent_authority.py` reads
/// them. `deferred` settles a Decision since 2026-09-12: setting a question
/// aside is the owner's act exactly as accepting one is; superseded and
/// rejected retire rather than decide.
pub const SETTLING: &[SettlingProperty] = &[
    SettlingProperty {
        node_type: node::REQUIREMENT,
        property: "status",
        when: SettleWhen::NotIn(&["proposed"]),
    },
    SettlingProperty {
        node_type: node::DECISION,
        property: "status",
        when: SettleWhen::In(&["accepted", "deferred"]),
    },
    SettlingProperty {
        node_type: node::DESIGN_RULE,
        property: "enforced",
        when: SettleWhen::Present,
    },
];

/// The settling property of `node_type`, if nodes of that type assert intent.
pub fn settling_property(node_type: &str) -> Option<&'static SettlingProperty> {
    SETTLING.iter().find(|s| s.node_type == node_type)
}

/// Does a node of this type holding these properties assert settled intent?
pub fn settles_intent(node_type: &str, props: &HashMap<String, Value>) -> bool {
    settling_property(node_type).is_some_and(|s| s.when.settles(props.get(s.property)))
}

/// A settle a write would make.
#[derive(Clone, Debug, PartialEq)]
pub struct NewSettle {
    pub node_type: String,
    pub id: String,
    pub property: &'static str,
    pub value: Value,
}

impl NewSettle {
    /// `Decision dec:x status=accepted` — what the refusal and the report name.
    pub fn describe(&self) -> String {
        let v = match &self.value {
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            other => format!("{other:?}"),
        };
        format!("{} {} {}={}", self.node_type, self.id, self.property, v)
    }
}

/// Would writing `after` over `stored` SETTLE intent — move the node INTO a
/// settling value, or change the settling value it holds? Re-writing the value
/// it already holds is not a new settle (an edit to an accepted Decision's
/// name needs no second signature); `stored: None` means the node is new.
pub fn newly_settles(
    node_type: &str,
    id: &str,
    stored: Option<&HashMap<String, Value>>,
    after: &HashMap<String, Value>,
) -> Option<NewSettle> {
    let sp = settling_property(node_type)?;
    let new = after.get(sp.property).filter(|v| !v.is_null());
    if !sp.when.settles(new) {
        return None;
    }
    let old = stored
        .and_then(|s| s.get(sp.property))
        .filter(|v| !v.is_null());
    if old == new {
        return None;
    }
    Some(NewSettle {
        node_type: node_type.to_string(),
        id: id.to_string(),
        property: sp.property,
        value: new.cloned().unwrap_or(Value::Null),
    })
}

impl DesignGraph {
    /// Does this node carry the owner's signature — an outgoing
    /// `AUTHORED_BY` whose role set holds `approver`?
    pub fn has_approver(&self, node_id: &str) -> Result<bool, DynoError> {
        Ok(self
            .outgoing(node_id, Some(edge::AUTHORED_BY))?
            .iter()
            .any(|e| authored_roles(e).iter().any(|r| r == "approver")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    #[test]
    fn the_three_cases_read_the_way_the_gate_reads_them() {
        let s = |t: &str, p: &[(&str, Value)]| settles_intent(t, &props(p));
        let st = |v: &str| Value::String(v.into());
        assert!(!s(node::REQUIREMENT, &[]));
        assert!(!s(node::REQUIREMENT, &[("status", st("proposed"))]));
        for v in ["accepted", "deferred", "dropped", "met"] {
            assert!(
                s(node::REQUIREMENT, &[("status", st(v))]),
                "Requirement {v}"
            );
        }
        for (v, want) in [
            ("proposed", false),
            ("accepted", true),
            ("deferred", true),
            ("rejected", false),
            ("superseded", false),
        ] {
            assert_eq!(
                s(node::DECISION, &[("status", st(v))]),
                want,
                "Decision {v}"
            );
        }
        assert!(!s(node::DESIGN_RULE, &[]));
        assert!(s(node::DESIGN_RULE, &[("enforced", Value::Bool(false))]));
        assert!(s(node::DESIGN_RULE, &[("enforced", Value::Bool(true))]));
        assert!(!s(node::CAPABILITY, &[("status", st("realized"))]));
    }

    #[test]
    fn rewriting_the_value_already_held_is_not_a_new_settle() {
        let accepted = props(&[("status", Value::String("accepted".into()))]);
        assert!(newly_settles(node::DECISION, "dec:x", None, &accepted).is_some());
        assert!(newly_settles(node::DECISION, "dec:x", Some(&accepted), &accepted).is_none());
        let deferred = props(&[("status", Value::String("deferred".into()))]);
        assert!(
            newly_settles(node::DECISION, "dec:x", Some(&accepted), &deferred).is_some(),
            "changing one settled value for another is the owner's act too"
        );
        let proposed = props(&[("status", Value::String("proposed".into()))]);
        assert!(newly_settles(node::DECISION, "dec:x", Some(&accepted), &proposed).is_none());
    }
}
