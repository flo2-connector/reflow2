//! REASONING CROSSES DESIGNS, AT THE REGISTRY
//! (`req:reasoning-crosses-designs-within-a-hub`, phase 4b of the 2026-10-18
//! build list).
//!
//! The core holds one design and opens no other (`reflow2_core::far` says
//! why it cannot compute a far node's truth itself). A registry server holds
//! every design under its root, so it is the assembler — the same layer the
//! hub's view lives in (`view.rs`), which is why a local `--registry-root`
//! hub gets this exactly as flo2.io does. Two things happen here:
//!
//! 1. **Far readings, before a call.** A design whose reasoning rests on a
//!    reference to another design's node has that node read THERE before the
//!    call runs ([`ReflowService::far_readings`]): its computed truth, proof
//!    status and what it holds only under, or `stale` / `unreachable`. The
//!    readings are handed to the call through [`FarScoped`], which installs
//!    them in the core's per-call scope on every poll of the call's future,
//!    so they reach every tool that reads a premise (`get_node`,
//!    `reasoning_report`, `detect_gaps`, the change walk …) without a tool
//!    knowing.
//! 2. **A change walk continues, after it ran.** Through a hub's address, a
//!    walk that reached nodes of one design continues into each other design
//!    of the view that holds a reference to one of them
//!    ([`ReflowService::walk_on`]): the radius there runs from those
//!    references, its ids carry that design's prefix, and the references are
//!    named in `boundary_crossings` and the radius carried under
//!    `across_designs`. It goes on from there, each design entered once.
//!
//! WHAT BOUNDS IT, both halves:
//!
//! - **Only what the caller may read.** A far design outside the readable set
//!   the host handed over on THIS call is never opened: its reference reads
//!   `unreachable`, and a walk never enters it, saying nothing of what it
//!   holds (`service::not_readable` is the same test `other_design` applies).
//! - **Only behind a host that establishes the caller.** A registry with no
//!   trusted gateway and no bearer gate reaches no other design, as for
//!   `other_design`; a server that is not a registry reaches none. There the
//!   core reads every reference as unreachable and says why.
//! - **One design's guard at a time.** Every far design is read under its own
//!   read guard alone (`RegistryHandle::read_other_design`), never while this
//!   design's is held: the readings are taken before the call takes its own,
//!   and the walk continues after the radius was computed and its guard
//!   dropped. That is what keeps A→B against B→A from deadlocking on tokio's
//!   writer-preferring `RwLock`, as in the view.
//! - **Per call, never stored.** Nothing read across is written into either
//!   design; two callers with different readable sets get different readings.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use reflow2_core::far::{FarReading, FarReadings};
use reflow2_core::{BlastRadius, DesignGraph, PropagateOptions};
use serde_json::{Value as JsonValue, json};

use super::{READABLE_GRAPH_IDS, ReflowService, not_readable};
use crate::registry_http::{Refusal, RegistryHandle};

tokio::task_local! {
    /// The designs a change walk may continue into on the call now being
    /// served: the hub and its members the caller may read, set by the hub's
    /// view (`view::serve`) around every call it makes on one design. `None`
    /// on a call that did not come through a hub's address, where a walk stays
    /// in its own design as before.
    pub(crate) static WALK_AMONG: Option<Vec<String>>;
}

/// A call's future, with the call's far readings installed in the core's
/// per-call scope (`reflow2_core::far::scoped`) for every poll — so every
/// synchronous step the handler takes, on whatever worker thread polls it,
/// reads them, and nothing outside the call ever does.
pub(crate) struct FarScoped<F> {
    readings: Option<Arc<FarReadings>>,
    inner: Pin<Box<F>>,
}

impl<F> FarScoped<F> {
    pub(crate) fn new(readings: Option<Arc<FarReadings>>, inner: F) -> Self {
        FarScoped {
            readings,
            inner: Box::pin(inner),
        }
    }
}

impl<F: Future> Future for FarScoped<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        let this = self.get_mut();
        let readings = this.readings.clone();
        reflow2_core::far::scoped(readings, || this.inner.as_mut().poll(cx))
    }
}

impl ReflowService {
    /// The registry, when this server may read another design for this call:
    /// a registry behind a trusted gateway or a bearer gate (the admission
    /// `other_design` applies). `None` anywhere else.
    fn reaches_other_designs(&self) -> Option<&RegistryHandle> {
        let registry = self.registry.as_ref()?;
        match self.caller_rule.as_ref() {
            crate::caller::CallerRule::TrustedGateway { .. }
            | crate::caller::CallerRule::BearerToken { .. } => Some(registry),
            crate::caller::CallerRule::Local | crate::caller::CallerRule::Undeclared { .. } => None,
        }
    }

    /// What every reference in this design's reasoning reads as in its own
    /// design, for one call whose caller may read `readable`. `None` — and the
    /// call costs nothing — when this server reaches no other design or this
    /// design's reasoning names none.
    pub(crate) async fn far_readings(
        &self,
        readable: Option<&[String]>,
    ) -> Option<Arc<FarReadings>> {
        let registry = self.reaches_other_designs()?;
        let refs = {
            let g = self.graph.read().await;
            g.reasoning_references().ok()?
            // this design's guard drops here, before any other is taken
        };
        if refs.is_empty() {
            return None;
        }
        Some(Arc::new(read_far_ends(registry, refs, readable).await))
    }

    /// Carry a change walk on from this design into every design of the hub's
    /// view that holds a reference to a node it reached, and on from there.
    /// `None` when the call did not come through a hub's address or this
    /// server reaches no other design; the reply is then exactly as it was.
    /// Returns what to add to the reply: `across_designs` and the references
    /// to name in `boundary_crossings`, each carrying its design's prefix.
    pub(crate) async fn walk_on(
        &self,
        radius: &BlastRadius,
        opts: PropagateOptions,
    ) -> Option<(Vec<JsonValue>, Vec<String>)> {
        let among = WALK_AMONG.try_with(Clone::clone).ok().flatten()?;
        let registry = self.reaches_other_designs()?;
        let readable = READABLE_GRAPH_IDS.try_with(Clone::clone).ok().flatten();
        let mut entered: BTreeSet<String> = BTreeSet::from([radius.design.clone()]);
        let mut queue: VecDeque<(String, BTreeSet<String>)> =
            VecDeque::from([(radius.design.clone(), reached(radius))]);
        let mut across = Vec::new();
        let mut crossings = Vec::new();
        while let Some((from, reached_there)) = queue.pop_front() {
            for design in &among {
                if entered.contains(design) || not_readable(design, readable.as_deref()).is_some() {
                    continue;
                }
                // Which references this design holds to what the walk
                // reached, and what its reasoning reads across, under its
                // own guard alone.
                let listed = registry
                    .read_other_design(design, |g| {
                        Ok::<_, reflow2_core::DynoError>((
                            g.references_into(&from, &reached_there)?,
                            g.reasoning_references()?,
                        ))
                    })
                    .await;
                let (into, refs) = match listed {
                    Ok(Ok(found)) => found,
                    Ok(Err(e)) => {
                        across.push(not_walked(design, &from, e.to_string()));
                        continue;
                    }
                    Err(refusal) => {
                        across.push(not_walked(design, &from, refusal_words(design, &refusal)));
                        continue;
                    }
                };
                if into.is_empty() {
                    continue;
                }
                entered.insert(design.clone());
                // Its premises across designs read as they read now, so the
                // walk compares before and after against the truth, not
                // against "unreachable".
                let readings = Some(Arc::new(
                    read_far_ends(registry, refs, readable.as_deref()).await,
                ));
                let seeds = into.clone();
                let walked = registry
                    .read_other_design(design, |g: &DesignGraph| {
                        reflow2_core::far::scoped(readings, || {
                            let seeds: Vec<&str> = seeds.iter().map(String::as_str).collect();
                            g.propagate_from(&seeds, opts)
                        })
                    })
                    .await;
                let there = match walked {
                    Ok(Ok(r)) => r,
                    Ok(Err(e)) => {
                        across.push(not_walked(design, &from, e.to_string()));
                        continue;
                    }
                    Err(refusal) => {
                        across.push(not_walked(design, &from, refusal_words(design, &refusal)));
                        continue;
                    }
                };
                let entered_at: Vec<String> =
                    into.iter().map(|id| format!("{design}:{id}")).collect();
                crossings.extend(entered_at.iter().cloned());
                let mut summary =
                    serde_json::to_value(there.summarize()).unwrap_or(JsonValue::Null);
                super::view::prefix_ids(&mut summary, design);
                across.push(json!({
                    "design": design,
                    "from_design": from,
                    "entered_at": entered_at,
                    "why": format!(
                        "design {design} holds a reference to what the walk reached in {from}, \
                         so the walk continues there from it"
                    ),
                    "radius": summary,
                }));
                queue.push_back((design.clone(), reached(&there)));
            }
        }
        Some((across, crossings))
    }
}

impl ReflowService {
    /// THROUGH A HUB, A CHANGE WALK CONTINUES INTO EVERY DESIGN OF THE VIEW
    /// THAT HOLDS A REFERENCE TO WHAT IT REACHED
    /// (req:reasoning-crosses-designs-within-a-hub, part 2; `service::far`):
    /// each such reference is named in `boundary_crossings` with its design's
    /// prefix, and the radius there is carried under `across_designs`. A call
    /// that did not come through a hub's address is left exactly as it was.
    pub(crate) async fn carry_walk_on(
        &self,
        radius: &BlastRadius,
        opts: PropagateOptions,
        out: &mut serde_json::Value,
    ) {
        let Some((across, crossings)) = self.walk_on(radius, opts).await else {
            return;
        };
        let Some(obj) = out.as_object_mut() else {
            return;
        };
        if let Some(serde_json::Value::Array(named)) = obj.get_mut("boundary_crossings") {
            named.extend(crossings.into_iter().map(serde_json::Value::String));
        }
        obj.insert("across_designs".into(), serde_json::Value::Array(across));
    }
}

/// Every node a radius stood on or reached.
fn reached(radius: &BlastRadius) -> BTreeSet<String> {
    radius
        .seeds
        .iter()
        .cloned()
        .chain(radius.impacted.iter().map(|n| n.node_id.clone()))
        .collect()
}

/// What a walk says about a design it could not look into.
fn not_walked(design: &str, from: &str, why: String) -> JsonValue {
    json!({
        "design": design,
        "from_design": from,
        "not_walked": why,
    })
}

/// A registry refusal, in words that name the design and nothing it holds.
fn refusal_words(design: &str, refusal: &Refusal) -> String {
    match refusal {
        Refusal::NotADesign(_) => format!("no design {design} is served here"),
        Refusal::Busy(why) | Refusal::CouldNotOpen(why) => {
            format!("design {design} could not be read right now: {why}")
        }
    }
}

/// Read each reference's far node in its own design, one design's guard at a
/// time, for a caller who may read `readable`.
async fn read_far_ends(
    registry: &RegistryHandle,
    refs: Vec<(String, String, String)>,
    readable: Option<&[String]>,
) -> FarReadings {
    let mut by_design: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (xref, design, node) in refs {
        by_design.entry(design).or_default().push((xref, node));
    }
    let mut readings = FarReadings::new();
    for (design, wanted) in by_design {
        let unreachable = |why: String, readings: &mut FarReadings| {
            for (xref, node) in &wanted {
                readings.insert(
                    xref.clone(),
                    FarReading::unreachable(
                        &design,
                        node,
                        format!(
                            "unreachable: {why}, so {node} there is unknown here, never read as \
                             holding"
                        ),
                    ),
                );
            }
        };
        if not_readable(&design, readable).is_some() {
            unreachable(
                format!("design {design} is not among the designs this call may read"),
                &mut readings,
            );
            continue;
        }
        let nodes: Vec<String> = wanted.iter().map(|(_, n)| n.clone()).collect();
        let read = registry
            .read_other_design(&design, |g| {
                nodes
                    .iter()
                    .map(|n| g.read_for_another_design(n))
                    .collect::<Result<Vec<_>, _>>()
            })
            .await;
        match read {
            Ok(Ok(found)) => {
                for ((xref, _), reading) in wanted.iter().zip(found) {
                    readings.insert(xref.clone(), reading);
                }
            }
            Ok(Err(e)) => unreachable(
                format!("design {design} could not be read ({e})"),
                &mut readings,
            ),
            Err(refusal) => unreachable(refusal_words(&design, &refusal), &mut readings),
        }
    }
    readings
}
