//! Relating a node to what is already in the design — and recording the
//! judgement when nothing is honestly related.
//!
//! `dec:idea-what-notices-an-idea-that-connects-to-nothing`, accepted
//! 2026-08-21. The measurement behind it: 145 brainstormed ideas joined by 12
//! edges, 111 of them reaching no other idea within two hops, while the
//! relation vocabulary that would have joined them had existed the whole time
//! and was used 81 times elsewhere in the same graph. **The missing leg was
//! never the tool. It was that nothing asked, and nothing noticed.**
//!
//! # Why both outcomes go through one door
//!
//! An act of relating has two honest endings: an edge, or the finding that
//! there is nothing to draw. Only the first leaves a trace today, which makes
//! the second indistinguishable from never having looked — and any detector
//! built over "has no relation" would therefore keep reporting people who did
//! the work. So [`DesignGraph::review_relations`] accepts either ending and
//! refuses to accept neither. It is the shape the dedup guard already uses:
//! that guard does not infer that you considered the near-duplicate, it makes
//! you say so, and then the saying is a fact rather than a guess.
//!
//! # Why a note and not a flag
//!
//! `no_relation_note` carries what was searched and what was nearest, because
//! that is the part a later reader can overturn. A bare boolean would record
//! that a review happened and nothing about whether it was any good — and the
//! reader who disagrees would have nothing to disagree *with*.
//!
//! # What this deliberately does not do
//!
//! It does not suggest relations. Offering candidates is the dedup guard's job
//! and the skill's; choosing among them is a judgement, and a machine that
//! proposed edges here would be manufacturing exactly the false neighbours the
//! brainstorm skill forbids. A fabricated relation is worse than a missing one,
//! because anything that searches by neighbourhood repeats it forever.

use crate::foundation::core::{DynoError, PropertyDef, PropertyType, Schema, Value};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::graph::DesignGraph;
use crate::nodes::{Props, edge, node};

/// The relations a review may draw — the inference layer's "why" vocabulary,
/// which is wildcard-ended and so joins any two nodes.
///
/// Listed here rather than accepting any edge type on purpose. `CONTAINS` and
/// `SATISFIES` are structure, not commentary; letting a review draw one would
/// put load-bearing design edges behind a tool whose whole premise is that the
/// author is still thinking.
pub const REVIEW_RELATIONS: &[&str] = &[
    edge::CONTRADICTS,
    edge::EVOLVES_INTO,
    edge::DEPENDS_ON,
    edge::CAUSES,
    edge::TRIGGERS,
    edge::BLOCKS,
    edge::DUPLICATES,
    edge::ANTICIPATES,
    edge::OBSOLETES,
    edge::RISKS,
    edge::MITIGATES,
    edge::MASKS,
    edge::VIOLATES,
];

/// One relation a review draws, with the reason it was drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct RelationLink {
    /// One of [`REVIEW_RELATIONS`].
    pub relation: String,
    pub other_type: String,
    pub other_id: String,
    /// WHY this relation is true, in a sentence. Required: a relation with no
    /// evidence is an assertion the next reader can neither check nor overturn.
    pub evidence: String,
    /// Draw the edge FROM the other node instead of from the node under review.
    ///
    /// Every one of these relations reads as a sentence — *from RELATION to* —
    /// and backwards the same edge asserts something false. "The older idea
    /// EVOLVES_INTO this one" and "this one EVOLVES_INTO the older" are not the
    /// same claim, and nothing downstream can tell that one of them is wrong.
    pub incoming: bool,
    /// Values for the relation's DEFAULTED properties — the ones
    /// [`review_link_properties`] names, such as CONTRADICTS `alignment`.
    /// Empty is exactly the behaviour before this field existed: the schema
    /// default is stored.
    ///
    /// ⭐ WHY THESE AND ONLY THESE. A property with no default is simply
    /// absent when nobody says it. One WITH a default materialises on create,
    /// so it asserts a value whether or not the writer meant it — and until
    /// 2026-10-13 a writer here could not say otherwise. Every CONTRADICTS a
    /// review drew was stored `opposing`; 10 in reflow2's own design said in
    /// their evidence that they were corroborations, and 9 were reported as
    /// contradictions
    /// (`fact:root-cause-review-relations-cannot-mark-a-contradiction-supporting-and-the-default-says-opposing-2026-10-13`).
    ///
    /// Never inferred from `evidence`: what the writer says here is what is
    /// stored, and nothing reads the prose for it.
    pub properties: BTreeMap<String, Value>,
}

/// The properties a review link may set for `relation`: every property the
/// schema declares on that edge type WITH A DEFAULT, sorted by name. Anything
/// else on a link is refused, naming these.
///
/// Read from the schema rather than listed, so a default added to a review
/// relation is sayable the day it is declared — and
/// `crates/reflow2-mcp/tests/every_defaulted_edge_property_is_exposed_or_exempt.rs`
/// fails until the served surface carries a field for it.
pub fn review_link_properties<'s>(
    schema: &'s Schema,
    relation: &str,
) -> Vec<(&'s str, &'s PropertyDef)> {
    let mut out: Vec<(&str, &PropertyDef)> = schema
        .edge_types
        .get(relation)
        .map(|def| {
            def.properties
                .iter()
                .filter(|(_, p)| p.default.is_some())
                .map(|(k, p)| (k.as_str(), p))
                .collect()
        })
        .unwrap_or_default();
    out.sort_by(|a, b| a.0.cmp(b.0));
    out
}

/// A property value as a person reads it in a refusal.
fn shown(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        other => format!("{other:?}"),
    }
}

/// `` `name` (a, b, c) `` for an enum, `` `name` (true or false) `` for a flag.
fn legal(name: &str, def: &PropertyDef) -> String {
    match (&def.prop_type, &def.values) {
        (PropertyType::Enum, Some(values)) => format!("`{name}` ({})", values.join(", ")),
        (PropertyType::Bool, _) => format!("`{name}` (true or false)"),
        (t, _) => format!("`{name}` ({t:?})"),
    }
}

/// Refuse a per-link property the relation does not declare with a default,
/// or a value the schema does not allow for it. Each refusal names the
/// property and what would have been legal.
fn check_link_properties(schema: &Schema, l: &RelationLink) -> Result<(), DynoError> {
    let takes = review_link_properties(schema, &l.relation);
    for (name, value) in &l.properties {
        let Some((_, def)) = takes.iter().find(|(n, _)| n == name) else {
            let offered = if takes.is_empty() {
                format!("{} takes none", l.relation)
            } else {
                format!(
                    "{} takes {}",
                    l.relation,
                    takes
                        .iter()
                        .map(|(n, d)| legal(n, d))
                        .collect::<Vec<_>>()
                        .join(" and ")
                )
            };
            return Err(DynoError::EdgeValidation {
                edge_type: l.relation.clone(),
                property: name.clone(),
                message: format!(
                    "the {rel} to '{other}' cannot set `{name}`: {rel} does not declare it with \
                     a default. A link sets only the properties its relation would otherwise \
                     default; {offered}.",
                    rel = l.relation,
                    other = l.other_id,
                ),
            });
        };
        let ok = match (&def.prop_type, value) {
            (PropertyType::Enum, Value::String(s)) => def
                .values
                .as_ref()
                .is_none_or(|values| values.iter().any(|v| v == s)),
            (PropertyType::Bool, Value::Bool(_)) => true,
            (PropertyType::String, Value::String(_)) => true,
            _ => false,
        };
        if !ok {
            return Err(DynoError::EdgeValidation {
                edge_type: l.relation.clone(),
                property: name.clone(),
                message: format!(
                    "'{}' is not a {} `{name}` (the link to '{}'): it takes {}.",
                    shown(value),
                    l.relation,
                    l.other_id,
                    legal(name, def)
                ),
            });
        }
    }
    Ok(())
}

/// What a review did.
/// How much of the idea population has been through the linking judgement —
/// and, deliberately, how much of that this can and cannot tell you.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct LinkingReport {
    /// Decisions declaring `kind: exploratory` — the population the discipline
    /// applies to, and the denominator of every figure below.
    pub ideas: usize,
    /// Of those, how many carry at least one review relation. NOT how many were
    /// reviewed — see `not_observed_about`.
    pub linked: usize,
    /// Of those, how many carry `no_relation_note`: somebody looked and
    /// recorded that nothing was honestly related. **A full answer, not a
    /// weaker one**, and the half that separates a judged idea from an
    /// unopened one.
    pub noted: usize,
    /// Neither. For these, "nobody looked" and "looked and found nothing"
    /// cannot be told apart.
    pub silent: usize,
    /// Which ones, so the number is never a bare ratio.
    pub silent_ids: Vec<String>,
    /// Decisions declaring `kind: exploratory`, `choice`, or nothing at all.
    /// The third is a real state, not a gap: absent means nobody said.
    pub exploratory: usize,
    pub choice: usize,
    pub kind_unstated: usize,
    /// What this report cannot see. Carried WITH the numbers rather than left
    /// to the reader, because a caveat that lives only in the tool that
    /// computes it gets stripped by whatever quotes the figure
    /// (`cap:a-derived-number-carries-what-it-was-computed-over`).
    pub not_observed_about: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ReviewOutcome {
    pub node_type: String,
    pub node_id: String,
    /// Relations drawn by this call, as `"CONTRADICTS -> dec:x"`.
    pub drawn: Vec<String>,
    /// Relations this call asked for that were already present. Reported rather
    /// than silently counted as new, so re-running a review does not read as
    /// having found something.
    pub already_present: Vec<String>,
    /// The note stored, when the review drew nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    /// How the node stands after the review.
    pub state: ReviewState,
}

/// The three states a detector has to tell apart. Only the third is a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewState {
    /// Carries at least one relation to another node.
    Linked,
    /// Reviewed, and deliberately not linked — the judgement is on the record.
    ReviewedUnlinked,
}

impl DesignGraph {
    /// Record the relations of one node: draw the honest ones, or say in
    /// writing that there were none.
    ///
    /// # Errors
    ///
    /// Refuses, rather than accepting a half-record:
    ///
    /// - **neither links nor a note** — the whole point is that "I looked and
    ///   found nothing" is an answer, and an answer has to be given;
    /// - an unknown relation, naming the ones that exist;
    /// - a link with no evidence;
    /// - a link to a node that does not exist, or to itself;
    /// - a link property its relation does not declare with a default, or a
    ///   value the schema does not allow for it, naming the legal ones;
    /// - a link property that differs from what an edge already stored for
    ///   the same relation says — it is never dropped as "already present";
    /// - a node that does not exist.
    pub fn review_relations(
        &mut self,
        node_type: &str,
        node_id: &str,
        links: &[RelationLink],
        note: Option<&str>,
    ) -> Result<ReviewOutcome, DynoError> {
        let Some(existing) = self.get_node(node_type, node_id)? else {
            return Err(DynoError::NodeNotFound {
                node_type: node_type.into(),
                node_id: node_id.into(),
            });
        };

        let note = note.map(str::trim).filter(|n| !n.is_empty());
        if links.is_empty() && note.is_none() {
            return Err(DynoError::Validation {
                node_type: node_type.into(),
                property: "no_relation_note".into(),
                message: "a review needs an outcome: either the relations you drew, or a note \
                          saying what you searched and why nothing was honest. Recording neither \
                          leaves this node indistinguishable from one nobody has opened, which \
                          is the state this call exists to rule out."
                    .into(),
            });
        }

        // Validate EVERY link before writing any of them. A review that drew
        // two edges and then refused the third would leave the design holding
        // half a judgement, and the caller with no way to know which half.
        for l in links {
            if !REVIEW_RELATIONS.contains(&l.relation.as_str()) {
                return Err(DynoError::Validation {
                    node_type: node_type.into(),
                    property: "relation".into(),
                    message: format!(
                        "'{}' is not a relation a review can draw (one of {}). Structural edges \
                         like CONTAINS and SATISFIES are design, not commentary, and are written \
                         with their own tools.",
                        l.relation,
                        REVIEW_RELATIONS.join(", ")
                    ),
                });
            }
            if l.evidence.trim().is_empty() {
                return Err(DynoError::Validation {
                    node_type: node_type.into(),
                    property: "evidence".into(),
                    message: format!(
                        "the {} to '{}' needs its reason in one sentence — a relation with no \
                         evidence is an assertion the next reader can neither check nor overturn.",
                        l.relation, l.other_id
                    ),
                });
            }
            if l.other_id == node_id {
                return Err(DynoError::Validation {
                    node_type: node_type.into(),
                    property: "other_id".into(),
                    message: format!("'{node_id}' cannot be related to itself."),
                });
            }
            if self.get_node(&l.other_type, &l.other_id)?.is_none() {
                return Err(DynoError::NodeNotFound {
                    node_type: l.other_type.clone(),
                    node_id: l.other_id.clone(),
                });
            }
            check_link_properties(self.schema(), l)?;
            self.check_against_stored(node_id, l)?;
        }

        let mut drawn = Vec::new();
        let mut already_present = Vec::new();
        for l in links {
            let (from_t, from_i, to_t, to_i): (&str, &str, &str, &str) = if l.incoming {
                (&l.other_type, &l.other_id, node_type, node_id)
            } else {
                (node_type, node_id, &l.other_type, &l.other_id)
            };
            // THE EDGE IS READ BACK AS A SENTENCE, SUBJECT FIRST.
            //
            // flo2, 2026-09-19: they landed two BLOCKS edges asserting the
            // reverse of what their own `evidence` prose said, in one session,
            // and caught both only by re-reading their own call. The direction
            // flag lets a caller draw the edge they meant; it does nothing to
            // confirm they got it right.
            //
            // This label used to read `BLOCKS -> cmp:artifact-store`, which
            // omits the subject and leaves the reader to supply it from memory
            // — and a reader who has just written the call supplies what they
            // INTENDED rather than what they sent. `req:deleting-an-artifact
            // BLOCKS cmp:artifact-store` reads wrong immediately when it is
            // wrong. Every relation is echoed this way rather than only the
            // asymmetric ones, because the label already existed and a uniform
            // shape is easier to read than a conditional one; it is the
            // asymmetric relations (BLOCKS, CAUSES, DEPENDS_ON, EVOLVES_INTO,
            // OBSOLETES) where being able to see it backwards earns the change.
            let label = format!("{from_i} {} {to_i}", l.relation);
            let present = self
                .outgoing(from_i, Some(&l.relation))?
                .into_iter()
                .any(|e| e.to_id == to_i);
            if present {
                already_present.push(label);
                continue;
            }
            // The link's own properties, checked above; anything it does not
            // say is left to the schema default, exactly as before.
            let mut props = Props::new().set("evidence", l.evidence.trim());
            for (k, v) in &l.properties {
                props = props.set(k, v.clone());
            }
            self.create_edge(&l.relation, from_t, from_i, to_t, to_i, props)?;
            drawn.push(label);
        }

        // The note is stored only when the review drew nothing. Where an edge
        // exists the edge IS the record, and a note beside it would be a second
        // account of the same act for a reader to reconcile.
        let store_note = drawn.is_empty() && already_present.is_empty();
        if store_note && let Some(n) = note {
            let mut props = Props::new().set("no_relation_note", n);
            for (k, v) in &existing.properties {
                if k != "no_relation_note" {
                    props = props.set(k, v.clone());
                }
            }
            self.create_node(node_type, node_id, props)?;
        }

        Ok(ReviewOutcome {
            node_type: node_type.to_string(),
            node_id: node_id.to_string(),
            state: if store_note {
                ReviewState::ReviewedUnlinked
            } else {
                ReviewState::Linked
            },
            note: if store_note {
                note.map(str::to_string)
            } else {
                None
            },
            drawn,
            already_present,
        })
    }

    /// A link that names a property value for an edge ALREADY STORED: the
    /// same value is fine (the edge reads as already present); a different one
    /// is refused, before anything is written.
    ///
    /// Reporting it `already_present` would tell a writer who said
    /// `alignment: supporting` that their corroboration was recorded while
    /// the stored edge still says `opposing` — the very misreading this field
    /// exists to end. Revising it here would rewrite a stored judgement with
    /// no record of what it said before. So the refusal names what is stored
    /// and the route that restates it, which is deliberate and visible.
    fn check_against_stored(&self, node_id: &str, l: &RelationLink) -> Result<(), DynoError> {
        if l.properties.is_empty() {
            return Ok(());
        }
        let (from_i, to_i) = if l.incoming {
            (l.other_id.as_str(), node_id)
        } else {
            (node_id, l.other_id.as_str())
        };
        let Some(stored) = self
            .outgoing(from_i, Some(&l.relation))?
            .into_iter()
            .find(|e| e.to_id == to_i)
        else {
            return Ok(());
        };
        let takes = review_link_properties(self.schema(), &l.relation);
        for (name, want) in &l.properties {
            // An edge written before the property existed carries nothing,
            // and reads as the default — which is what every reader sees.
            let have = stored.properties.get(name).cloned().or_else(|| {
                takes
                    .iter()
                    .find(|(n, _)| n == name)
                    .and_then(|(_, d)| d.default.clone())
            });
            if have.as_ref() == Some(want) {
                continue;
            }
            let have = have.as_ref().map_or_else(|| "nothing".to_string(), shown);
            return Err(DynoError::EdgeValidation {
                edge_type: l.relation.clone(),
                property: name.clone(),
                message: format!(
                    "'{from_i}' {rel} '{to_i}' is already stored with `{name}` {have}, and this \
                     link says {want}. review_relations draws a relation and never revises one, \
                     so it will not report this as already present. To restate the stored edge, \
                     call create_edge with edge_type {rel}, the same two ends, and props \
                     carrying `{name}` {want} AND the stored `evidence` again — create_edge \
                     replaces the edge's properties, so evidence left out is lost.",
                    rel = l.relation,
                    want = shown(want),
                ),
            });
        }
        Ok(())
    }

    /// Does this node carry any relation from the review vocabulary, in either
    /// direction?
    ///
    /// Direction-blind on purpose: an idea somebody else's idea CONTRADICTS is
    /// just as connected as one that contradicts something. Asking only about
    /// outgoing edges would report the second idea in every pair as an orphan.
    pub(crate) fn has_review_relation(&self, node_id: &str) -> Result<bool, DynoError> {
        for r in REVIEW_RELATIONS {
            if !self.outgoing(node_id, Some(r))?.is_empty()
                || !self.incoming(node_id, Some(r))?.is_empty()
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Proposed Decisions that carry no relation and no note — the ideas
    /// **nobody has opened**, as distinct from the ones somebody judged and
    /// found genuinely new.
    ///
    /// Excludes decision POINTS (two or more registered alternatives): those
    /// are a fork being weighed rather than a thought nobody has connected, and
    /// `undecided_decision_point` already asks about them. Excludes parked
    /// nodes, which are the recorded answer "this correctly attaches to
    /// nothing".
    /// **Is the linking discipline being followed?** — the question the design
    /// could not answer about itself.
    ///
    /// `unreviewed_ideas` counts the ideas nobody has opened, and that count is
    /// consumed by a detector the owner has ACCEPTED (an unlinked idea is not a
    /// defect — `decision:ack:baa3d7b03a08dedb`, 2026-08-23). So the number was
    /// computed, wired to something deliberately silent, and reachable from
    /// nowhere else. Measured 2026-08-30, that silence hid a real shape: 140 of
    /// 207 ideas carried a relation and `no_relation_note` had been used TWICE,
    /// so for the 67 unlinked ideas "nobody looked" and "looked and found
    /// nothing" were indistinguishable — the exact distinction the note exists
    /// to preserve.
    ///
    /// THIS REPORTS, IT NEVER PRESSES. It raises no gap and changes no
    /// detector: the owner's judgement that an unlinked idea is not a defect
    /// stands untouched. What it removes is the inability to SEE.
    ///
    /// ⚠️ AND IT SAYS WHAT IT CANNOT KNOW, in `not_observed_about`, because the
    /// honest limit is severe: an edge can be drawn by a dozen tools, and
    /// nothing records WHICH drew it. So `linked` counts ideas that have a
    /// relation, never ideas somebody reviewed — the two are not the same, and
    /// the report must not let a reader mistake one for the other.
    pub fn linking_report(&self) -> Result<LinkingReport, DynoError> {
        let mut r = LinkingReport::default();
        for dec in self.scan_nodes(node::DECISION)? {
            let kind = dec
                .properties
                .get("kind")
                .and_then(crate::foundation::core::Value::as_str)
                .map(str::to_string);
            let noted = dec.properties.contains_key("no_relation_note");
            let linked = self.has_review_relation(&dec.node_id)?;
            match kind.as_deref() {
                Some("exploratory") => r.exploratory += 1,
                Some("choice") => r.choice += 1,
                _ => r.kind_unstated += 1,
            }
            // Only ideas carry the discipline. A Decision nobody has classified
            // is counted apart rather than assumed to be one — the same reason
            // the property has no default.
            if kind.as_deref() != Some("exploratory") {
                continue;
            }
            r.ideas += 1;
            if linked {
                r.linked += 1;
            }
            if noted {
                r.noted += 1;
            }
            if !linked && !noted {
                r.silent += 1;
                r.silent_ids.push(dec.node_id.clone());
            }
        }
        r.silent_ids.sort();
        r.not_observed_about = vec![
            "WHETHER THE DISCIPLINE WAS FOLLOWED. An edge can be drawn by any of a dozen tools              and nothing records which drew it, so `linked` counts ideas that HAVE a relation,              never ideas somebody reviewed. A relation drawn in passing by `governed_by` or              `satisfies` is indistinguishable here from one somebody judged."
                .into(),
            "THE IDEAS CARRYING NO `kind`. They are counted in `kind_unstated` and excluded from              every other figure, because their only evidence of being ideas is an id prefix that              was deliberately retired — reading it here would launder it back in."
                .into(),
            "WHETHER A SILENT IDEA IS A PROBLEM. It is not reported as a defect and raises no              gap: an unlinked idea legitimately governs nothing yet, which is the owner's              standing judgement."
                .into(),
        ];
        Ok(r)
    }

    /// Requirements and ACCEPTED decisions linked to no other intent: no review
    /// relation, no DECOMPOSES or GOVERNED_BY either way, and no note saying
    /// the relations were reviewed (F1 of
    /// `req:the-pieces-of-one-picture-are-found-together`). A capability
    /// satisfying a requirement is build, not related intent, so it does not
    /// count. Dropped and met requirements, parked nodes and decision points
    /// with registered alternatives are left out. The open-idea twin is
    /// [`Self::unreviewed_ideas`].
    pub fn unlinked_intent(&self) -> Result<Vec<String>, DynoError> {
        let linked_by = |id: &str| -> Result<bool, DynoError> {
            if self.has_review_relation(id)? {
                return Ok(true);
            }
            for e in [edge::DECOMPOSES, edge::GOVERNED_BY] {
                if !self.outgoing(id, Some(e))?.is_empty()
                    || !self.incoming(id, Some(e))?.is_empty()
                {
                    return Ok(true);
                }
            }
            Ok(false)
        };
        let mut out = Vec::new();
        for r in self.scan_nodes(node::REQUIREMENT)? {
            let status = r
                .properties
                .get("status")
                .and_then(crate::foundation::core::Value::as_str);
            if matches!(status, Some("dropped") | Some("met")) {
                continue;
            }
            if r.properties.contains_key("no_relation_note") || self.is_parked(&r.node_id)? {
                continue;
            }
            if !linked_by(&r.node_id)? {
                out.push(r.node_id.clone());
            }
        }
        for d in self.scan_nodes(node::DECISION)? {
            if d.properties
                .get("status")
                .and_then(crate::foundation::core::Value::as_str)
                != Some("accepted")
            {
                continue;
            }
            if d.properties.contains_key("no_relation_note")
                || self.is_parked(&d.node_id)?
                || self.alternatives_for(&d.node_id)?.len() >= 2
            {
                continue;
            }
            if !linked_by(&d.node_id)? {
                out.push(d.node_id.clone());
            }
        }
        out.sort();
        Ok(out)
    }

    pub fn unreviewed_ideas(&self) -> Result<Vec<String>, DynoError> {
        let mut out = Vec::new();
        for dec in self.scan_nodes(node::DECISION)? {
            if dec
                .properties
                .get("status")
                .and_then(crate::foundation::core::Value::as_str)
                != Some("proposed")
            {
                continue;
            }
            if dec.properties.contains_key("no_relation_note") {
                continue;
            }
            if self.alternatives_for(&dec.node_id)?.len() >= 2 {
                continue;
            }
            if self.is_parked(&dec.node_id)? {
                continue;
            }
            if self.has_review_relation(&dec.node_id)? {
                continue;
            }
            out.push(dec.node_id.clone());
        }
        out.sort();
        Ok(out)
    }
}
