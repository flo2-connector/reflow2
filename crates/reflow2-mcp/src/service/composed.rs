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
pub(crate) const PAIR_JOIN: &str = reflow2_core::touch::PAIR_JOIN;

/// The sentence the composed reply carries on Anthony's choice (a) of
/// `dec:idea-a-composed-check-says-which-findings-only-the-composition-made`:
/// a design beside others it has no link to reads as a cut-off cluster.
pub(crate) const CUT_OFF_SENTENCE: &str = "A design with no link to the others reads, beside \
     them, as a cut-off cluster (`unthreaded_cluster`, \"no traceability edge to the main body\", \
     the main body being the largest design): a finding the composition made, which that design \
     alone does not have. Links (below) thread linked designs into one body; a pin that nothing \
     in its own design is wired to threads only itself to the design it names.";

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
        ok_json(bound_composed(
            shaped,
            budget,
            "Every count is still here (`counts`, and `link_status.counts`). A shortened list \
             is a sample: name fewer designs in `other_graph_ids` (a pair alone gives that seam \
             in full), or raise `budget_chars` if this client has the room. Open a finding's id \
             with get_node: another design's id carries its graph_id prefix.",
            &[
                "by_design.",
                "by_pair.",
                "across",
                "of_the_whole",
                "declared_links",
                "computed_links",
                "candidates",
                "link_status.linked",
                "link_status.candidates_only",
                "link_status.never_touch",
                "link_status.not_checked",
            ],
            &["by_design", "by_pair"],
            &["across", "of_the_whole"],
        ))
    }
}

/// THE COMPOSED REPLY HOLDS ITS BUDGET (measured 2026-10-22: 58,505
/// characters against 30,000 on a hub of fifteen exports, every list at its
/// floor of one). The generic sampler keeps a whole entry per list and never
/// cuts an id, and a composition breaks both: one `by_pair` key per pair, each
/// holding a finding that names a whole cut-off cluster. So, only when the
/// sampler alone cannot fit, the reply is reduced in rungs before it samples,
/// each rung named in `withheld_to_fit`:
///
/// 1. each finding's `affected_ids` are cut to the first few (20, then 5,
///    then 1), the finding keeping `affected_ids_total`;
/// 2. whole keys of the per-design and per-pair maps are withheld (keeping
///    at most 8, then 3, then 1, then none), every key's count staying in
///    `counts`.
///
/// Counts, what was composed and what was not are never cut. A reply that
/// fits is returned untouched.
pub(crate) fn bound_composed(
    full: JsonValue,
    budget: usize,
    hint: &str,
    lists: &[&str],
    finding_maps: &[&str],
    finding_lists: &[&str],
) -> JsonValue {
    use crate::reply_budget::{bound_reply_sampling_only, chars_of};
    let fits = |v: &JsonValue| chars_of(v) <= budget;
    if fits(&full) {
        return full;
    }
    let first = bound_reply_sampling_only(full.clone(), budget, hint, lists);
    if fits(&first) {
        return first;
    }
    // (ids kept per finding, map keys kept per map)
    const RUNGS: [(usize, Option<usize>); 7] = [
        (20, None),
        (5, None),
        (5, Some(8)),
        (1, Some(8)),
        (1, Some(3)),
        (1, Some(1)),
        (1, Some(0)),
    ];
    let mut last = first;
    for (ids, keys) in RUNGS {
        let mut body = full.clone();
        let mut findings_cut = 0usize;
        let mut ids_withheld = 0usize;
        let mut cut = |f: &mut JsonValue| {
            let Some(list) = f.get_mut("affected_ids").and_then(JsonValue::as_array_mut) else {
                return;
            };
            let total = list.len();
            if total > ids {
                list.truncate(ids);
                findings_cut += 1;
                ids_withheld += total - ids;
                f["affected_ids_total"] = json!(total);
            }
        };
        for m in finding_maps {
            if let Some(map) = body.get_mut(*m).and_then(JsonValue::as_object_mut) {
                for list in map.values_mut() {
                    if let Some(l) = list.as_array_mut() {
                        l.iter_mut().for_each(&mut cut);
                    }
                }
            }
        }
        for l in finding_lists {
            if let Some(list) = body.get_mut(*l).and_then(JsonValue::as_array_mut) {
                list.iter_mut().for_each(&mut cut);
            }
        }
        let mut withheld = json!({
            "affected_ids": {
                "shown_per_finding": ids,
                "findings_cut": findings_cut,
                "ids_withheld": ids_withheld,
                "note": "A finding whose ids were cut keeps `affected_ids_total`; run the \
                         composition over fewer designs, or raise `budget_chars`, to read \
                         them all.",
            }
        });
        if let Some(keep) = keys {
            for m in finding_maps {
                if let Some(map) = body.get_mut(*m).and_then(JsonValue::as_object_mut) {
                    let total = map.len();
                    if total > keep {
                        let drop: Vec<String> = map.keys().skip(keep).cloned().collect();
                        for k in drop {
                            map.remove(&k);
                        }
                        withheld[*m] = json!({
                            "keys_shown": keep,
                            "keys_total": total,
                            "note": "the withheld keys and their counts are in `counts`",
                        });
                    }
                }
            }
        }
        body["withheld_to_fit"] = withheld;
        let mut out = bound_reply_sampling_only(body, budget, hint, lists);
        if out.get("budget").is_none() {
            // The rung alone made it fit, so the sampler had nothing to say:
            // the reply still says it was bounded, and where.
            out["budget"] = json!({
                "applied": true,
                "budget_chars": budget,
                "full_chars": chars_of(&full),
                "note": format!(
                    "WITHHELD TO FIT. The whole answer is {} characters against a budget of \
                     {budget}: `withheld_to_fit` says what was cut. {hint}",
                    chars_of(&full)
                ),
            });
        }
        if fits(&out) {
            return out;
        }
        last = out;
    }
    last
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

    // WHERE THE DESIGNS TOUCH, and every pair's status — the not-composed
    // designs' pairs included, as `not_checked`.
    let mut linked: Vec<JsonValue> = Vec::new();
    let mut candidates_only: Vec<JsonValue> = Vec::new();
    let mut never_touch: Vec<JsonValue> = Vec::new();
    let mut not_checked: Vec<JsonValue> = Vec::new();
    for p in &report.pairs {
        let key = format!("{}{PAIR_JOIN}{}", p.a, p.b);
        match p.status {
            "linked" => {
                linked.push(json!({"pair": key, "declared": p.declared,
                                   "computed": p.computed, "candidates": p.candidates}));
            }
            "candidates_only" => {
                candidates_only.push(json!({"pair": key, "candidates": p.candidates}));
            }
            _ => never_touch.push(JsonValue::String(key)),
        }
    }
    // A pair with a design not composed was not checked; `not_composed`
    // says why that design was not read.
    let missing: Vec<String> = not_composed
        .iter()
        .filter_map(|n| Some(n.get("design")?.as_str()?.to_string()))
        .collect();
    for (i, m) in missing.iter().enumerate() {
        for d in order.iter().chain(&missing[i + 1..]) {
            let key = if order.contains(d) {
                format!("{d}{PAIR_JOIN}{m}")
            } else {
                format!("{m}{PAIR_JOIN}{d}")
            };
            not_checked.push(JsonValue::String(key));
        }
    }
    let link_counts = json!({
        "linked": linked.len(),
        "candidates_only": candidates_only.len(),
        "never_touch": never_touch.len(),
        "not_checked": not_checked.len(),
    });
    let declared_links: Vec<&reflow2_core::TouchLink> = report
        .touching
        .links
        .iter()
        .filter(|l| l.link == "declared")
        .collect();
    let computed_links: Vec<&reflow2_core::TouchLink> = report
        .touching
        .links
        .iter()
        .filter(|l| l.link == "computed")
        .collect();
    let links_counted = json!({
        "declared": declared_links.len(),
        "computed": computed_links.len(),
        "candidates": report.touching.candidates.len(),
    });

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
         `of_the_whole` none (about the combined graph, which no one design owns). Each design \
         is composed under its own namespace, so no two share a node: they touch only through \
         `declared_links` and `computed_links`, each drawn as an edge labelled `declared` or \
         `computed` before the checks ran, so a finding across one is filed `by_pair`. \
         ⚠ `link_status` says for EVERY pair whether it is `linked`, `candidates_only`, \
         `never_touch` (checked: no evidence) or `not_checked`; a pair absent from `by_pair` \
         is never agreement. {CUT_OFF_SENTENCE} {not_sentence}"
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
            "links": links_counted,
            "link_status": link_counts,
        },
        "by_design": findings_by_design,
        "by_pair": findings_by_pair,
        "across": across,
        "of_the_whole": whole,
        "not_composed": not_composed,
        "link_status": {
            "counts": link_counts.clone(),
            "linked": linked,
            "candidates_only": candidates_only,
            "never_touch": never_touch,
            "not_checked": not_checked,
        },
        "declared_links": declared_links,
        "computed_links": computed_links,
        "candidates": report.touching.candidates,
        "set_aside": report.touching.set_aside,
        "repositories": report.touching.repositories,
        "links_note": reflow2_core::LINKS_NOTE,
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
