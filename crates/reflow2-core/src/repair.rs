//! What still rests on a patch — clause (c) of
//! `req:a-fix-says-whether-it-corrected-the-cause`.
//!
//! # The question this answers, in the words it was asked in
//!
//! Anthony, 2026-08-17: *"I'm not sure how many fixes have been patches versus
//! going back to the drawing board, planning how it should have been done
//! correctly, and then re-implementing proper fix."* Until this existed, the
//! design could not answer it — measured on this project's own graph the same
//! day: **472 ChangeEvents across eleven change types, every one naming what
//! MOVED and not one saying whether it was the RIGHT fix.**
//!
//! # ⭐ Why the UNSTATED count is the load-bearing number
//!
//! A report that listed the contained symptoms and stopped would read as *"two
//! things rest on a patch"* on a design where two hundred repairs said nothing
//! at all. That is the exact shape of
//! `req:a-report-says-what-it-swept-and-whether-its-checks-ran` — a bare list
//! cannot distinguish EXERCISED AND FOUND NOTHING from HAD NOTHING TO EXAMINE,
//! and an empty one reads as the first while usually being the second.
//!
//! So `unstated` is a field, not a footnote, and on a design that has just
//! adopted the field it will dominate. **That is correct and it is the point:**
//! the honest first answer to "what rests on a patch?" is "nobody has said, for
//! 217 of 220 repairs", and a report that hid that would be worse than no
//! report.
//!
//! # Why no detector, and no grandfathering machinery
//!
//! The requirement asks for three things: (a) record the disposition, (b) a
//! containment names what it stands in for, (c) the count is reportable. It
//! never asks for a gap. A report that names its own unstated count already
//! notices absence, in the same place a reader is looking — so there is no
//! boundary node to mint, no bulk-parking of two hundred historical events, and
//! nothing to keep in step. The existing `fix_without_recorded_cause` gap covers
//! the adjacent question (a fix joined to no cause) and is left alone.
//!
//! # ⚠️ It reports and does not judge
//!
//! A workaround is often the CORRECT call under a deadline. Nothing here ranks,
//! scores or flags one: it counts what authors said and lists what they said it
//! stands in for (`dec:report-dont-judge`). And it can only ever reflect what
//! the author CLAIMED — it cannot detect a patch reported as a correction.

use crate::foundation::core::DynoError;
use crate::graph::DesignGraph;
use crate::nodes::node;

/// One repair that contained a symptom, with the correction it stands in for.
#[derive(Debug, Clone, serde::Serialize)]
pub struct StandingPatch {
    pub change_event_id: String,
    pub name: String,
    /// `change_type` as recorded — a patch is not confined to the fix types,
    /// and reporting the real value beats implying one.
    pub change_type: String,
    /// What the author said the proper fix would be. Never empty: the type that
    /// carries it makes a containment without one unconstructable.
    pub stands_in_for: String,
}

/// What rests on a patch, and how much of the ledger has not said.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RepairReport {
    /// ChangeEvents whose `change_type` is a repair kind — the population this
    /// question ranges over.
    pub repairs: usize,
    /// …of which the author said the cause was corrected.
    pub corrected_cause: usize,
    /// …of which the author said a symptom was contained.
    pub contained_symptom: usize,
    /// ⭐ …and of which NOBODY SAID. Read this before the two above: a small
    /// `contained_symptom` beside a large `unstated` means the design does not
    /// know what rests on a patch, not that little does.
    pub unstated: usize,
    /// …and of THOSE, how many are history told after the fact
    /// (`rationale_basis` `recalled` or `unknown`) — the `why` skill's record
    /// of a designer explaining old fixes. Still unstated, and still counted
    /// in `unstated`: nobody said, and this does not pretend otherwise. Split
    /// out because the remedy differs. An unstated fix recorded at the time
    /// is a question for whoever made it; one recalled years later was never
    /// part of the interview's small set of questions, and on a project that
    /// has just been interviewed it would otherwise swamp the number the
    /// discipline is meant to move.
    pub unstated_recalled: usize,
    /// The standing patches themselves, each naming the fix it stands in for.
    pub standing: Vec<StandingPatch>,
    /// The one line a reader needs first, stating which empty an empty answer is.
    pub note: String,
}

/// Which `change_type` values are repairs at all.
///
/// The same two the `fix_without_recorded_cause` detector ranges over, and
/// sharing the definition matters: two lists of "what counts as a fix" would
/// drift, and the gap and this report would then disagree about the
/// denominator of the same question.
pub const REPAIR_CHANGE_TYPES: &[&str] = &["defect_fix", "test_failure_fix"];

/// Reads through `scan_live_nodes`, not `scan_nodes`: a repair on a node an
/// accepted Decision has withdrawn is a patch on something that no longer
/// exists, and counting it would inflate the debt with work nobody owes.
pub(crate) fn repair_report(g: &DesignGraph) -> Result<RepairReport, DynoError> {
    let mut repairs = 0usize;
    let mut corrected = 0usize;
    let mut contained = 0usize;
    let mut standing: Vec<StandingPatch> = Vec::new();
    let mut unstated_recalled = 0usize;

    for ev in g.scan_live_nodes(node::CHANGE_EVENT)? {
        let prop = |k: &str| {
            ev.properties
                .iter()
                .find(|(n, _)| n.as_str() == k)
                .and_then(|(_, v)| v.as_str())
                .map(str::to_string)
        };
        let change_type = prop("change_type").unwrap_or_default();
        if !REPAIR_CHANGE_TYPES.contains(&change_type.as_str()) {
            continue;
        }
        repairs += 1;
        match prop("repair").as_deref() {
            Some("corrected_cause") => corrected += 1,
            Some("contained_symptom") => {
                contained += 1;
                standing.push(StandingPatch {
                    change_event_id: ev.node_id.clone(),
                    name: prop("name").unwrap_or_default(),
                    change_type,
                    // A containment cannot be constructed without this through
                    // `Repair`, but a graph can be written by an older binary or
                    // by an import, so the read side states what it found rather
                    // than trusting the writer.
                    stands_in_for: prop("stands_in_for").unwrap_or_else(|| {
                        String::from(
                            "(not recorded — written before `stands_in_for` existed, or imported)",
                        )
                    }),
                });
            }
            _ => {
                if prop("rationale_basis")
                    .as_deref()
                    .and_then(crate::temporal::RationaleBasis::parse)
                    .is_some_and(crate::temporal::RationaleBasis::is_after_the_fact)
                {
                    unstated_recalled += 1;
                }
            }
        }
    }
    standing.sort_by(|a, b| a.change_event_id.cmp(&b.change_event_id));
    let unstated = repairs - corrected - contained;

    let note = if repairs == 0 {
        String::from(
            "NOTHING TO EXAMINE: this design records no repairs yet, so the empty answer says \
             nothing about whether anything rests on a patch.",
        )
    } else if unstated == repairs {
        format!(
            "NOBODY HAS SAID, for all {repairs} repair(s). This design has not yet recorded a \
             repair disposition, so 'nothing rests on a patch' is NOT what this report means — it \
             means the question has never been answered. Pass `repair` to add_change_event \
             (or snapshot_before_change).{}",
            recalled_clause(unstated_recalled),
        )
    } else if unstated > 0 {
        format!(
            "{contained} standing patch(es) out of {repairs} repair(s) — but {unstated} said \
             nothing, so this is a floor and not a total. The unstated ones are neither corrected \
             nor contained; they are unanswered.{}",
            recalled_clause(unstated_recalled),
        )
    } else {
        format!(
            "Every one of {repairs} repair(s) states its disposition: {corrected} corrected the \
             cause, {contained} contained a symptom. This total is complete."
        )
    };

    Ok(RepairReport {
        repairs,
        corrected_cause: corrected,
        contained_symptom: contained,
        unstated,
        unstated_recalled,
        standing,
        note,
    })
}

/// The sentence that says how much of `unstated` is recalled history — empty
/// when none is, so a design that never ran the `why` skill reads exactly as
/// it did before.
fn recalled_clause(unstated_recalled: usize) -> String {
    if unstated_recalled == 0 {
        String::new()
    } else {
        format!(
            " Of the unstated, {unstated_recalled} are fixes recalled after the fact (the reason \
             was given later from memory, or is unknown), not fixes recorded when they were made."
        )
    }
}

impl DesignGraph {
    /// What still rests on a patch — see the module docs.
    pub fn repair_report(&self) -> Result<RepairReport, DynoError> {
        repair_report(self)
    }
}

/// THE STORED WORD for "this fix answered no finding": `answered_none: true`
/// on the ChangeEvent, written by `add_change_event` with `answers: []`.
///
/// `dec:a-fix-names-the-findings-it-answers-in-the-same-call` (Anthony,
/// 2026-10-25): a defect_fix or test_failure_fix that answers nothing is one
/// loop line "until it names what it answered or says it answered none". An
/// INVALIDATES edge from the change is the first; this property is the
/// second. Absent means nobody said, and is never read as either.
pub const ANSWERED_NONE: &str = "answered_none";

/// The repairs no finding is named as answered by: a live ChangeEvent of a
/// repair type ([`REPAIR_CHANGE_TYPES`]) with no outgoing INVALIDATES and no
/// [`ANSWERED_NONE`]. An event minted by a checksum ACCEPTANCE is left out,
/// as `fix_without_recorded_cause` leaves it out: it wears a fix label and is
/// not a repair (a CHANGED edge marked `accepted_baseline`).
///
/// Why it is read here, at loop_status, and not as a gap: the cause was
/// measured as "a separate optional step mostly does not get done"
/// (`fact:root-cause-a-fix-rarely-records-what-it-answered-because-it-is-a-separate-call-2026-10-25`),
/// so the cure is the step joining the call (`answers`) and one line where
/// every session already looks, not a question per event.
pub(crate) fn fixes_answering_nothing(g: &DesignGraph) -> Result<Vec<String>, DynoError> {
    use crate::foundation::core::Value;
    use crate::nodes::edge;
    let mut out = Vec::new();
    for ev in g.scan_live_nodes(node::CHANGE_EVENT)? {
        let change_type = ev
            .properties
            .get("change_type")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !REPAIR_CHANGE_TYPES.contains(&change_type) {
            continue;
        }
        if ev
            .properties
            .get(ANSWERED_NONE)
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            continue;
        }
        let out_edges = g.outgoing(&ev.node_id, None)?;
        let accepted = out_edges.iter().any(|e| {
            e.edge_type == edge::CHANGED
                && e.properties
                    .get("accepted_baseline")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
        });
        let answers = out_edges.iter().any(|e| e.edge_type == edge::INVALIDATES);
        if !accepted && !answers {
            out.push(ev.node_id);
        }
    }
    out.sort();
    Ok(out)
}

/// The two counts loop_status carries from [`repair_report`] in one line
/// (`dec:loop-status-carries-one-repair-line`, Anthony 2026-10-25): the
/// standing patches, and the repairs nobody labelled. The detail stays in
/// repair_report, which the usage baseline found never called through
/// flo2.io in a month — so its two numbers ride the call every session makes.
#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RepairLine {
    /// Repairs recorded as `contained_symptom`: each a patch still standing in
    /// for a proper fix.
    pub standing_patches: usize,
    /// Repairs that say neither `corrected_cause` nor `contained_symptom`.
    pub unlabelled: usize,
}

impl RepairLine {
    pub(crate) fn of(r: &RepairReport) -> Self {
        Self {
            standing_patches: r.contained_symptom,
            unlabelled: r.unstated,
        }
    }

    /// The line, or nothing on a design that records no repair at all.
    pub fn line(&self) -> Option<String> {
        if self.standing_patches == 0 && self.unlabelled == 0 {
            return None;
        }
        Some(format!(
            "{} standing patch(es), {} repair(s) nobody labelled — repair_report lists each \
             patch with the proper fix it stands in for; label a repair with add_change_event's \
             `repair` (corrected_cause or contained_symptom). Not counted against `clean`: it \
             reports what authors said and never judges it.",
            self.standing_patches, self.unlabelled
        ))
    }
}
