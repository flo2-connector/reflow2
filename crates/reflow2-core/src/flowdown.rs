//! Intent sent from one design to another — a parent's to its part, or a
//! customer's to a supplier — and received there (slice 3 of the hub work,
//! `req:a-requirement-reaches-a-design-its-sender-does-not-own-at-proposed-with-its-origin-and-binds-on-the-receivers-word`).
//!
//! Settled by Anthony, 2026-10-07 and 08: nothing is COPIED.
//! - **moved**: a requirement wholly the receiver's leaves the sender. The
//!   sender's node is replaced by a reference to its new home (its ending kept
//!   as history), and the move is refused while a capability in the sender still
//!   satisfies it.
//! - **piece**: a requirement spanning several receivers stays with the sender;
//!   each receiver holds its own piece, DECOMPOSES-linked across designs.
//! - **derived**: a decision stays with the sender; what a receiver must do
//!   because of it is a DERIVED requirement there, GOVERNED_BY the decision.
//!
//! What lands in the receiver is `proposed`, attributed to the sender, and only
//! the receiver's owner moves it. An edge cannot cross a store, so each side is a
//! separate write in its own design (`receive_from_design` there,
//! `send_to_design` here); the agent makes both, and each side's references
//! (`crate::crosslink`) say where the other half lives.

use std::collections::HashMap;

use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::nodes::{edge, node};

/// The three ways intent travels.
pub const SEND_KINDS: [&str; 3] = ["moved", "piece", "derived"];

/// A requirement this design holds because another design sent it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReceivedIntent {
    pub requirement_id: String,
    pub name: String,
    /// `proposed` until this design's owner accepts or drops it.
    pub status: String,
    pub kind: String,
    /// The sending design and the node there.
    pub from_design: String,
    pub from_node_id: String,
}

/// Something this design sent to another.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SentIntent {
    /// The node here it came from (for a move, the node it replaced).
    pub node_id: String,
    pub kind: String,
    /// The receiving design and the node there.
    pub to_design: String,
    pub to_node_id: String,
    /// The reference here that stands in for the node there.
    pub reference: String,
}

fn text(props: &HashMap<String, Value>, k: &str) -> Option<String> {
    props
        .get(k)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
}

fn check_kind(kind: &str) -> Result<(), DynoError> {
    if SEND_KINDS.contains(&kind) {
        return Ok(());
    }
    Err(DynoError::Validation {
        node_type: node::REQUIREMENT.into(),
        property: "kind".into(),
        message: format!(
            "{kind:?} is not a way intent travels. `moved`: a requirement wholly the receiver's; \
             `piece`: the receiver's part of a requirement that spans several; `derived`: what \
             the receiver must do because of a decision"
        ),
    })
}

impl DesignGraph {
    /// In the RECEIVING design: hold intent another design sent, at `proposed`.
    /// Refused when the requirement already exists here and is no longer
    /// `proposed`: once this design's owner has ruled on it, a sender cannot
    /// rewrite it.
    #[allow(clippy::too_many_arguments)]
    pub fn receive_from_design(
        &mut self,
        id: &str,
        name: &str,
        statement: &str,
        from_design: &str,
        from_node_id: &str,
        from_name: Option<&str>,
        kind: &str,
        sender: Option<&str>,
    ) -> Result<ReceivedIntent, DynoError> {
        check_kind(kind)?;
        if let Some(existing) = self.get_node(node::REQUIREMENT, id)? {
            let status = text(&existing.properties, "status").unwrap_or_default();
            if !status.is_empty() && status != "proposed" {
                return Err(DynoError::Validation {
                    node_type: node::REQUIREMENT.into(),
                    property: "status".into(),
                    message: format!(
                        "'{id}' is already {status} here: this design's owner has ruled on it, \
                         so a sender cannot rewrite it. Send it as a new requirement, or ask the \
                         owner to reopen it"
                    ),
                });
            }
        }
        let from_type = if kind == "derived" {
            node::DECISION
        } else {
            node::REQUIREMENT
        };
        let reference =
            self.ensure_design_reference(from_design, from_node_id, Some(from_type), from_name)?;
        self.add_requirement(id, name, statement)?;
        let lineage = match kind {
            "piece" => "decomposed",
            "derived" => "derived",
            _ => "original",
        };
        let mut props: HashMap<String, Value> = self
            .get_node(node::REQUIREMENT, id)?
            .map(|n| n.properties)
            .unwrap_or_default();
        props.insert("status".into(), Value::from("proposed"));
        props.insert("provenance".into(), Value::from("imported"));
        props.insert("lineage".into(), Value::from(lineage));
        props.insert("received_from_design".into(), Value::from(from_design));
        props.insert("received_from_node".into(), Value::from(from_node_id));
        props.insert("received_kind".into(), Value::from(kind));
        self.create_node(node::REQUIREMENT, id, props)?;
        match kind {
            "piece" => {
                self.create_edge(
                    edge::DECOMPOSES,
                    node::REQUIREMENT,
                    id,
                    node::RESOURCE,
                    &reference,
                    HashMap::new(),
                )?;
            }
            "derived" => {
                self.create_edge(
                    edge::GOVERNED_BY,
                    node::REQUIREMENT,
                    id,
                    node::RESOURCE,
                    &reference,
                    HashMap::new(),
                )?;
            }
            _ => {}
        }
        if let Some(who) = sender.map(str::trim).filter(|s| !s.is_empty()) {
            self.authored_by(node::REQUIREMENT, id, who, Some("author"), None)?;
        }
        Ok(ReceivedIntent {
            requirement_id: id.to_string(),
            name: name.to_string(),
            status: "proposed".into(),
            kind: kind.to_string(),
            from_design: from_design.to_string(),
            from_node_id: from_node_id.to_string(),
        })
    }

    /// In the SENDING design: record what was sent, and for a move, hand the
    /// requirement over. `approver` is this design's owner, whose word a move
    /// needs because it settles the requirement here (it leaves this design).
    pub fn send_to_design(
        &mut self,
        node_id: &str,
        to_design: &str,
        to_node_id: &str,
        to_name: Option<&str>,
        kind: &str,
        approver: Option<&str>,
    ) -> Result<SentIntent, DynoError> {
        check_kind(kind)?;
        let node_type = if kind == "derived" {
            node::DECISION
        } else {
            node::REQUIREMENT
        };
        let Some(sent) = self.get_node(node_type, node_id)? else {
            return Err(DynoError::NodeNotFound {
                node_type: node_type.into(),
                node_id: node_id.into(),
            });
        };
        if kind == "moved" {
            let satisfiers: Vec<String> = self
                .incoming(node_id, Some(edge::SATISFIES))?
                .into_iter()
                .map(|e| e.from_id)
                .collect();
            if !satisfiers.is_empty() {
                return Err(DynoError::Validation {
                    node_type: node::REQUIREMENT.into(),
                    property: "kind".into(),
                    message: format!(
                        "'{node_id}' is still satisfied here by {}, so it is not wholly the \
                         receiver's. Re-home those capabilities first, or send it as a `piece` \
                         and keep it here",
                        satisfiers.join(", ")
                    ),
                });
            }
            if approver.map(str::trim).filter(|a| !a.is_empty()).is_none() {
                return Err(DynoError::Validation {
                    node_type: node::REQUIREMENT.into(),
                    property: "approver".into(),
                    message: "moving a requirement out of this design settles it here (it \
                              leaves), so it takes this design owner's word: name them in \
                              `approver`"
                        .into(),
                });
            }
        }
        let shown = to_name
            .map(str::to_string)
            .or_else(|| text(&sent.properties, "name"));
        let reference = self.ensure_design_reference(
            to_design,
            to_node_id,
            Some(node::REQUIREMENT),
            shown.as_deref(),
        )?;
        let mut rprops = self
            .get_node(node::RESOURCE, &reference)?
            .map(|n| n.properties)
            .unwrap_or_default();
        rprops.insert("sent_kind".into(), Value::from(kind));
        rprops.insert("sent_from_node".into(), Value::from(node_id));
        self.create_node(node::RESOURCE, &reference, rprops)?;
        match kind {
            "moved" => {
                self.create_edge(
                    edge::OBSOLETES,
                    node::RESOURCE,
                    &reference,
                    node::REQUIREMENT,
                    node_id,
                    HashMap::from([(
                        "evidence".to_string(),
                        Value::from(format!("moved to {to_node_id} in design {to_design}")),
                    )]),
                )?;
                let mut props = sent.properties.clone();
                props.insert("status".into(), Value::from("dropped"));
                props.insert("moved_to_design".into(), Value::from(to_design));
                props.insert("moved_to_node".into(), Value::from(to_node_id));
                self.create_node(node::REQUIREMENT, node_id, props)?;
                if let Some(a) = approver {
                    self.authored_by(node::REQUIREMENT, node_id, a.trim(), Some("approver"), None)?;
                }
            }
            "piece" => {
                self.create_edge(
                    edge::DECOMPOSES,
                    node::RESOURCE,
                    &reference,
                    node::REQUIREMENT,
                    node_id,
                    HashMap::new(),
                )?;
            }
            _ => {
                self.create_edge(
                    edge::GOVERNED_BY,
                    node::RESOURCE,
                    &reference,
                    node::DECISION,
                    node_id,
                    HashMap::new(),
                )?;
            }
        }
        Ok(SentIntent {
            node_id: node_id.to_string(),
            kind: kind.to_string(),
            to_design: to_design.to_string(),
            to_node_id: to_node_id.to_string(),
            reference,
        })
    }

    /// Every requirement here that another design sent.
    pub fn received_intents(&self) -> Result<Vec<ReceivedIntent>, DynoError> {
        let mut out = Vec::new();
        for n in self.scan_nodes(node::REQUIREMENT)? {
            let Some(from_design) = text(&n.properties, "received_from_design") else {
                continue;
            };
            out.push(ReceivedIntent {
                name: text(&n.properties, "name").unwrap_or_default(),
                status: text(&n.properties, "status").unwrap_or_else(|| "proposed".into()),
                kind: text(&n.properties, "received_kind").unwrap_or_default(),
                from_node_id: text(&n.properties, "received_from_node").unwrap_or_default(),
                from_design,
                requirement_id: n.node_id,
            });
        }
        out.sort_by(|a, b| a.requirement_id.cmp(&b.requirement_id));
        Ok(out)
    }

    /// Everything this design sent to another.
    pub fn sent_intents(&self) -> Result<Vec<SentIntent>, DynoError> {
        let mut out = Vec::new();
        for n in self.scan_nodes(node::RESOURCE)? {
            let Some(kind) = text(&n.properties, "sent_kind") else {
                continue;
            };
            out.push(SentIntent {
                node_id: text(&n.properties, "sent_from_node").unwrap_or_default(),
                kind,
                to_design: text(&n.properties, "design_graph_id").unwrap_or_default(),
                to_node_id: text(&n.properties, "design_node_id").unwrap_or_default(),
                reference: n.node_id,
            });
        }
        out.sort_by(|a, b| a.reference.cmp(&b.reference));
        Ok(out)
    }
}
