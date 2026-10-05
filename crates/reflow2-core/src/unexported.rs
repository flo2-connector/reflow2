//! What the store holds that its last export does not — grouped by who wrote it.
//!
//! # The finding this answers
//!
//! The VS Code field log of 2026-10-05 (`art:vscode-call-field-log-2026-10-05`):
//! one store held another session's uncommitted work, so an export made to
//! commit one session's changes either swept the other's in or was skipped —
//! twice in one day it was skipped. `compare_designs` against the committed
//! export showed WHICH nodes were unexported (20, then 49), and nothing said
//! whose they were. The reporter's idea: list the unexported changes so the
//! person can see whose they are before exporting.
//!
//! # What this is, and what it deliberately is not
//!
//! A READING AID over a diff that already exists. The diff is
//! [`crate::compare::compare_designs`] of the record against the live store —
//! the same computation `compare_designs` with `base_path` serves, so the two
//! can never disagree about what moved. This module only sorts that diff into
//! groups by what the store records about each item's writer, and bounds it.
//!
//! It is NOT a partial export, and does not lead to one. An export carries the
//! whole store; exporting one group alone could leave an edge whose other end
//! it left out, and which shape a scoped export should take is an open
//! decision for the owner
//! (`dec:idea-an-export-can-leave-out-another-sessions-unexported-work`).
//!
//! # Who wrote an item — only what the store records, never a guess
//!
//! The core takes no clock and records no session, so the grouping keys are
//! exactly the ones the store already holds:
//!
//! - **the writer**: a `Contributor` whose `AUTHORED_BY` author edge on the
//!   item was written or changed since the record. A session gets such an edge
//!   by declaring `writes_for` ([`crate::attribution`]); the agent it wrote
//!   through rides on the edge as `authored_via` ([`crate::acting`]). An edge
//!   that was already in the record says who wrote the item BEFORE, not who
//!   changed it since, so it does not attribute the change.
//! - **the epoch**: the `DesignEpoch` an item, or a `ChangeEvent` that CHANGED
//!   it, is pinned to (`AT_EPOCH` / `OCCURS_DURING`). The store's own
//!   bookkeeping epoch for preserved prior states is not a planned increment
//!   and is not used as one.
//! - **the window**: dates the items themselves carry (a ChangeEvent's
//!   `detected_at`, a TemporalFact's `valid_from`, an author edge's
//!   `authored_at`). Most writes carry none, and an undated item is counted
//!   as undated rather than given a date.
//!
//! An item with no recorded writer — a write made with no `writes_for` in
//! force, and every removal — is counted under `written_by: None`, never
//! folded into a person's group.
//!
//! # Bounded on purpose
//!
//! At most [`MAX_GROUPS`] groups and [`NAMED_PER_GROUP`] ids per group. What is
//! not shown is COUNTED (`groups_not_shown`, `changes_not_shown`,
//! `ids_not_shown`), and `full_list` names the call that lists every item.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::compare::{LIVE_GRAPH_LABEL, compare_designs};
use crate::export::{ExportedEdge, GraphExport};
use crate::foundation::core::{DynoError, Value};
use crate::graph::DesignGraph;
use crate::nodes::{edge, node};
use crate::temporal::PRESERVE_EPOCH_ID;

/// How many groups one answer shows. The rest are counted.
pub const MAX_GROUPS: usize = 8;

/// How many item ids one group names. The rest are counted.
pub const NAMED_PER_GROUP: usize = 5;

/// How many items of each kind, in a group or in total.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ChangeCounts {
    pub nodes_added: usize,
    pub nodes_changed: usize,
    pub nodes_removed: usize,
    pub edges_added: usize,
    pub edges_changed: usize,
    pub edges_removed: usize,
}

impl ChangeCounts {
    /// Every item counted here.
    pub fn total(&self) -> usize {
        self.nodes_added
            + self.nodes_changed
            + self.nodes_removed
            + self.edges_added
            + self.edges_changed
            + self.edges_removed
    }

    fn bump(&mut self, kind: Kind) {
        match kind {
            Kind::NodeAdded => self.nodes_added += 1,
            Kind::NodeChanged => self.nodes_changed += 1,
            Kind::NodeRemoved => self.nodes_removed += 1,
            Kind::EdgeAdded => self.edges_added += 1,
            Kind::EdgeChanged => self.edges_changed += 1,
            Kind::EdgeRemoved => self.edges_removed += 1,
        }
    }

    /// "2 node(s) added, 1 changed; 3 edge(s) added" — only what is non-zero.
    pub fn phrase(&self) -> String {
        fn part(label: &str, n: [(usize, &str); 3]) -> Option<String> {
            // "2 node(s) added, 1 changed": the noun rides on the first count.
            let said: Vec<String> = n
                .iter()
                .filter(|(c, _)| *c > 0)
                .enumerate()
                .map(|(i, (c, w))| {
                    if i == 0 {
                        format!("{c} {label} {w}")
                    } else {
                        format!("{c} {w}")
                    }
                })
                .collect();
            (!said.is_empty()).then(|| said.join(", "))
        }
        [
            part(
                "node(s)",
                [
                    (self.nodes_added, "added"),
                    (self.nodes_changed, "changed"),
                    (self.nodes_removed, "removed"),
                ],
            ),
            part(
                "edge(s)",
                [
                    (self.edges_added, "added"),
                    (self.edges_changed, "changed"),
                    (self.edges_removed, "removed"),
                ],
            ),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("; ")
    }
}

/// The dates a group's items carry. Absent on a group whose items carry none.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DateWindow {
    pub first: String,
    pub last: String,
    /// How many of the group's items carry a date. The rest are undated.
    pub dated: usize,
}

/// The unexported changes one writer made, in one epoch.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UnexportedGroup {
    /// The Contributor the store credits with these changes. `None` when the
    /// store records no writer for them — see `summary` for why.
    pub written_by: Option<String>,
    /// The agent(s) the credited writes went through (`authored_via`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub via: Vec<String>,
    /// The DesignEpoch these changes are pinned to, when one is recorded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epoch: Option<String>,
    pub counts: ChangeCounts,
    /// Up to [`NAMED_PER_GROUP`] item ids: nodes first (design content before
    /// the supporting layer), then edges as `TYPE from -> to`.
    pub ids: Vec<String>,
    /// How many of the group's items `ids` does not name.
    pub ids_not_shown: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<DateWindow>,
    /// The group in one sentence.
    pub summary: String,
}

/// What the live store holds that a record does not, grouped by writer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UnexportedChanges {
    /// The record the store was compared against.
    pub base: String,
    /// True when the store and the record agree on every node and edge.
    pub identical: bool,
    pub totals: ChangeCounts,
    /// At most [`MAX_GROUPS`], largest first.
    pub groups: Vec<UnexportedGroup>,
    /// Groups not shown, and how many changes they hold — counted, never
    /// dropped.
    pub groups_not_shown: usize,
    pub changes_not_shown: usize,
    /// The call that lists every change in full.
    pub full_list: String,
    /// How the grouping is computed, and what it cannot see.
    pub note: String,
}

#[derive(Debug, Clone, Copy)]
enum Kind {
    NodeAdded,
    NodeChanged,
    NodeRemoved,
    EdgeAdded,
    EdgeChanged,
    EdgeRemoved,
}

/// The grouping key: writer, the agents it went through, the epoch.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    written_by: Option<String>,
    via: Vec<String>,
    epoch: Option<String>,
}

#[derive(Default)]
struct Bucket {
    counts: ChangeCounts,
    /// (sort rank, id) — nodes of the design band, then the supporting band,
    /// then edges, each in diff order.
    ids: Vec<(u8, String)>,
    dates: Vec<String>,
}

/// The live store's nodes and edges, indexed for the attribution walk.
struct LiveIndex<'a> {
    node_type: BTreeMap<&'a str, &'a str>,
    props: BTreeMap<&'a str, &'a crate::export::Props>,
    out: BTreeMap<&'a str, Vec<&'a ExportedEdge>>,
    inc: BTreeMap<&'a str, Vec<&'a ExportedEdge>>,
}

impl<'a> LiveIndex<'a> {
    fn new(live: &'a GraphExport) -> Self {
        let mut ix = LiveIndex {
            node_type: BTreeMap::new(),
            props: BTreeMap::new(),
            out: BTreeMap::new(),
            inc: BTreeMap::new(),
        };
        for n in &live.nodes {
            ix.node_type.insert(&n.node_id, &n.node_type);
            ix.props.insert(&n.node_id, &n.properties);
        }
        for e in &live.edges {
            ix.out.entry(&e.from_id).or_default().push(e);
            ix.inc.entry(&e.to_id).or_default().push(e);
        }
        ix
    }

    fn out_of(&self, id: &str, ty: &str) -> impl Iterator<Item = &&'a ExportedEdge> {
        self.out
            .get(id)
            .into_iter()
            .flatten()
            .filter(move |e| e.edge_type == ty)
    }

    fn incoming_of(&self, id: &str, ty: &str) -> impl Iterator<Item = &&'a ExportedEdge> {
        self.inc
            .get(id)
            .into_iter()
            .flatten()
            .filter(move |e| e.edge_type == ty)
    }

    fn prop(&self, id: &str, key: &str) -> Option<String> {
        self.props
            .get(id)
            .and_then(|p| p.get(key))
            .and_then(|v| v.as_str().map(str::to_string))
    }
}

fn strings(v: Option<&Value>) -> Vec<String> {
    match v {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect(),
        Some(Value::String(s)) => vec![s.clone()],
        _ => Vec::new(),
    }
}

/// Does this AUTHORED_BY edge hold the author role? The set shape (`roles`)
/// and the legacy single `role` both count; an edge with neither is the old
/// schema default, which was author.
fn is_author_edge(e: &ExportedEdge) -> bool {
    let roles = strings(e.properties.get("roles"));
    if !roles.is_empty() {
        return roles.iter().any(|r| r == "author");
    }
    match e.properties.get("role").and_then(|v| v.as_str()) {
        Some(r) => r == "author",
        None => true,
    }
}

type EdgeKey = (String, String, String);

fn edge_key(ty: &str, from: &str, to: &str) -> EdgeKey {
    (ty.to_string(), from.to_string(), to.to_string())
}

struct Attribution<'a> {
    ix: LiveIndex<'a>,
    /// Edges written or changed since the record.
    new_edges: BTreeSet<EdgeKey>,
    /// Nodes added or changed since the record.
    moved_nodes: BTreeSet<String>,
}

impl Attribution<'_> {
    /// The writer(s) and agent(s) the store credits with a change to `id`
    /// since the record, plus the dates those credits carry.
    fn credited(&self, id: &str) -> Option<(String, Vec<String>, Vec<String>)> {
        let mut who: BTreeSet<String> = BTreeSet::new();
        let mut via: BTreeSet<String> = BTreeSet::new();
        let mut dates = Vec::new();
        for e in self.ix.out_of(id, edge::AUTHORED_BY) {
            if !is_author_edge(e)
                || !self
                    .new_edges
                    .contains(&edge_key(&e.edge_type, &e.from_id, &e.to_id))
            {
                continue;
            }
            who.insert(e.to_id.clone());
            via.extend(strings(e.properties.get("authored_via")));
            if let Some(d) = e.properties.get("authored_at").and_then(|v| v.as_str()) {
                dates.push(d.to_string());
            }
        }
        if who.is_empty() {
            return None;
        }
        Some((
            who.into_iter().collect::<Vec<_>>().join(", "),
            via.into_iter().collect(),
            dates,
        ))
    }

    /// The epoch an item belongs to: its own pin, a DesignEpoch's own id, or
    /// the pin of a ChangeEvent that changed it (one changed since the record
    /// first). The store's preserve-on-write bookkeeping epoch is skipped.
    fn epoch_of(&self, id: &str) -> Option<String> {
        if self.ix.node_type.get(id).copied() == Some(node::DESIGN_EPOCH) {
            return (id != PRESERVE_EPOCH_ID).then(|| id.to_string());
        }
        let own = |n: &str| -> Option<String> {
            let mut pins: Vec<&str> = self
                .ix
                .out_of(n, edge::AT_EPOCH)
                .chain(self.ix.out_of(n, edge::OCCURS_DURING))
                .map(|e| e.to_id.as_str())
                .filter(|t| *t != PRESERVE_EPOCH_ID)
                .collect();
            pins.sort();
            pins.first().map(|s| s.to_string())
        };
        if let Some(e) = own(id) {
            return Some(e);
        }
        let mut changes: Vec<&str> = self
            .ix
            .incoming_of(id, edge::CHANGED)
            .map(|e| e.from_id.as_str())
            .collect();
        // Changes made since the record first: they are the ones that moved
        // this item into the list.
        changes.sort_by_key(|c| (!self.moved_nodes.contains(*c), c.to_string()));
        changes.into_iter().find_map(own)
    }

    /// The grouping key and the dates for a node added or changed since the
    /// record.
    fn node_key(&self, id: &str) -> (Key, Vec<String>) {
        let mut dates = Vec::new();
        for (ty, prop) in [
            (node::CHANGE_EVENT, "detected_at"),
            (node::TEMPORAL_FACT, "valid_from"),
        ] {
            if self.ix.node_type.get(id).copied() == Some(ty)
                && let Some(d) = self.ix.prop(id, prop)
            {
                dates.push(d);
            }
        }
        // A preserved prior state belongs with the node it preserves.
        if self.ix.node_type.get(id).copied() == Some(node::SNAPSHOT)
            && let Some(subject) = self
                .ix
                .incoming_of(id, edge::HAS_SNAPSHOT)
                .map(|e| e.from_id.clone())
                .next()
            && subject != id
        {
            let (key, _) = self.node_key_direct(&subject);
            return (key, dates);
        }
        let (key, credit_dates) = self.node_key_direct(id);
        dates.extend(credit_dates);
        (key, dates)
    }

    fn node_key_direct(&self, id: &str) -> (Key, Vec<String>) {
        let epoch = self.epoch_of(id);
        if let Some((who, via, dates)) = self.credited(id) {
            return (
                Key {
                    written_by: Some(who),
                    via,
                    epoch,
                },
                dates,
            );
        }
        // Credited through the change that records it: a ChangeEvent written
        // since the record by a credited writer CHANGED this item.
        let via_change = self
            .ix
            .incoming_of(id, edge::CHANGED)
            .map(|e| e.from_id.as_str())
            .filter(|c| self.moved_nodes.contains(*c))
            .find_map(|c| self.credited(c));
        if let Some((who, via, dates)) = via_change {
            return (
                Key {
                    written_by: Some(who),
                    via,
                    epoch,
                },
                dates,
            );
        }
        (
            Key {
                written_by: None,
                via: Vec::new(),
                epoch,
            },
            Vec::new(),
        )
    }

    /// The grouping key for an edge added or changed since the record: the
    /// group of an endpoint that moved, else the writer a credit edge names.
    fn edge_group_key(&self, e: &ExportedEdge) -> (Key, Vec<String>) {
        for end in [&e.from_id, &e.to_id] {
            if self.moved_nodes.contains(end.as_str()) {
                let (key, _) = self.node_key(end);
                return (key, Vec::new());
            }
        }
        if e.edge_type == edge::AUTHORED_BY && is_author_edge(e) {
            let mut dates = Vec::new();
            if let Some(d) = e.properties.get("authored_at").and_then(|v| v.as_str()) {
                dates.push(d.to_string());
            }
            return (
                Key {
                    written_by: Some(e.to_id.clone()),
                    via: strings(e.properties.get("authored_via")),
                    epoch: self.epoch_of(&e.from_id),
                },
                dates,
            );
        }
        if e.edge_type == edge::ACTS_FOR {
            return (
                Key {
                    written_by: Some(e.to_id.clone()),
                    via: vec![e.from_id.clone()],
                    epoch: None,
                },
                Vec::new(),
            );
        }
        (Key::default(), Vec::new())
    }
}

impl DesignGraph {
    /// What this store holds that `base` (a record of it) does not, grouped by
    /// what the store records about who wrote each change. See the module docs
    /// for the keys, the bounds and what it cannot see.
    pub fn unexported_changes(
        &self,
        base: &GraphExport,
        base_label: &str,
    ) -> Result<UnexportedChanges, DynoError> {
        let live = self.export_graph()?;
        Ok(group_unexported(base, &live, base_label))
    }
}

/// [`DesignGraph::unexported_changes`] over a live document already in hand.
pub fn group_unexported(
    base: &GraphExport,
    live: &GraphExport,
    base_label: &str,
) -> UnexportedChanges {
    let diff = compare_designs(base, live, base_label, LIVE_GRAPH_LABEL);

    let mut moved_nodes: BTreeSet<String> = BTreeSet::new();
    for band in [&diff.design, &diff.supporting] {
        moved_nodes.extend(band.added.iter().map(|n| n.node_id.clone()));
        moved_nodes.extend(band.changed.iter().map(|n| n.node_id.clone()));
    }
    let mut new_edges: BTreeSet<EdgeKey> = BTreeSet::new();
    new_edges.extend(
        diff.edges_added
            .iter()
            .map(|e| edge_key(&e.edge_type, &e.from_id, &e.to_id)),
    );
    new_edges.extend(
        diff.edges_changed
            .iter()
            .map(|e| edge_key(&e.edge_type, &e.from_id, &e.to_id)),
    );
    let at = Attribution {
        ix: LiveIndex::new(live),
        new_edges,
        moved_nodes,
    };

    let mut buckets: BTreeMap<Key, Bucket> = BTreeMap::new();
    let mut totals = ChangeCounts::default();
    let mut put = |key: Key, kind: Kind, rank: u8, id: String, dates: Vec<String>| {
        totals.bump(kind);
        let b = buckets.entry(key).or_default();
        b.counts.bump(kind);
        b.ids.push((rank, id));
        b.dates.extend(dates);
    };

    for (rank, band) in [(0u8, &diff.design), (1u8, &diff.supporting)] {
        for n in &band.added {
            let (key, dates) = at.node_key(&n.node_id);
            put(key, Kind::NodeAdded, rank, n.node_id.clone(), dates);
        }
        for n in &band.changed {
            let (key, dates) = at.node_key(&n.node_id);
            put(key, Kind::NodeChanged, rank, n.node_id.clone(), dates);
        }
        // Nothing records who removed a node.
        for n in &band.removed {
            put(
                Key::default(),
                Kind::NodeRemoved,
                rank,
                n.node_id.clone(),
                Vec::new(),
            );
        }
    }
    let live_edge = |ty: &str, from: &str, to: &str| -> Option<&ExportedEdge> {
        at.ix.out_of(from, ty).find(|e| e.to_id == to).map(|e| &**e)
    };
    for (kind, list) in [
        (
            Kind::EdgeAdded,
            diff.edges_added
                .iter()
                .map(|e| (&e.edge_type, &e.from_id, &e.to_id))
                .collect::<Vec<_>>(),
        ),
        (
            Kind::EdgeChanged,
            diff.edges_changed
                .iter()
                .map(|e| (&e.edge_type, &e.from_id, &e.to_id))
                .collect::<Vec<_>>(),
        ),
    ] {
        for (ty, from, to) in list {
            let (key, dates) = match live_edge(ty, from, to) {
                Some(e) => at.edge_group_key(e),
                None => (Key::default(), Vec::new()),
            };
            put(key, kind, 2, format!("{ty} {from} -> {to}"), dates);
        }
    }
    for e in &diff.edges_removed {
        put(
            Key::default(),
            Kind::EdgeRemoved,
            2,
            format!("{} {} -> {}", e.edge_type, e.from_id, e.to_id),
            Vec::new(),
        );
    }

    // Largest first; a credited writer before the uncredited remainder on a
    // tie; then by key, so the order is a property of the data.
    let mut all: Vec<(Key, Bucket)> = buckets.into_iter().collect();
    all.sort_by(|(ka, a), (kb, b)| {
        b.counts
            .total()
            .cmp(&a.counts.total())
            .then_with(|| ka.written_by.is_none().cmp(&kb.written_by.is_none()))
            .then_with(|| ka.cmp(kb))
    });
    let groups_not_shown = all.len().saturating_sub(MAX_GROUPS);
    let changes_not_shown: usize = all
        .iter()
        .skip(MAX_GROUPS)
        .map(|(_, b)| b.counts.total())
        .sum();
    let groups: Vec<UnexportedGroup> = all
        .into_iter()
        .take(MAX_GROUPS)
        .map(|(key, mut b)| {
            b.ids.sort_by_key(|(rank, _)| *rank);
            let total = b.counts.total();
            let ids: Vec<String> = b
                .ids
                .into_iter()
                .take(NAMED_PER_GROUP)
                .map(|(_, id)| id)
                .collect();
            let window = window_of(&b.dates);
            let summary = summarise(&key, &b.counts, &ids, total, window.as_ref());
            UnexportedGroup {
                written_by: key.written_by,
                via: key.via,
                epoch: key.epoch,
                ids_not_shown: total - ids.len(),
                counts: b.counts,
                ids,
                window,
                summary,
            }
        })
        .collect();

    UnexportedChanges {
        base: base_label.to_string(),
        identical: diff.summary.identical,
        totals,
        groups,
        groups_not_shown,
        changes_not_shown,
        full_list: format!(
            "compare_designs with base_path \"{base_label}\" lists every change, with each \
             property that moved."
        ),
        note: "Grouped by what the store records about who wrote each change: an author edge \
               (AUTHORED_BY) written since the record, which a session's writes get when it \
               declares writes_for, with the agent it went through (authored_via); and the \
               epoch an item, or a ChangeEvent that changed it, is pinned to. A write made with \
               no writes_for in force, and every removal, has no recorded writer, so it is \
               counted under written_by: null rather than guessed. This is a list to read before \
               exporting, not a partial export: an export carries the whole store, and exporting \
               one group alone could leave edges pointing at nodes it left out."
            .to_string(),
    }
}

fn window_of(dates: &[String]) -> Option<DateWindow> {
    let first = dates.iter().min()?;
    let last = dates.iter().max()?;
    Some(DateWindow {
        first: first.clone(),
        last: last.clone(),
        dated: dates.len(),
    })
}

fn summarise(
    key: &Key,
    counts: &ChangeCounts,
    ids: &[String],
    total: usize,
    window: Option<&DateWindow>,
) -> String {
    let who = match &key.written_by {
        Some(w) if key.via.is_empty() => w.clone(),
        Some(w) => format!("{w} via {}", key.via.join(", ")),
        None => "no record of who wrote these (a write with no writes_for in force, or a \
                 removal)"
            .to_string(),
    };
    let mut s = format!("{who}: {}", counts.phrase());
    if let Some(e) = &key.epoch {
        s.push_str(&format!(" · epoch {e}"));
    }
    match window {
        Some(w) if w.first == w.last => s.push_str(&format!(" · dated {}", w.first)),
        Some(w) => s.push_str(&format!(" · dated {} to {}", w.first, w.last)),
        None => s.push_str(" · undated"),
    }
    if !ids.is_empty() {
        s.push_str(&format!(" · {}", ids.join(", ")));
        if total > ids.len() {
            s.push_str(&format!(" (+{} more)", total - ids.len()));
        }
    }
    s
}
