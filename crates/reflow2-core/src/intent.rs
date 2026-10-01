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

// ═══ WHO MAY SIGN — the caller's own signature, checked where it is written ═══
//
// #616 fix 4, under the settled ruling
// `dec:idea-authentication-is-somebody-elses-layer-and-the-line-is-the-contributor-id`
// (option (e), 2026-09-28). A LOCAL engine (stdio, --shared, --http on
// loopback) is unchanged: no policy is installed and every writer behaves as it
// always has. An engine SERVED FOR OTHERS installs one of the two policies below
// for the length of each write, the way `crate::acting` installs the agent:
//
// · the caller is ESTABLISHED (a declared trusted gateway names them on the
//   call, or a verified token does): every AUTHORED_BY the write records,
//   author or approver, is for that contributor, and one naming anyone else is
//   refused;
// · nothing establishes the caller: reads and proposals still work, and no
//   write may sign (add, remove or re-date an approver role) or move a status
//   into settled intent.
//
// ⭐ CHECKED WHERE THE SIGNATURE IS WRITTEN, never per tool. The store has one
// AUTHORED_BY write (`DesignGraph::create_edge_unstamped`, the point #632 stamps
// the acting agent at), one AUTHORED_BY delete (`delete_edge`) and two node
// writes (`create_node`, `create_node_refs_checked_later`), and the policy is
// asked at each. Every door — the typed helper, create_edge, create_edges,
// draw_edges items, acknowledge_gaps items, import, merge, and any door added
// later — goes through them, so none can go round it
// (fact:root-cause-the-settle-rule-guards-the-typed-doors-and-the-generic-writers-go-around-it-2026-09-29,
// "SCOPE WIDENED"). A handler that writes several things asks [`DesignGraph::may_sign`],
// the same rule, BEFORE its first write, so a refusal leaves nothing half-written.

/// Who may sign on the write now in progress — how the engine is served
/// decides it (`reflow2-mcp`'s `caller`), and a local engine installs none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Signer {
    /// The engine established who is calling. Every AUTHORED_BY this write
    /// records names `contributor`.
    Caller {
        /// The Contributor id of the caller.
        contributor: String,
        /// How the engine knows, in a phrase a refusal can quote ("named by
        /// the trusted gateway `flo2.io` on this call").
        how: String,
    },
    /// The engine is served for others and nothing establishes who is
    /// calling: no write may sign or settle.
    Nobody {
        /// Why, and what an operator declares to change it — quoted whole by
        /// every refusal.
        why: String,
    },
}

impl DesignGraph {
    /// Hold every write from here on to `signer`. Ended by [`Self::end_signing`]
    /// when the write that began it ends, so one call's caller never leaks into
    /// the next.
    pub fn begin_signing(&mut self, signer: Signer) {
        self.signer = Some(signer);
    }

    /// Stop holding writes to a signer — the local default.
    pub fn end_signing(&mut self) {
        self.signer = None;
    }

    /// The signer the current write is held to, if the engine installed one.
    pub fn signer(&self) -> Option<&Signer> {
        self.signer.as_ref()
    }

    /// May the current write record `contributor` in `role` on an AUTHORED_BY?
    /// The same rule the store applies at the write, for a handler to ask
    /// BEFORE its first write, so a refusal leaves nothing half-written.
    /// Always `Ok` on a local engine.
    pub fn may_sign(&self, contributor: &str, role: &str) -> Result<(), DynoError> {
        match &self.signer {
            None => Ok(()),
            Some(Signer::Caller {
                contributor: me,
                how,
            }) => {
                if contributor == me {
                    Ok(())
                } else {
                    Err(signed_for_another(
                        me,
                        how,
                        contributor,
                        &[role.to_string()],
                        None,
                    ))
                }
            }
            Some(Signer::Nobody { why }) => {
                if role == "approver" {
                    Err(nobody_signs(
                        why,
                        &format!("record {contributor} as the approver"),
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }

    /// The store's check on one AUTHORED_BY write from `from_id` to `to_id`
    /// whose stored properties will be `after`. Called by the store's only
    /// edge write; a no-op rewrite of an edge exactly as it stands is never a
    /// new signature.
    pub(crate) fn check_signature_write(
        &self,
        from_id: &str,
        to_id: &str,
        after: &HashMap<String, Value>,
    ) -> Result<(), DynoError> {
        let Some(signer) = &self.signer else {
            return Ok(());
        };
        let before = self.stored_authorship(from_id, to_id)?;
        let mut after = after.clone();
        crate::graph::normalize_authored_by_props(&mut after);
        match signer {
            Signer::Caller { contributor, how } => {
                if to_id == contributor || before.as_ref() == Some(&after) {
                    return Ok(());
                }
                let roles = crate::graph::list_of_strings(after.get("roles"));
                Err(signed_for_another(
                    contributor,
                    how,
                    to_id,
                    &roles,
                    Some(from_id),
                ))
            }
            Signer::Nobody { why } => {
                let approval = |p: Option<&HashMap<String, Value>>| {
                    p.map(|p| {
                        (
                            crate::graph::list_of_strings(p.get("roles"))
                                .iter()
                                .any(|r| r == "approver"),
                            p.get(crate::graph::role_date_key("approver")).cloned(),
                        )
                    })
                    .unwrap_or((false, None))
                };
                if approval(before.as_ref()) == approval(Some(&after)) {
                    return Ok(());
                }
                Err(nobody_signs(
                    why,
                    &format!("change the approval {to_id} carries on '{from_id}'"),
                ))
            }
        }
    }

    /// The store's check on deleting the AUTHORED_BY from `from_id` to
    /// `to_id` — removing a signature is writing one. Called by `delete_edge`.
    pub(crate) fn check_signature_removal(
        &self,
        from_id: &str,
        to_id: &str,
    ) -> Result<(), DynoError> {
        let Some(signer) = &self.signer else {
            return Ok(());
        };
        let Some(before) = self.stored_authorship(from_id, to_id)? else {
            return Ok(());
        };
        let roles = crate::graph::list_of_strings(before.get("roles"));
        match signer {
            Signer::Caller { contributor, how } if to_id != contributor => {
                Err(DynoError::Validation {
                    node_type: edge::AUTHORED_BY.into(),
                    property: "to_id".into(),
                    message: format!(
                        "REFUSED, and nothing was written: this call is {contributor}'s ({how}), and it \
                     would REMOVE {to_id}'s AUTHORED_BY on '{from_id}' ({}). On a server that \
                     establishes who is calling, a signature is the signer's own to write and to \
                     withdraw: {to_id} withdraws it through their own sign-in. Your own AUTHORED_BY \
                     ({contributor}) is yours to remove.",
                        roles.join(", ")
                    ),
                })
            }
            Signer::Nobody { why } if roles.iter().any(|r| r == "approver") => Err(nobody_signs(
                why,
                &format!("remove {to_id}'s approval of '{from_id}'"),
            )),
            _ => Ok(()),
        }
    }

    /// The store's check on a node write: with nobody established, no value
    /// may move INTO settled intent ([`newly_settles`]). With a caller
    /// established the settle stands on its signature, which is checked where
    /// it is written.
    pub(crate) fn check_settle_write(
        &self,
        node_type: &str,
        id: &str,
        after: &HashMap<String, Value>,
    ) -> Result<(), DynoError> {
        let Some(Signer::Nobody { why }) = &self.signer else {
            return Ok(());
        };
        if settling_property(node_type).is_none() {
            return Ok(());
        }
        let stored = self.get_node(node_type, id)?.map(|n| n.properties);
        match newly_settles(node_type, id, stored.as_ref(), after) {
            None => Ok(()),
            Some(settle) => Err(nobody_signs(
                why,
                &format!("settle intent ({})", settle.describe()),
            )),
        }
    }

    /// The AUTHORED_BY from `from_id` to `to_id` as it stands, in the set
    /// shape; `None` when there is none.
    fn stored_authorship(
        &self,
        from_id: &str,
        to_id: &str,
    ) -> Result<Option<HashMap<String, Value>>, DynoError> {
        Ok(self
            .outgoing(from_id, Some(edge::AUTHORED_BY))?
            .into_iter()
            .find(|e| e.to_id == to_id)
            .map(|e| {
                let mut p = e.properties;
                crate::graph::normalize_authored_by_props(&mut p);
                p
            }))
    }
}

/// The refusal for a signature in someone else's name. It says who the
/// caller is, how the engine knows, and how an owner signs.
fn signed_for_another(
    me: &str,
    how: &str,
    named: &str,
    roles: &[String],
    on: Option<&str>,
) -> DynoError {
    let what = match on {
        Some(from) => format!("an AUTHORED_BY on '{from}' naming {named}"),
        None => format!("an AUTHORED_BY naming {named}"),
    };
    let roles = if roles.is_empty() {
        String::new()
    } else {
        format!(" ({})", roles.join(", "))
    };
    DynoError::Validation {
        node_type: edge::AUTHORED_BY.into(),
        property: "contributor".into(),
        message: format!(
            "REFUSED, and nothing was written: this call is {me}'s ({how}), and it would record \
             {what}{roles} — a signature in someone else's name. On a server that establishes \
             who is calling, every AUTHORED_BY a call writes, author or approver, is the \
             caller's own. To sign this yourself, name {me}. For {named}'s signature, {named} \
             makes the call through their own sign-in; nobody signs for them here."
        ),
    }
}

/// The refusal when nothing establishes who is calling.
fn nobody_signs(why: &str, what: &str) -> DynoError {
    DynoError::Validation {
        node_type: edge::AUTHORED_BY.into(),
        property: "approver".into(),
        message: format!("REFUSED, and nothing was written: this call would {what}, and {why}"),
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
