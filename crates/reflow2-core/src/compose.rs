//! COMPOSE — analyse two designs together without merging them
//! (`req:composed-analysis`).
//!
//! The user's framing, and it is better than the bespoke comparator this
//! started as: to check whether two projects line up, **import one design into
//! the other and run reflow2's ordinary checks over the whole**. The seam's
//! problems then surface as the gaps they already are — a contract with no
//! provider once both sides are present, a requirement nothing satisfies across
//! the join, a duplicate that is one thing named twice — instead of needing a
//! detector nobody else benefits from. It is the principle this project keeps
//! rediscovering: make the existing computation *see* more rather than write a
//! new one.
//!
//! ## Why this cannot just call `import_graph`
//!
//! `import_graph` writes every node under its **original id** with upsert
//! semantics. It exists to layer an export onto the design it came from, and
//! its own documentation says so. Point it at a *different* design and the
//! dependency's `cap:store` silently overwrites the consumer's — corruption
//! with no error, which is the worst shape a bug can take here.
//!
//! Neither of the other two composition mechanisms does this job either, and
//! the distinction is worth keeping straight:
//!
//! | Mechanism | What it composes |
//! |---|---|
//! | `mirror_surface` | the other design's **published surface only**, kept foreign |
//! | `merge_designs` | two **versions of the same design**, three-way |
//! | this | two **different designs**, for analysis only |
//!
//! ## Nothing is written to your design
//!
//! The combined graph is built **in memory and thrown away**. Your design is
//! read, never modified; the dependency is never persisted into it. That
//! disposes of the two hazards `req:composed-analysis` names — an export of
//! yours can never start shipping the dependency's internals, and there is no
//! residue to clean up — and it follows the precedent set by the ingest
//! handshake's prepare rounds, which replay against a throwaway graph for the
//! same reason.
//!
//! ## Ids are namespaced, and findings say which side they came from
//!
//! Every imported node is rewritten to `{namespace}::{id}`, edges with it, so
//! two designs that both call something `cmp:store` stay distinct. Every
//! finding is then attributed — **yours**, **theirs**, or **crosses the seam** —
//! because a consumer shown its dependency's internal gaps as if they were its
//! own will switch the whole feature off, and rightly.

use std::collections::BTreeMap;

use crate::foundation::core::DynoError;

use crate::export::GraphExport;
use crate::graph::DesignGraph;
use crate::heal::HealIssue;
use crate::touch::{NameCandidate, PairTouch, SetAside, TouchDesign, TouchLink, Touching};

/// Which design a finding belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Every node it names is yours. You would have seen this without composing.
    Ours,
    /// Every node it names belongs to the imported design. Reported, because
    /// hiding it would be dishonest, but it is not yours to fix.
    Theirs,
    /// It names nodes from BOTH designs. **This is the whole point** — a finding
    /// that only exists because the two were analysed together.
    Seam,
}

/// One finding from the combined design, attributed.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ComposedFinding {
    pub side: Side,
    /// `gap` or `defect`.
    pub kind: &'static str,
    pub source: String,
    pub title: String,
    /// Ids as they appear in the combined graph — imported ones still carry
    /// their `{namespace}::` prefix, so a reader can always tell whose is whose.
    pub affected_ids: Vec<String>,
}

/// What analysing the two together found.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ComposedReport {
    /// The imported design's own id, as its export declared it.
    pub imported_graph_id: String,
    /// The prefix its ids were given.
    pub namespace: String,
    pub imported_nodes: usize,
    pub imported_edges: usize,
    /// Edges the import could not place, with the reason — never dropped in
    /// silence.
    pub skipped_edges: Vec<String>,
    /// **Findings that span both designs.** Listed first because they are the
    /// only ones neither design could have found alone.
    pub seam_findings: Vec<ComposedFinding>,
    /// Findings wholly within your design.
    pub our_findings: Vec<ComposedFinding>,
    /// Findings wholly within theirs — reported, not yours to fix.
    pub their_findings: Vec<ComposedFinding>,
    /// Where the two designs touch, read from what their records hold
    /// ([`crate::touch`]): references one records to a node of the other,
    /// each drawn as an edge labelled `declared` in the combined graph.
    pub declared_links: Vec<TouchLink>,
    /// Links computed on exact evidence (the same file, a pin naming the
    /// other design), each drawn as an edge labelled `computed`, so the
    /// checks above saw across the join through it. Never read as declared.
    pub computed_links: Vec<TouchLink>,
    /// Interfaces whose names closely match with the same medium: for a
    /// person to confirm. Never drawn, never read by a check.
    pub candidates: Vec<NameCandidate>,
    /// The pair's status: `linked`, `candidates_only` or `never_touch`.
    pub link_status: PairTouch,
    /// Matches looked at and set aside as no evidence, counted.
    pub set_aside: SetAside,
    pub links_note: String,
}

/// Prefix an id, leaving an already-prefixed one alone.
fn ns(namespace: &str, id: &str) -> String {
    format!("{namespace}::{id}")
}

impl DesignGraph {
    /// Analyse this design together with another, without touching either
    /// (`req:composed-analysis`).
    ///
    /// `namespace` distinguishes the imported design's ids — usually its
    /// `graph_id`. See the module docs for why this is not `import_graph` and
    /// why nothing is persisted.
    pub fn compose_and_analyse(
        &self,
        other: &GraphExport,
        namespace: &str,
    ) -> Result<ComposedReport, DynoError> {
        if namespace.trim().is_empty() {
            return Err(DynoError::Query(
                "a namespace is required: without one the two designs' ids would collide, \
                 which is the whole reason this is not import_graph"
                    .to_string(),
            ));
        }

        // Build the combined graph in memory. Ours first, unchanged.
        let mut combined = DesignGraph::open_in_memory()?;
        let ours = self.export_graph()?;
        combined.import_graph(&ours)?;
        let our_ids: std::collections::BTreeSet<String> =
            ours.nodes.iter().map(|n| n.node_id.clone()).collect();

        // Theirs, namespaced, so nothing of ours is overwritten.
        let theirs = namespaced(other, namespace);
        let report = combined.import_graph(&theirs)?;

        // WHERE THEY ALREADY TOUCH, drawn as real edges in the combined graph
        // before any check runs (`req:designs-are-shown-where-they-already-touch-before-anyone-declares-it`).
        let this = self.graph_id().to_string();
        let their_key = if namespace == this {
            format!("{namespace} (composed)")
        } else {
            namespace.to_string()
        };
        let mut touching = crate::touch::where_designs_touch(&[
            TouchDesign {
                key: &this,
                graph_id: &this,
                export: &ours,
            },
            TouchDesign {
                key: &their_key,
                graph_id: &other.graph_id,
                export: other,
            },
        ]);
        draw_links(&mut combined, &mut touching, |d, id| {
            if d == this {
                id.to_string()
            } else {
                ns(namespace, id)
            }
        })?;
        let link_status = touching
            .pairs(&[this.as_str(), their_key.as_str()])
            .into_iter()
            .next()
            .expect("two designs make one pair");

        let is_ours = |id: &str| our_ids.contains(id);
        let classify = |ids: &[String]| -> Side {
            let mine = ids.iter().any(|i| is_ours(i));
            let theirs = ids.iter().any(|i| !is_ours(i));
            match (mine, theirs) {
                (true, true) => Side::Seam,
                (false, true) => Side::Theirs,
                _ => Side::Ours,
            }
        };

        let mut buckets: BTreeMap<&'static str, Vec<ComposedFinding>> = BTreeMap::new();
        for gap in combined.detect_gaps()? {
            let side = classify(&gap.affected_ids);
            buckets
                .entry(bucket(side))
                .or_default()
                .push(ComposedFinding {
                    side,
                    kind: "gap",
                    source: serde_json::to_string(&gap.gap_source)
                        .unwrap_or_default()
                        .trim_matches('"')
                        .to_string(),
                    title: gap.title,
                    affected_ids: gap.affected_ids,
                });
        }
        for defect in combined.open_defects()? {
            let ids = defect_ids(&defect);
            let side = classify(&ids);
            buckets
                .entry(bucket(side))
                .or_default()
                .push(ComposedFinding {
                    side,
                    kind: "defect",
                    source: format!("{:?}", defect.category),
                    title: defect.message.clone(),
                    affected_ids: ids,
                });
        }

        Ok(ComposedReport {
            imported_graph_id: other.graph_id.clone(),
            namespace: namespace.to_string(),
            imported_nodes: theirs.nodes.len(),
            imported_edges: report.edges_written,
            skipped_edges: report
                .skipped_edges
                .iter()
                .map(|s| format!("{s:?}"))
                .collect(),
            seam_findings: buckets.remove("seam").unwrap_or_default(),
            our_findings: buckets.remove("ours").unwrap_or_default(),
            their_findings: buckets.remove("theirs").unwrap_or_default(),
            declared_links: touching
                .links
                .iter()
                .filter(|l| l.link == "declared")
                .cloned()
                .collect(),
            computed_links: touching
                .links
                .iter()
                .filter(|l| l.link == "computed")
                .cloned()
                .collect(),
            candidates: touching.candidates,
            link_status,
            set_aside: touching.set_aside,
            links_note: crate::touch::LINKS_NOTE.to_string(),
        })
    }
}

/// Draw every link whose two ends are in the combined graph as a real edge,
/// labelled with its kind, then rewrite its ids to the form the reply shows
/// (`shown`). A link that cannot be drawn says why, in its `detail`.
fn draw_links(
    combined: &mut DesignGraph,
    touching: &mut Touching,
    shown: impl Fn(&str, &str) -> String,
) -> Result<(), DynoError> {
    use crate::foundation::core::Value;
    for l in &mut touching.links {
        let from = shown(&l.from_design, &l.from_id);
        let to = shown(&l.to_design, &l.to_id);
        l.drawn = Some(false);
        if !l.to_type.is_empty() && !l.to_id.is_empty() {
            let mut props: std::collections::HashMap<String, Value> =
                std::collections::HashMap::new();
            props.insert(crate::touch::LINK_PROPERTY.to_string(), Value::from(l.link));
            props.insert("evidence".to_string(), Value::from(l.evidence));
            props.insert("note".to_string(), Value::from(l.detail.as_str()));
            match combined.create_edge(
                crate::touch::LINK_EDGE,
                &l.from_type,
                &from,
                &l.to_type,
                &to,
                props,
            ) {
                Ok(_) => l.drawn = Some(true),
                Err(e) => l.detail = format!("{}; not drawn: {e}", l.detail),
            }
        }
        l.from_id = from;
        l.to_id = to;
    }
    for c in &mut touching.candidates {
        c.a_id = shown(&c.a_design, &c.a_id);
        c.b_id = shown(&c.b_design, &c.b_id);
    }
    Ok(())
}

/// A copy of `other` with every node id, and both ends of every edge, under
/// `{namespace}::` — the one rewrite both compositions make.
fn namespaced(other: &GraphExport, namespace: &str) -> GraphExport {
    let mut theirs = other.clone();
    for n in &mut theirs.nodes {
        n.node_id = ns(namespace, &n.node_id);
    }
    for e in &mut theirs.edges {
        e.from_id = ns(namespace, &e.from_id);
        e.to_id = ns(namespace, &e.to_id);
    }
    theirs
}

fn bucket(side: Side) -> &'static str {
    match side {
        Side::Seam => "seam",
        Side::Ours => "ours",
        Side::Theirs => "theirs",
    }
}

/// A defect's affected ids, however the issue names them.
fn defect_ids(issue: &HealIssue) -> Vec<String> {
    issue.affected_ids.clone()
}

// ── SEVERAL DESIGNS AT ONCE ─────────────────────────────────────────────────
//
// `req:one-composed-check-runs-over-a-hub-and-several-of-its-designs`: a hub
// effort spans three to six designs, and composing them pairwise with THIS
// design never analyses the seam between two of the others. So the same
// mechanism takes a list: this design unchanged, each other design under its
// own `{design}::` namespace, ONE combined graph built in memory and thrown
// away, the ordinary checks run once over the whole. A finding is attributed
// to the SET of designs its ids belong to — one design, a pair, or more —
// rather than to ours / theirs / seam, which only means anything with two.

/// One other design composed in by [`DesignGraph::compose_many`]: its export
/// and the name its ids are namespaced under (its graph_id, as a rule).
#[derive(Debug, Clone, Copy)]
pub struct ComposeWith<'a> {
    pub design: &'a str,
    pub export: &'a GraphExport,
}

/// What one design brought into the combined graph.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ComposedDesign {
    pub design: String,
    pub nodes: usize,
    pub edges: usize,
    /// Edges the import could not place, with the reason — never dropped in
    /// silence.
    pub skipped_edges: Vec<String>,
}

/// One finding from the combined graph, attributed to the designs it names.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AttributedFinding {
    /// `gap` or `defect`.
    pub kind: &'static str,
    pub source: String,
    pub title: String,
    /// This design's ids as they are; another design's as `<design>:<id>`,
    /// the form a hub's view and a read aimed at a pinned design open.
    pub affected_ids: Vec<String>,
    /// The designs its ids belong to, once each: this design first, then in
    /// the order they were composed. One design is that design's own finding;
    /// two is the seam between them; more is a finding across them all.
    /// EMPTY when it names no node at all ("nothing confirms it works"): it is
    /// about the composition as a whole, which no one design owns. (The
    /// two-design composition files such a finding as ours.)
    pub designs: Vec<String>,
}

/// What analysing this design and several others together found.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ManyComposedReport {
    /// This design's graph_id: its ids are not prefixed.
    pub this_design: String,
    pub this_nodes: usize,
    /// Each other design, in the order composed.
    pub composed: Vec<ComposedDesign>,
    /// Every finding of the ordinary checks over the whole, in the order the
    /// checks return them (gaps, then defects).
    pub findings: Vec<AttributedFinding>,
    /// Where the designs already touch ([`crate::touch`]): declared and
    /// computed links (each drawn as a labelled edge in the combined graph
    /// before the checks ran) and name candidates (never drawn), with ids in
    /// the form `affected_ids` uses.
    pub touching: Touching,
    /// Every pair of the designs composed, in composition order, with its
    /// status.
    pub pairs: Vec<PairTouch>,
}

impl DesignGraph {
    /// Analyse this design together with several others at once, without
    /// touching any (`req:one-composed-check-runs-over-a-hub-and-several-of-its-designs`).
    ///
    /// Each other design's ids go under `{design}::`, so no two collide;
    /// a design named twice, an empty name, or this design's own graph_id is
    /// refused rather than composed into a collision. With no other design the
    /// combined graph is this design alone, and every finding is its own.
    pub fn compose_many(
        &self,
        others: &[ComposeWith<'_>],
    ) -> Result<ManyComposedReport, DynoError> {
        let this = self.graph_id().to_string();
        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for o in others {
            if o.design.trim().is_empty() {
                return Err(DynoError::Query(
                    "every design composed needs a name to namespace its ids under; one was \
                     empty"
                        .to_string(),
                ));
            }
            if o.design == this {
                return Err(DynoError::Query(format!(
                    "{this} is this design itself, which is always composed; name the other \
                     designs only"
                )));
            }
            if !seen.insert(o.design) {
                return Err(DynoError::Query(format!(
                    "design {} is named twice; each design is composed once",
                    o.design
                )));
            }
        }

        let mut combined = DesignGraph::open_in_memory()?;
        let ours = self.export_graph()?;
        combined.import_graph(&ours)?;
        let our_ids: std::collections::BTreeSet<String> =
            ours.nodes.iter().map(|n| n.node_id.clone()).collect();

        let mut composed = Vec::with_capacity(others.len());
        for o in others {
            let theirs = namespaced(o.export, o.design);
            let report = combined.import_graph(&theirs)?;
            composed.push(ComposedDesign {
                design: o.design.to_string(),
                nodes: theirs.nodes.len(),
                edges: report.edges_written,
                skipped_edges: report
                    .skipped_edges
                    .iter()
                    .map(|s| format!("{s:?}"))
                    .collect(),
            });
        }

        // WHERE THEY ALREADY TOUCH, drawn as real edges before any check runs
        // (`req:designs-are-shown-where-they-already-touch-before-anyone-declares-it`).
        let mut touch_set = vec![TouchDesign {
            key: &this,
            graph_id: &this,
            export: &ours,
        }];
        touch_set.extend(others.iter().map(|o| TouchDesign {
            key: o.design,
            graph_id: o.design,
            export: o.export,
        }));
        let mut touching = crate::touch::where_designs_touch(&touch_set);
        draw_links(&mut combined, &mut touching, |d, id| {
            if d == this { id.to_string() } else { ns(d, id) }
        })?;
        // The reply shows another design's ids as `<design>:<id>`.
        let display = |d: &str, combined_id: &str| -> String {
            if d == this {
                combined_id.to_string()
            } else {
                combined_id
                    .strip_prefix(&format!("{d}::"))
                    .map(|rest| format!("{d}:{rest}"))
                    .unwrap_or_else(|| combined_id.to_string())
            }
        };
        for l in &mut touching.links {
            l.from_id = display(&l.from_design, &l.from_id);
            l.to_id = display(&l.to_design, &l.to_id);
        }
        for c in &mut touching.candidates {
            c.a_id = display(&c.a_design, &c.a_id);
            c.b_id = display(&c.b_design, &c.b_id);
        }

        // Which design an id belongs to, and the form a reader opens it by.
        // Ours first (an id of ours may itself contain `::`), then the
        // longest namespace that prefixes it.
        let mut by_length: Vec<&str> = others.iter().map(|o| o.design).collect();
        by_length.sort_by_key(|d| std::cmp::Reverse(d.len()));
        let place = |id: &str| -> (Option<String>, String) {
            if our_ids.contains(id) {
                return (Some(this.clone()), id.to_string());
            }
            for d in &by_length {
                if let Some(rest) = id.strip_prefix(&format!("{d}::")) {
                    return (Some((*d).to_string()), format!("{d}:{rest}"));
                }
            }
            (None, id.to_string())
        };
        let order: Vec<&str> = std::iter::once(this.as_str())
            .chain(others.iter().map(|o| o.design))
            .collect();
        let attribute = |kind: &'static str, source: String, title: String, ids: &[String]| {
            let mut named: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            let mut affected_ids = Vec::with_capacity(ids.len());
            for id in ids {
                let (design, shown) = place(id);
                if let Some(d) = design {
                    named.insert(d);
                }
                affected_ids.push(shown);
            }
            let designs = order
                .iter()
                .filter(|d| named.contains(**d))
                .map(|d| (*d).to_string())
                .collect();
            AttributedFinding {
                kind,
                source,
                title,
                affected_ids,
                designs,
            }
        };

        let mut findings = Vec::new();
        for gap in combined.detect_gaps()? {
            let source = serde_json::to_string(&gap.gap_source)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();
            findings.push(attribute("gap", source, gap.title, &gap.affected_ids));
        }
        for defect in combined.open_defects()? {
            let ids = defect_ids(&defect);
            findings.push(attribute(
                "defect",
                format!("{:?}", defect.category),
                defect.message.clone(),
                &ids,
            ));
        }

        let keys: Vec<&str> = std::iter::once(this.as_str())
            .chain(others.iter().map(|o| o.design))
            .collect();
        let pairs = touching.pairs(&keys);
        Ok(ManyComposedReport {
            this_nodes: ours.nodes.len(),
            this_design: this,
            composed,
            findings,
            touching,
            pairs,
        })
    }
}
