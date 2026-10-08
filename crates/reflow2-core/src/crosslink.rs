//! A relation to a node in ANOTHER design, recorded in the design that makes it
//! (`req:a-relation-between-nodes-in-two-designs-is-recorded-in-the-design-that-makes-it`,
//! Anthony's pick 2026-10-07).
//!
//! An edge cannot cross a store, so the far node is stood in for by a small
//! REFERENCE: a Resource of kind `design-node-reference`, id
//! `xref:<design>:<node>`, naming the other design, the node there and, when
//! this design pins that design with a baseline, the fingerprint the far design
//! had when the link was made. The relation itself is an ordinary review-relation
//! edge to the reference, so every reader of edges (search's `linked`, the
//! ripple, review_relations' echo) sees it without learning a new shape.
//!
//! ⚠️ WHY A RESOURCE AND NOT A STAND-IN OF THE FAR NODE'S OWN TYPE. A stand-in
//! Requirement would be read by every detector as one of this design's own
//! requirements (unsatisfied, unverified), because nothing skips imported nodes.
//! A Resource is an external thing by definition. The cost: DECOMPOSES is
//! Requirement-to-Requirement in the schema, so decomposition across designs is
//! not expressible this way yet (measured 2026-10-07: refused); the review
//! relations all are.
//!
//! No clock: re-linking is the acknowledgement, exactly as re-declaring a pin
//! re-takes its baseline (`dec:ask-not-repair`).

use std::collections::HashMap;

use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::nodes::node;

/// The `resource_type` of a reference to a node in another design.
pub const DESIGN_NODE_REFERENCE: &str = "design-node-reference";

/// The id of the reference standing in for `node_id` in `design`.
pub fn reference_id(design: &str, node_id: &str) -> String {
    format!("xref:{design}:{node_id}")
}

/// A reference to a node in another design, with the links that point at it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DesignReference {
    /// The reference's own id here (`xref:<design>:<node>`).
    pub id: String,
    /// The other design's id.
    pub design: String,
    /// The node's id in that design.
    pub node_id: String,
    /// Its type there, when the linker said.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_type: Option<String>,
    /// Its name, as the linker gave it, or its id.
    pub name: String,
    /// The fingerprint of the other design, as this design's pin of it last
    /// recorded, when the link was made. Absent when nothing here watched that
    /// design at the time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint_at_link: Option<String>,
    /// The relations drawn between this design's nodes and the reference.
    pub links: Vec<ReferenceLink>,
}

/// One relation between a node here and a reference.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReferenceLink {
    /// The node here.
    pub node_id: String,
    /// The relation.
    pub relation: String,
    /// `out`: *here RELATION there*. `in`: *there RELATION here*.
    pub direction: &'static str,
}

fn text(props: &HashMap<String, Value>, k: &str) -> Option<String> {
    props
        .get(k)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string)
}

impl DesignGraph {
    /// Make sure a reference to `node_id` in `design` exists, and return its
    /// id. Calling it again refreshes the fingerprint: that is how a link is
    /// acknowledged after the far design moved.
    pub fn ensure_design_reference(
        &mut self,
        design: &str,
        node_id: &str,
        node_type: Option<&str>,
        name: Option<&str>,
    ) -> Result<String, DynoError> {
        let design = design.trim();
        let node_id = node_id.trim();
        if design.is_empty() || node_id.is_empty() {
            return Err(DynoError::Validation {
                node_type: node::RESOURCE.into(),
                property: "other_design".into(),
                message: "a link into another design needs that design's id and the node's id \
                          there"
                    .into(),
            });
        }
        if design == self.graph_id() {
            return Err(DynoError::Validation {
                node_type: node::RESOURCE.into(),
                property: "other_design".into(),
                message: format!(
                    "'{design}' is THIS design. Link to {node_id} directly, without \
                     other_design: a reference stands in only for a node another design holds"
                ),
            });
        }
        let id = reference_id(design, node_id);
        let fingerprint = self
            .declared_dependencies()?
            .into_iter()
            .find(|d| d.graph_id.as_deref() == Some(design))
            .and_then(|d| d.design_address_hash.or(d.design_export_hash))
            .filter(|h| !h.trim().is_empty());
        let mut props: HashMap<String, Value> = self
            .get_node(node::RESOURCE, &id)?
            .map(|n| n.properties)
            .unwrap_or_default();
        let shown = name
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .or_else(|| text(&props, "name"))
            .unwrap_or_else(|| format!("{node_id} (in design {design})"));
        props.insert("name".into(), Value::from(shown.as_str()));
        props.insert("resource_type".into(), Value::from(DESIGN_NODE_REFERENCE));
        props.insert("design_graph_id".into(), Value::from(design));
        props.insert("design_node_id".into(), Value::from(node_id));
        if let Some(t) = node_type.map(str::trim).filter(|t| !t.is_empty()) {
            props.insert("design_node_type".into(), Value::from(t));
        }
        match fingerprint {
            Some(f) => {
                props.insert("design_fingerprint_at_link".into(), Value::from(f.as_str()));
            }
            None => {
                props.remove("design_fingerprint_at_link");
            }
        }
        self.create_node(node::RESOURCE, &id, props)?;
        Ok(id)
    }

    /// Every reference to a node in another design, with its links.
    pub fn design_references(&self) -> Result<Vec<DesignReference>, DynoError> {
        let mut out = Vec::new();
        for n in self.scan_nodes(node::RESOURCE)? {
            if text(&n.properties, "resource_type").as_deref() != Some(DESIGN_NODE_REFERENCE) {
                continue;
            }
            let mut links = Vec::new();
            // The review relations, plus the two slice 3 draws across designs:
            // a piece DECOMPOSES its parent's requirement, a derived requirement
            // is GOVERNED_BY the decision that forced it.
            let kinds = crate::relate::REVIEW_RELATIONS.iter().copied().chain([
                crate::nodes::edge::DECOMPOSES,
                crate::nodes::edge::GOVERNED_BY,
            ]);
            for r in kinds {
                for e in self.incoming(&n.node_id, Some(r))? {
                    links.push(ReferenceLink {
                        node_id: e.from_id,
                        relation: r.to_string(),
                        direction: "out",
                    });
                }
                for e in self.outgoing(&n.node_id, Some(r))? {
                    links.push(ReferenceLink {
                        node_id: e.to_id,
                        relation: r.to_string(),
                        direction: "in",
                    });
                }
            }
            out.push(DesignReference {
                design: text(&n.properties, "design_graph_id").unwrap_or_default(),
                node_id: text(&n.properties, "design_node_id").unwrap_or_default(),
                node_type: text(&n.properties, "design_node_type"),
                name: text(&n.properties, "name").unwrap_or_else(|| n.node_id.clone()),
                fingerprint_at_link: text(&n.properties, "design_fingerprint_at_link"),
                id: n.node_id,
                links,
            });
        }
        out.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(out)
    }

    /// The other design and node a reference stands in for, if `id` is one.
    pub fn reference_target(&self, id: &str) -> Result<Option<(String, String)>, DynoError> {
        if !id.starts_with("xref:") {
            return Ok(None);
        }
        let Some(n) = self.get_node(node::RESOURCE, id)? else {
            return Ok(None);
        };
        if text(&n.properties, "resource_type").as_deref() != Some(DESIGN_NODE_REFERENCE) {
            return Ok(None);
        }
        Ok(text(&n.properties, "design_graph_id").zip(text(&n.properties, "design_node_id")))
    }
}
