//! Stored twins: a relation reflow2 holds BOTH as a node property and as an
//! edge, with the property declared the authority and the edge its derived
//! copy (`req:a-relation-stored-in-more-than-one-place-has-one-authoritative-copy-and-no-copy-drifts-unnoticed`).
//!
//! ⭐ WHY THE STORE KEEPS THE COPY, NOT EACH WRITER. Measured 2026-09-28 on
//! reflow2's own design: 354 of 763 findings carried a `subject_id` and no
//! `HAS_TEMPORAL_FACT` edge, 4 hung from a different node than their
//! `subject_id`, and a Flow's stored exit named step 8 of 9. The root cause
//! (`fact:root-cause-each-writer-draws-the-second-copy-by-hand-and-three-writers-never-did-2026-09-28`)
//! was that each writer drew the second copy by hand: `record_finding` did,
//! the generic `create_node` and `report_manual_work` never did, and adding a
//! step never touched the stored exit. Fixing any one writer leaves the next
//! one to forget. So the schema declares the twin ([`TwinOf`]) and the three
//! places every write passes keep it:
//!
//! - [`DesignGraph::create_node`] keeps the derived edge on every node write
//!   ([`DesignGraph::keep_twins_on_write`]).
//! - Open and import run [`DesignGraph::repair_stored_twins`], and REPORT
//!   what they changed. Never silently, because a repair nobody is told about
//!   is the drift this exists to end.
//! - The generic edge tools ask [`DesignGraph::twin_edge_refusal`] before
//!   drawing or deleting a derived edge by hand.
//!
//! A Flow's entry and exit went the other way. They are COMPUTED from step
//! order and no longer stored
//! (`dec:a-flows-order-is-its-step-order-and-entry-and-exit-are-computed`),
//! so their stored copies are [`RETIRED_PROPERTIES`] the repair removes.

use std::collections::HashMap;

use serde::Serialize;

use crate::foundation::core::{DynoError, TwinEnd, TwinOf, Value};
use crate::foundation::store::StoredEdge;
use crate::graph::DesignGraph;
use crate::nodes::{Props, node};

/// One declared twin: the authoritative property on a node type, and the edge
/// that is its derived copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredTwin {
    pub node_type: String,
    pub property: String,
    #[serde(flatten)]
    pub twin: TwinOf,
}

/// Properties that were a stored second copy of something now computed, with
/// the ruling that retired each. The repair removes any a design still holds
/// and says so; the schema no longer declares them, so nothing writes them.
pub const RETIRED_PROPERTIES: &[(&str, &str, &str)] = &[
    (
        node::FLOW,
        "entry_point",
        "dec:a-flows-order-is-its-step-order-and-entry-and-exit-are-computed",
    ),
    (
        node::FLOW,
        "exit_point",
        "dec:a-flows-order-is-its-step-order-and-entry-and-exit-are-computed",
    ),
    (
        node::CAPABILITY,
        "is_entry_point",
        "dec:a-flows-order-is-its-step-order-and-entry-and-exit-are-computed",
    ),
    (
        node::CAPABILITY,
        "is_exit_point",
        "dec:a-flows-order-is-its-step-order-and-entry-and-exit-are-computed",
    ),
];

/// What a repair changed, one line per change, so a reader can check each.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TwinRepairs {
    /// Derived edges that were missing and are now drawn from their property.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub added: Vec<String>,
    /// Edges that joined a node to something its property does NOT name, and
    /// became the edge the twin declares for that (`strays_become`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub moved: Vec<String>,
    /// Edges that disagreed with their property and have no other home.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub removed: Vec<String>,
    /// Stored copies of values that are now computed, removed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub retired_properties: Vec<String>,
}

impl TwinRepairs {
    /// Nothing was out of step.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.moved.is_empty()
            && self.removed.is_empty()
            && self.retired_properties.is_empty()
    }

    /// How many repairs, of every kind.
    pub fn total(&self) -> usize {
        self.added.len() + self.moved.len() + self.removed.len() + self.retired_properties.len()
    }

    /// One line a person can read, or `None` when nothing was repaired.
    pub fn summary(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }
        Some(format!(
            "repaired {} relation(s) stored twice so the copy matches its authority: \
             {} derived edge(s) added, {} moved to the edge that means it, {} removed, \
             {} retired stored value(s) removed",
            self.total(),
            self.added.len(),
            self.moved.len(),
            self.removed.len(),
            self.retired_properties.len()
        ))
    }
}

/// The far end of a twin edge: the node the property names.
fn far_end(e: &StoredEdge, names: TwinEnd) -> &str {
    match names {
        TwinEnd::Source => &e.from_id,
        TwinEnd::Target => &e.to_id,
    }
}

fn string_prop<'a>(props: &'a HashMap<String, Value>, key: &str) -> Option<&'a str> {
    props
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

impl DesignGraph {
    /// Every twin the schema declares, sorted by node type then property.
    pub fn stored_twins(&self) -> Vec<StoredTwin> {
        let mut out: Vec<StoredTwin> = self
            .schema()
            .node_types
            .iter()
            .flat_map(|(ty, def)| {
                def.properties.iter().filter_map(move |(prop, d)| {
                    d.twin_of.clone().map(|twin| StoredTwin {
                        node_type: ty.clone(),
                        property: prop.clone(),
                        twin,
                    })
                })
            })
            .collect();
        out.sort_by(|a, b| {
            a.node_type
                .cmp(&b.node_type)
                .then(a.property.cmp(&b.property))
        });
        out
    }

    fn twins_of_type(&self, node_type: &str) -> Vec<(String, TwinOf)> {
        let Some(def) = self.schema().node_types.get(node_type) else {
            return Vec::new();
        };
        let mut out: Vec<(String, TwinOf)> = def
            .properties
            .iter()
            .filter_map(|(p, d)| d.twin_of.clone().map(|t| (p.clone(), t)))
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    /// Does `node_type` declare any twin? Asked on every node write, so it
    /// is a lookup, not a scan.
    pub(crate) fn declares_twins(&self, node_type: &str) -> bool {
        self.schema()
            .node_types
            .get(node_type)
            .is_some_and(|d| d.properties.values().any(|p| p.twin_of.is_some()))
    }

    /// The type of the node `id` names. Read from the sibling type property
    /// when the twin declares one; otherwise each declared type is asked in
    /// turn, the walk the node-reference guard makes. `None` when nothing
    /// holds the id.
    fn resolve_named(
        &self,
        props: &HashMap<String, Value>,
        twin: &TwinOf,
        id: &str,
    ) -> Result<Option<String>, DynoError> {
        if let Some(t) = twin
            .type_property
            .as_deref()
            .and_then(|tp| string_prop(props, tp))
            && self.get_node(t, id)?.is_some()
        {
            return Ok(Some(t.to_string()));
        }
        let mut types: Vec<String> = self.schema().node_types.keys().cloned().collect();
        types.sort();
        for t in types {
            if self.get_node(&t, id)?.is_some() {
                return Ok(Some(t));
            }
        }
        Ok(None)
    }

    /// The edges of the twin's type at the property's node: incoming when the
    /// property names the source, outgoing when it names the target.
    fn twin_edges_at(&self, owner_id: &str, twin: &TwinOf) -> Result<Vec<StoredEdge>, DynoError> {
        match twin.names {
            TwinEnd::Source => self.incoming(owner_id, Some(&twin.edge)),
            TwinEnd::Target => self.outgoing(owner_id, Some(&twin.edge)),
        }
    }

    /// Draw the derived edge between the property's node and the node it names.
    fn draw_twin(
        &mut self,
        twin: &TwinOf,
        owner_type: &str,
        owner_id: &str,
        named_type: &str,
        named_id: &str,
    ) -> Result<(), DynoError> {
        let (ft, fid, tt, tid) = match twin.names {
            TwinEnd::Source => (named_type, named_id, owner_type, owner_id),
            TwinEnd::Target => (owner_type, owner_id, named_type, named_id),
        };
        self.create_edge(&twin.edge, ft, fid, tt, tid, Props::new())?;
        Ok(())
    }

    /// Keep every twin `node_type` declares in step with the node just
    /// written. Called by [`create_node`](Self::create_node) after the write,
    /// with the node's properties before it (`prior`, `None` for a new node).
    ///
    /// - When the authority MOVED, the edge the old value derived goes with
    ///   it. Otherwise a revised `subject_id` would leave the finding hanging
    ///   from its old subject as well, which is the second kind of drift the
    ///   measurement found.
    /// - When the derived edge is missing, it is drawn. It is never redrawn
    ///   when present, because the store REPLACES an edge's properties on a
    ///   repeat write, and a writer may have put some there.
    /// - A value naming nothing is left to the node-reference guard and the
    ///   dangling-reference detector, which already speak for it. A twin
    ///   declared `outlives_target` is allowed to.
    pub(crate) fn keep_twins_on_write(
        &mut self,
        node_type: &str,
        id: &str,
        props: &HashMap<String, Value>,
        prior: Option<&HashMap<String, Value>>,
    ) -> Result<(), DynoError> {
        for (prop, twin) in self.twins_of_type(node_type) {
            let now = string_prop(props, &prop).map(str::to_string);
            let before = prior
                .and_then(|p| string_prop(p, &prop))
                .map(str::to_string);
            if let Some(b) = before.as_deref()
                && now.as_deref() != Some(b)
            {
                let (f, t) = match twin.names {
                    TwinEnd::Source => (b, id),
                    TwinEnd::Target => (id, b),
                };
                self.delete_edge(&twin.edge, f, t)?;
            }
            let Some(v) = now else { continue };
            if self
                .twin_edges_at(id, &twin)?
                .iter()
                .any(|e| far_end(e, twin.names) == v)
            {
                continue;
            }
            if let Some(named_type) = self.resolve_named(props, &twin, &v)? {
                self.draw_twin(&twin, node_type, id, &named_type, &v)?;
            }
        }
        Ok(())
    }

    /// Why drawing (or, with `deleting`, removing) this edge by hand would put
    /// a stored twin out of step. `None` when the edge is not a declared twin's
    /// copy, or when the write agrees with the property.
    ///
    /// For the generic edge tools, which can name any edge type. The typed
    /// writers already draw the copy their property names. A refusal says
    /// what to do instead: set the property, and the edge follows. For a
    /// finding it also names the edge that means "this also concerns that".
    ///
    /// The node holding the property is found by the twin's own declared type,
    /// so the call needs only ids: `delete_edge` has no endpoint types to pass.
    pub fn twin_edge_refusal(
        &self,
        edge_type: &str,
        from_id: &str,
        to_id: &str,
        deleting: bool,
    ) -> Result<Option<String>, DynoError> {
        for st in self.stored_twins() {
            if st.twin.edge != edge_type {
                continue;
            }
            let (owner_id, far) = match st.twin.names {
                TwinEnd::Source => (to_id, from_id),
                TwinEnd::Target => (from_id, to_id),
            };
            let owner_type = st.node_type.as_str();
            let Some(owner) = self.get_node(owner_type, owner_id)? else {
                // Not this twin's node type, or a missing endpoint, which the
                // edge guard refuses in its own words.
                continue;
            };
            let names = string_prop(&owner.properties, &st.property);
            let agrees = names == Some(far);
            let msg = if deleting && agrees {
                format!(
                    "{edge_type} {from_id} -> {to_id} is the copy of {owner_type}.{} on \
                     '{owner_id}', which still names '{far}'. The property is the authority \
                     and the edge follows it: change {} to move the relation.",
                    st.property, st.property
                )
            } else if !deleting && !agrees {
                let named = names.map_or_else(|| "nothing".to_string(), |n| format!("'{n}'"));
                let mut m = format!(
                    "{edge_type} {from_id} -> {to_id} would disagree with {owner_type}.{} on \
                     '{owner_id}', which names {named}. {edge_type} here is the COPY of that \
                     property, so the store draws it: set {} and the edge follows.",
                    st.property, st.property
                );
                if let Some(other) = &st.twin.strays_become {
                    m.push_str(&format!(
                        " To say this {owner_type} ALSO concerns '{far}', draw {other} from \
                         '{owner_id}' to '{far}' — that is the edge for it \
                         (dec:has-temporal-fact-means-only-the-subject-and-also-concerns-is-about-entity)."
                    ));
                }
                m
            } else {
                continue;
            };
            return Ok(Some(msg));
        }
        Ok(None)
    }

    /// Bring every declared twin into step with its authority, and say what
    /// was changed. Idempotent, so a second run reports nothing.
    ///
    /// - A missing derived edge is drawn.
    /// - An edge joining a node to something its property does not name
    ///   becomes the twin's `strays_become` edge (a finding's second
    ///   `HAS_TEMPORAL_FACT` becomes `ABOUT_ENTITY`) or is removed.
    /// - A [`RETIRED_PROPERTIES`] value still stored is removed.
    ///
    /// Run on open and after an import is staged. Both REPORT the result:
    /// open through [`repaired_on_open`](Self::repaired_on_open), which
    /// `loop_status` reads, and import in its `ImportReport`.
    pub fn repair_stored_twins(&mut self) -> Result<TwinRepairs, DynoError> {
        let mut r = TwinRepairs::default();
        for st in self.stored_twins() {
            let twin = &st.twin;
            for n in self.scan_nodes(&st.node_type)? {
                let owner = n.node_id.as_str();
                let v = string_prop(&n.properties, &st.property).map(str::to_string);
                let edges = self.twin_edges_at(owner, twin)?;
                for e in &edges {
                    let far = far_end(e, twin.names).to_string();
                    if v.as_deref() == Some(far.as_str()) {
                        continue;
                    }
                    let names = v
                        .as_deref()
                        .map_or_else(|| "nothing".to_string(), |x| format!("'{x}'"));
                    match twin.strays_become.as_deref() {
                        Some(other) => {
                            if !self
                                .outgoing(owner, Some(other))?
                                .iter()
                                .any(|o| o.to_id == far)
                                && let Some(far_type) =
                                    self.resolve_named(&HashMap::new(), twin, &far)?
                            {
                                self.create_edge(
                                    other,
                                    &st.node_type,
                                    owner,
                                    &far_type,
                                    &far,
                                    Props::new(),
                                )?;
                            }
                            r.moved.push(format!(
                                "{} {} -> {} became {other} {owner} -> {far} ({}.{} names {names})",
                                twin.edge, e.from_id, e.to_id, st.node_type, st.property
                            ));
                        }
                        None => r.removed.push(format!(
                            "{} {} -> {} removed ({}.{} names {names})",
                            twin.edge, e.from_id, e.to_id, st.node_type, st.property
                        )),
                    }
                    self.delete_edge(&twin.edge, &e.from_id, &e.to_id)?;
                }
                let Some(v) = v else { continue };
                if edges.iter().any(|e| far_end(e, twin.names) == v) {
                    continue;
                }
                if let Some(named_type) = self.resolve_named(&n.properties, twin, &v)? {
                    self.draw_twin(twin, &st.node_type, owner, &named_type, &v)?;
                    let (f, t) = match twin.names {
                        TwinEnd::Source => (v.as_str(), owner),
                        TwinEnd::Target => (owner, v.as_str()),
                    };
                    r.added.push(format!(
                        "{} {f} -> {t} (the copy of {}.{})",
                        twin.edge, st.node_type, st.property
                    ));
                }
            }
        }
        for (ty, prop, why) in RETIRED_PROPERTIES {
            for n in self.scan_nodes(ty)? {
                if let Some(value) = n.properties.get(*prop) {
                    let shown = match value {
                        Value::String(s) => format!("'{s}'"),
                        Value::Bool(b) => b.to_string(),
                        Value::Int(i) => i.to_string(),
                        Value::Float(f) => f.to_string(),
                        other => format!("{other:?}"),
                    };
                    self.remove_properties(ty, &n.node_id, &[(*prop).to_string()])?;
                    r.retired_properties.push(format!(
                        "{ty} {}.{prop} = {shown} removed ({why})",
                        n.node_id
                    ));
                }
            }
        }
        Ok(r)
    }
}
