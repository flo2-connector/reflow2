//! Which AGENT a write went through, recorded beside the contributor it was
//! for — the ACTS_FOR rung.
//!
//! `req:a-write-and-an-approval-record-the-agent-and-the-person-it-acts-for`
//! (accepted, Anthony 2026-09-29, idea 1 of the two-agent exercise). The rung
//! `dec:design-authorship-identity` named on 2026-07-22 as git's
//! author/committer split and deferred: until it existed, a decision an agent
//! recorded under a person's delegation could name the person or the agent,
//! never both (fact:root-cause-an-agent-deciding-for-its-principal-has-no-record-because-the-acts-for-rung-was-deferred-2026-09-29).
//!
//! THE SHAPE, chosen over the brainstorm's alternatives
//! (dec:idea-an-agent-acting-for-a-person-is-recorded-as-both):
//!   · the agent is a `Contributor` of kind `automated_agent`, named by the
//!     caller — never minted from a client string;
//!   · every AUTHORED_BY role the write records gets the agent added to that
//!     role's `authored_via` / `reviewed_via` / `approved_via` SET, beside the
//!     contributor the edge points at (the author/approver of record);
//!   · the first such record draws `agent ACTS_FOR contributor`, carrying how
//!     the agent was named (`route`).
//!
//! It is stamped at ONE place, [`DesignGraph::authored_by`], which every
//! authorship and approval passes through, while [`DesignGraph::begin_acting`]
//! is in force — the same pattern as `crate::attribution`'s touch log, so no
//! handler has to remember it.
//!
//! 🛑 ATTRIBUTION, NEVER AUTHORITY. The agent's identity is self-declared by
//! whoever connected. It never signs anything: an approval still needs its
//! explicit `approver`, and naming an agent changes nothing about what a call
//! may settle. `rule:design-intent-moves-only-on-the-owners-word` is untouched.

use crate::DesignGraph;
use crate::foundation::core::{DynoError, Value};
use crate::nodes::{Props, edge, node};

/// How the agent of a write was named.
pub const ROUTES: [&str; 3] = ["session", "request", "client"];

/// The agent a write goes through, and how it was named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Acting {
    /// The `Contributor` id of the agent (kind `automated_agent`).
    pub agent: String,
    /// One of [`ROUTES`].
    pub route: String,
}

impl DesignGraph {
    /// Whether `agent_id` may be named as the agent a write goes through: it
    /// must already be a Contributor, and of kind `automated_agent` — a person
    /// named here would be recorded as having carried somebody else's word,
    /// which is a claim about them nobody made. Never invents one.
    pub fn require_acting_agent(&self, agent_id: &str) -> Result<(), DynoError> {
        self.require_contributor(agent_id, "acting_agent", "name as the acting agent")?;
        let kind = self
            .get_node(node::CONTRIBUTOR, agent_id)?
            .and_then(|n| {
                n.properties
                    .get("kind")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| "person".to_string());
        if kind != "automated_agent" {
            return Err(DynoError::Validation {
                node_type: node::CONTRIBUTOR.into(),
                property: "acting_agent".into(),
                message: format!(
                    "'{agent_id}' is a Contributor of kind `{kind}`, and the acting agent must be \
                     one of kind `automated_agent`: it is recorded as the agent that CARRIED a \
                     person's word, which is not a thing to say about a person. Name the agent's \
                     own Contributor (`add_contributor` with kind `automated_agent` first), or \
                     name nobody."
                ),
            });
        }
        Ok(())
    }

    /// The `automated_agent` Contributor whose `handle` equals `client_name` —
    /// how a local session's agent is found from the name its client gave at
    /// handshake. MATCHED, NEVER MINTED: `None` when no such Contributor
    /// exists (or several do), and nothing is created.
    pub fn agent_for_client(&self, client_name: &str) -> Result<Option<String>, DynoError> {
        let want = client_name.trim();
        if want.is_empty() {
            return Ok(None);
        }
        let mut found: Option<String> = None;
        for c in self.scan_nodes(node::CONTRIBUTOR)? {
            let p = &c.properties;
            let is_agent = p.get("kind").and_then(Value::as_str) == Some("automated_agent");
            let handle = p.get("handle").and_then(Value::as_str).map(str::trim);
            if is_agent && handle == Some(want) {
                if found.is_some() {
                    // Two agents claim the same client name: which one acted
                    // cannot be told, so neither is recorded.
                    return Ok(None);
                }
                found = Some(c.node_id.clone());
            }
        }
        Ok(found)
    }

    /// Record every authorship and approval from here on as going through
    /// `acting.agent`. Checked like any other naming of a contributor.
    pub fn begin_acting(&mut self, acting: Acting) -> Result<(), DynoError> {
        self.require_acting_agent(&acting.agent)?;
        self.acting = Some(acting);
        Ok(())
    }

    /// Stop recording an agent. Always called when the write that began it
    /// ends, so one call's agent never leaks into the next.
    pub fn end_acting(&mut self) {
        self.acting = None;
    }

    /// The agent the current write goes through, if one was named.
    pub fn acting(&self) -> Option<&Acting> {
        self.acting.as_ref()
    }

    /// For a write of `edge_type` from `from_id` to `to_id` while an agent is
    /// named: when it is an AUTHORED_BY edge, add the agent to the `*_via` set
    /// of every role the write newly asserts (a role the edge did not hold,
    /// or one whose date moves), and return the agent so the caller draws
    /// ACTS_FOR once the edge is stored. `None` when nothing was stamped —
    /// another edge type, no agent named, or the agent writing for itself.
    pub(crate) fn stamp_through_acting_agent(
        &self,
        edge_type: &str,
        from_id: &str,
        to_id: &str,
        props: &mut std::collections::HashMap<String, Value>,
    ) -> Result<Option<Acting>, DynoError> {
        if edge_type != edge::AUTHORED_BY {
            return Ok(None);
        }
        let Some(acting) = self.acting.as_ref().filter(|a| a.agent != to_id).cloned() else {
            return Ok(None);
        };
        let existing = self
            .outgoing(from_id, Some(edge::AUTHORED_BY))?
            .into_iter()
            .find(|e| e.to_id == to_id)
            .map(|e| {
                let mut p = e.properties;
                crate::graph::normalize_authored_by_props(&mut p);
                p
            });
        let mut asserted = props.clone();
        crate::graph::normalize_authored_by_props(&mut asserted);
        let mut stamped = false;
        for role in crate::graph::list_of_strings(asserted.get("roles")) {
            let date = crate::graph::role_date_key(&role);
            let is_new = existing.as_ref().is_none_or(|p| {
                !crate::graph::list_of_strings(p.get("roles")).contains(&role)
                    || (asserted.contains_key(date) && asserted.get(date) != p.get(date))
            });
            if is_new {
                crate::graph::stamp_via(props, &role, &acting.agent);
                stamped = true;
            }
        }
        Ok(stamped.then_some(acting))
    }

    /// Draw `agent ACTS_FOR contributor` if it is not there yet. The first
    /// route that drew it is kept: a later write through another route adds
    /// nothing a reader needs.
    pub(crate) fn record_acts_for(
        &mut self,
        acting: &Acting,
        contributor_id: &str,
    ) -> Result<(), DynoError> {
        let exists = self
            .outgoing(&acting.agent, Some(edge::ACTS_FOR))?
            .iter()
            .any(|e| e.to_id == contributor_id);
        if exists {
            return Ok(());
        }
        self.create_edge(
            edge::ACTS_FOR,
            node::CONTRIBUTOR,
            &acting.agent,
            node::CONTRIBUTOR,
            contributor_id,
            Props::new().set("route", acting.route.as_str()),
        )?;
        Ok(())
    }
}
