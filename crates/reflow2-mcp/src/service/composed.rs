//! ONE COMPOSED CHECK OVER A HUB AND SEVERAL OF ITS DESIGNS —
//! `req:one-composed-check-runs-over-a-hub-and-several-of-its-designs`, phase
//! 7's third step (`epoch:planned-p7-a-hub-on-one-machine`).
//!
//! `compose_and_analyse` composed THIS design with ONE other, so a hub effort
//! spanning three to six designs (hxm_program's hub of fifteen, reached only
//! through `reflow2 read|write`) was run pairwise and stitched by hand, and the
//! seam between two members was never analysed unless one of them was the
//! design calling. Here it takes several at once — `other_graph_ids`, or
//! `tracked_designs: true` for every design this one pins — composes them all
//! into ONE combined graph (`reflow2_core::DesignGraph::compose_many`), and
//! attributes each finding to the designs it names: one design, a pair, or
//! more.
//!
//! WHERE EACH DESIGN IS READ FROM, by where this design is served:
//!
//! - **Served alone** (stdio, the `--call` door, `--shared`): each design must
//!   be one this design PINS (`external_dependency` with its graph_id), and is
//!   read from the pin's committed export through `super::tracked`'s one
//!   resolution of a pin (`read_document`), so a design that could not be read
//!   is named in `not_composed` with the tracked search's own `state`s
//!   (`missing`, `unreadable`, `not_reached`, `nowhere`, `not_read`). The
//!   DOCUMENT is read, not a searchable design: composing needs only the
//!   export, and building a searchable design per member was most of what a
//!   tracked search costs.
//! - **On a server holding many designs** (a registry behind a trusted host,
//!   flo2.io): each is read from its LIVE STORE by graph_id through the one
//!   door, `ReflowService::other_design_named`, inside the set the host says
//!   the caller may read; one refused is named `unreachable` with the door's
//!   sentence. Through a hub's address, `tracked_designs` names every member
//!   of the view (`super::view`).
//!
//! 🛑 NOTHING IS WRITTEN AND NOTHING IS FETCHED. The combined graph is built in
//! memory and thrown away; no design, export or member store changes, no git
//! runs, no network is used. One design's guard at a time: each other design
//! is read (its guard taken and dropped inside) before this design's is taken.

use std::collections::BTreeMap;

use rmcp::ErrorData as McpError;
use rmcp::model::CallToolResult;
use serde_json::{Map as JsonMap, Value as JsonValue, json};

use super::tracked::{self, Pin};
use super::{ReflowService, dyno_err, ok_json};
use crate::service::ComposeReq;

/// The argument naming several designs to compose at once.
pub(crate) const OTHER_GRAPH_IDS: &str = "other_graph_ids";

/// The argument composing every design this one pins.
pub(crate) const TRACKED_DESIGNS: &str = "tracked_designs";

/// How a pair of designs is written as one key: `a×b`, in composition order.
pub(crate) const PAIR_JOIN: &str = "×";

/// Whether a `compose_and_analyse` call asks for several designs at once —
/// the shape this module serves; any other call keeps the one-other-design
/// path, byte for byte.
pub(crate) fn asks_for_several(req: &ComposeReq) -> bool {
    req.other_graph_ids.is_some() || req.tracked_designs == Some(true)
}

/// One design to compose: its graph_id, and its pin when this design pins it.
struct Target {
    design: String,
    pin: Option<Pin>,
}

/// One design read for the composition.
struct Loaded {
    design: String,
    doc: reflow2_core::GraphExport,
    read_from: String,
}

fn refuse(why: String) -> McpError {
    McpError::invalid_params(format!("REFUSED, and nothing ran: {why}"), None)
}

impl ReflowService {
    /// `compose_and_analyse` over this design and several others at once.
    pub(crate) async fn compose_several(
        &self,
        req: ComposeReq,
    ) -> Result<CallToolResult, McpError> {
        // ONE SHAPE PER CALL: each design is namespaced by its graph_id, so
        // the one-other-design arguments have nothing to say here.
        for (given, argument) in [
            (req.design.is_some(), "design"),
            (req.other_graph_id.is_some(), "other_graph_id"),
            (req.namespace.is_some(), "namespace"),
        ] {
            if given {
                return Err(refuse(format!(
                    "`{argument}` composes ONE other design, and this call names several \
                     (`{OTHER_GRAPH_IDS}` or `{TRACKED_DESIGNS}`). Pass one shape: several \
                     designs, each namespaced by its own graph_id, or one other design."
                )));
            }
        }
        let tracked = req.tracked_designs == Some(true);
        let named: Option<Vec<String>> = match req.other_graph_ids {
            None => None,
            Some(_) if tracked => {
                return Err(refuse(format!(
                    "pass `{OTHER_GRAPH_IDS}` (the designs named) OR `{TRACKED_DESIGNS}: true` \
                     (every design this one pins), not both."
                )));
            }
            Some(ids) => {
                let mut out: Vec<String> = Vec::new();
                for id in ids {
                    let id = id.trim().to_string();
                    if id.is_empty() {
                        return Err(refuse(format!(
                            "`{OTHER_GRAPH_IDS}` holds an empty graph_id."
                        )));
                    }
                    if out.contains(&id) {
                        return Err(refuse(format!(
                            "`{OTHER_GRAPH_IDS}` names design {id} twice; each design is \
                             composed once."
                        )));
                    }
                    out.push(id);
                }
                if out.is_empty() {
                    return Err(refuse(format!(
                        "`{OTHER_GRAPH_IDS}` names no design. Name at least one, or pass \
                         `{TRACKED_DESIGNS}: true` for every design this one pins."
                    )));
                }
                Some(out)
            }
        };
        let budget = req
            .budget_chars
            .unwrap_or(crate::reply_budget::DEFAULT_REPLY_BUDGET_CHARS);

        // This design's own pins, its guard dropped before any other is read.
        let (this, pins) = {
            let g = self.graph.read().await;
            let declared = g.declared_dependencies().map_err(dyno_err)?;
            (g.graph_id().to_string(), tracked::pins_of(&declared))
        };
        let on_registry = self.served_from_registry();
        let targets: Vec<Target> = match &named {
            None => pins
                .iter()
                .filter(|p| p.design != this)
                .map(|p| Target {
                    design: p.design.clone(),
                    pin: Some(p.clone()),
                })
                .collect(),
            Some(ids) => {
                let mut out = Vec::new();
                for id in ids {
                    if *id == this {
                        return Err(refuse(format!(
                            "`{OTHER_GRAPH_IDS}` names {id}, this design itself, which is \
                             always composed; name the other designs only."
                        )));
                    }
                    let pin = pins.iter().find(|p| &p.design == id).cloned();
                    if pin.is_none() && !on_registry {
                        let pinned: Vec<&str> = pins.iter().map(|p| p.design.as_str()).collect();
                        return Err(refuse(format!(
                            "`{OTHER_GRAPH_IDS}` names design {id}, which this design ({this}) \
                             does not pin. On a design served alone each design composed is \
                             one it pins (external_dependency with that graph_id), read from \
                             its committed export: {}.",
                            if pinned.is_empty() {
                                "it pins none".to_string()
                            } else {
                                pinned.join(", ")
                            }
                        )));
                    }
                    out.push(Target {
                        design: id.clone(),
                        pin,
                    });
                }
                out
            }
        };

        // Read each, one design's guard at a time, naming every one not read.
        let argument = if tracked {
            TRACKED_DESIGNS
        } else {
            OTHER_GRAPH_IDS
        };
        let mut loaded: Vec<Loaded> = Vec::new();
        let mut not_composed: Vec<JsonValue> = Vec::new();
        let mut names = JsonMap::new();
        for (i, t) in targets.iter().enumerate() {
            if let Some(pin) = &t.pin {
                names.insert(t.design.clone(), JsonValue::String(pin.name.clone()));
            }
            if i >= crate::upstream::MAX_UPSTREAMS_READ {
                not_composed.push(entry(
                    t,
                    "not_read",
                    &format!(
                        "this composition reads at most {} other designs, because each is a \
                         whole document loaded for the call; compose it by naming it in \
                         `{OTHER_GRAPH_IDS}` with fewer others",
                        crate::upstream::MAX_UPSTREAMS_READ
                    ),
                ));
                continue;
            }
            if on_registry {
                match self.other_design_named(argument, &t.design).await {
                    Ok(doc) => loaded.push(Loaded {
                        design: t.design.clone(),
                        doc,
                        read_from: "its live store on this server".to_string(),
                    }),
                    Err(e) => not_composed.push(entry(t, "unreachable", &e.message)),
                }
                continue;
            }
            let pin = t
                .pin
                .as_ref()
                .expect("served alone, every target is pinned");
            match tracked::read_document(pin) {
                Ok((doc, path)) => loaded.push(Loaded {
                    design: t.design.clone(),
                    doc,
                    read_from: path,
                }),
                Err((state, why)) => {
                    not_composed.push(tracked::not_searched_entry(pin, state, &why));
                }
            }
        }

        let report = {
            let g = self.graph.read().await;
            let with: Vec<reflow2_core::ComposeWith<'_>> = loaded
                .iter()
                .map(|l| reflow2_core::ComposeWith {
                    design: &l.design,
                    export: &l.doc,
                })
                .collect();
            g.compose_many(&with).map_err(dyno_err)?
        };
        let read_from: JsonMap<String, JsonValue> = loaded
            .iter()
            .map(|l| (l.design.clone(), JsonValue::String(l.read_from.clone())))
            .collect();
        let tracked_note = (tracked && pins.iter().all(|p| p.design == this)).then(|| {
            "This design pins no other design (no external_dependency here names a graph_id), \
             so `tracked_designs` had nothing to add: the composition is this design alone."
                .to_string()
        });
        let shaped = shape(
            report,
            names,
            read_from,
            not_composed,
            on_registry,
            tracked_note,
        );
        ok_json(crate::reply_budget::bound_reply_sampling_only(
            shaped,
            budget,
            "Every count is still here (`counts`). A shortened list is a sample: name fewer \
             designs in `other_graph_ids` (a pair alone gives that seam in full), or raise \
             `budget_chars` if this client has the room. Open a finding's id with get_node: \
             another design's id carries its graph_id prefix.",
            &["by_design.", "by_pair.", "across", "of_the_whole"],
        ))
    }
}

/// One design not composed, in the tracked search's `not_searched` shape when
/// it is pinned, and with what is known when it is not.
fn entry(t: &Target, state: &str, why: &str) -> JsonValue {
    match &t.pin {
        Some(pin) => tracked::not_searched_entry(pin, state, why),
        None => json!({"design": t.design, "state": state, "why": why}),
    }
}

/// The composed report, attributed: per design, per pair, across more, and
/// of the whole; every count; every design read and every one not.
fn shape(
    report: reflow2_core::ManyComposedReport,
    names: JsonMap<String, JsonValue>,
    read_from: JsonMap<String, JsonValue>,
    not_composed: Vec<JsonValue>,
    on_registry: bool,
    tracked_note: Option<String>,
) -> JsonValue {
    let this = report.this_design.clone();
    let order: Vec<String> = std::iter::once(this.clone())
        .chain(report.composed.iter().map(|c| c.design.clone()))
        .collect();
    let mut by_design: BTreeMap<usize, (String, Vec<JsonValue>)> = order
        .iter()
        .enumerate()
        .map(|(i, d)| (i, (d.clone(), Vec::new())))
        .collect();
    let mut by_pair: BTreeMap<(usize, usize), (String, Vec<JsonValue>)> = BTreeMap::new();
    let mut across: Vec<JsonValue> = Vec::new();
    let mut whole: Vec<JsonValue> = Vec::new();
    let position = |d: &str| order.iter().position(|o| o == d).unwrap_or(usize::MAX);
    for f in &report.findings {
        let one = json!({
            "kind": f.kind,
            "source": f.source,
            "title": f.title,
            "affected_ids": f.affected_ids,
        });
        match f.designs.as_slice() {
            [] => whole.push(one),
            [d] => {
                if let Some((_, list)) = by_design.get_mut(&position(d)) {
                    list.push(one);
                }
            }
            [a, b] => {
                by_pair
                    .entry((position(a), position(b)))
                    .or_insert_with(|| (format!("{a}{PAIR_JOIN}{b}"), Vec::new()))
                    .1
                    .push(one);
            }
            several => {
                let mut one = one;
                one["designs"] = json!(several);
                across.push(one);
            }
        }
    }

    let mut counts_by_design = JsonMap::new();
    let mut findings_by_design = JsonMap::new();
    for (_, (d, list)) in by_design {
        counts_by_design.insert(d.clone(), json!(list.len()));
        if !list.is_empty() {
            findings_by_design.insert(d, JsonValue::Array(list));
        }
    }
    let mut counts_by_pair = JsonMap::new();
    let mut findings_by_pair = JsonMap::new();
    for (_, (key, list)) in by_pair {
        counts_by_pair.insert(key.clone(), json!(list.len()));
        findings_by_pair.insert(key, JsonValue::Array(list));
    }
    let n = order.len();
    let pairs_composed = n * n.saturating_sub(1) / 2;
    let total = report.findings.len();
    let k = report.composed.len();
    let missed = not_composed.len();

    let opened = if on_registry {
        "read from its live store on this server, inside the set the host says you may read"
    } else {
        "read from its committed export on this machine, as of when it was exported"
    };
    let not_sentence = if missed == 0 {
        "Every design asked for was composed.".to_string()
    } else {
        format!(
            "{missed} design(s) asked for were NOT composed, each named in `not_composed` with \
             its `state` and why, so nothing here speaks for them."
        )
    };
    let note = format!(
        "compose_and_analyse over this design ({this}) and {k} other design(s), each {opened}, \
         in ONE combined graph built in memory and thrown away: no design, export or store was \
         written. Each other design's ids carry its graph_id ('<graph_id>:<id>', which get_node \
         opens); this design's are unprefixed. `by_design` holds the findings naming one design \
         only, `by_pair` those naming exactly two ('a{PAIR_JOIN}b'), `across` three or more, and \
         `of_the_whole` none (about the combined graph, which no one design owns). ⚠ Each design \
         is composed under its own namespace, so no two share a node: a pair appears in \
         `by_pair` only where a check reads both at once, and a pair absent from it was not \
         found to touch, which is not agreement. {not_sentence}"
    );

    let composed: Vec<JsonValue> = report
        .composed
        .iter()
        .map(|c| {
            json!({
                "design": c.design,
                "nodes": c.nodes,
                "edges": c.edges,
                "skipped_edges": c.skipped_edges,
            })
        })
        .collect();
    let mut out = json!({
        "this_design": this,
        "this_nodes": report.this_nodes,
        "designs_composed": order,
        "design_names": names,
        "read_from": read_from,
        "composed": composed,
        "counts": {
            "total": total,
            "by_design": counts_by_design,
            "by_pair": counts_by_pair,
            "across": across.len(),
            "of_the_whole": whole.len(),
            "pairs_composed": pairs_composed,
        },
        "by_design": findings_by_design,
        "by_pair": findings_by_pair,
        "across": across,
        "of_the_whole": whole,
        "not_composed": not_composed,
        "composed_note": note,
    });
    if let Some(note) = tracked_note {
        out["tracked_note"] = JsonValue::String(note);
    }
    if total == 0 {
        out["empty_because"] = JsonValue::String(format!(
            "the ordinary checks (detect_gaps and detect_defects) found nothing in the combined \
             graph of {n} design(s): this design and {k} other(s). {not_sentence}"
        ));
    }
    out
}
