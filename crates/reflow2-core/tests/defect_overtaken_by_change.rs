//! A recorded defect whose subject moved after it was written, with nothing
//! saying whether that was the fix, gets asked about.
//!
//! The mirror of `fix_without_recorded_cause`. Written from
//! `fact:a-defect-fixed-but-never-closed-on-the-record-was-re-fixed-wrongly-a-
//! day-later`: a fix landed, drew no INVALIDATES, the fact went on reading as
//! open, and the next session trusted it and re-fixed the symptom a different
//! way. A stale open defect is a live instruction to do the wrong thing.
//!
//! Ordering needs two dates, and the detector says so rather than guessing: an
//! undated change that is only NEAR the defect is named in the evidence and
//! never treated as later. A change JOINED to the defect needs no date: the
//! join places it after the defect it answers.
//!
//! WHICH CHANGE IT NAMES is the second half of this file, written from the
//! 2026-10-03 sweep of reflow2's own design
//! (`fact:the-overtaken-defect-question-named-the-real-fix-once-in-77-2026-10-03`):
//! of 77 questions it asked, the repair it named was the fix ONCE. Of the 20
//! defects that really were fixed it could see the fixing change for one: ten
//! fixes carried no date, seven were dated the day the defect was recorded, two
//! were recorded as `new_feature`, and seven were already joined to their defect
//! by CAUSES, MITIGATES or by naming it among what they changed — joins it never
//! read. A broad repair on a shared file re-asked 24 questions at once.

use reflow2_core::detect::{GapCandidate, GapSource};
use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, edge, node};

fn graph() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_artifact("art:detect", "detect.rs", None, None)
        .expect("artifact");
    g
}

fn defect(g: &mut DesignGraph, id: &str, subject: &str, valid_from: Option<&str>) {
    let mut props = Props::new()
        .set("name", id)
        .set("fact_type", "defect")
        .set("statement", "it fires on the wrong thing")
        .set("subject_id", subject);
    if let Some(d) = valid_from {
        props = props.set("valid_from", d);
    }
    g.create_node(node::TEMPORAL_FACT, id, props).expect("fact");
}

fn change_on(g: &mut DesignGraph, id: &str, target_type: &str, target: &str, at: Option<&str>) {
    change_of_kind(g, id, "defect_fix", target_type, target, at)
}

fn change_of_kind(
    g: &mut DesignGraph,
    id: &str,
    change_type: &str,
    target_type: &str,
    target: &str,
    at: Option<&str>,
) {
    let mut props = Props::new()
        .set("name", id)
        .set("change_type", change_type)
        .set("subject", "system");
    if let Some(d) = at {
        props = props.set("detected_at", d);
    }
    g.create_node(node::CHANGE_EVENT, id, props).expect("event");
    g.create_edge(
        edge::CHANGED,
        node::CHANGE_EVENT,
        id,
        target_type,
        target,
        Props::new(),
    )
    .expect("changed");
}

fn the_gaps(g: &DesignGraph) -> Vec<GapCandidate> {
    g.detect_gaps()
        .expect("detect")
        .into_iter()
        .filter(|x| x.gap_source == GapSource::DefectOvertakenByChange)
        .collect()
}

/// The case it was written from: the subject changed after the defect was
/// recorded, and nothing says whether that fixed it.
#[test]
fn a_later_dated_change_on_the_subject_asks_whether_it_was_the_fix() {
    let mut g = graph();
    defect(
        &mut g,
        "fact:defect-fires-wrong",
        "art:detect",
        Some("2026-09-04"),
    );
    change_on(
        &mut g,
        "chg:the-fix",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-05"),
    );

    let gaps = the_gaps(&g);
    assert_eq!(gaps.len(), 1);
    let gap = &gaps[0];
    assert_eq!(
        gap.affected_ids,
        vec!["chg:the-fix", "fact:defect-fires-wrong"],
        "anchored to the fact AND the change that may have fixed it"
    );
    assert!(
        gap.title.contains("did “chg:the-fix” fix it?"),
        "the question names the change it asks about: {}",
        gap.title
    );
    assert!(
        gap.description.contains("chg:the-fix"),
        "{}",
        gap.description
    );
}

/// INVALIDATES from the fix is the answer, and it clears the finding.
#[test]
fn a_fix_that_invalidates_the_fact_clears_it() {
    let mut g = graph();
    defect(
        &mut g,
        "fact:defect-fires-wrong",
        "art:detect",
        Some("2026-09-04"),
    );
    change_on(
        &mut g,
        "chg:the-fix",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-05"),
    );
    g.invalidates(
        node::CHANGE_EVENT,
        "chg:the-fix",
        node::TEMPORAL_FACT,
        "fact:defect-fires-wrong",
        Some("this was the fix"),
        Some("2026-09-05"),
    )
    .expect("invalidates");
    assert!(the_gaps(&g).is_empty());
}

/// A change BEFORE the defect was recorded cannot have fixed it. A repair on
/// the SAME day can, and usually did: a defect found and fixed in one session.
/// Seven of the twenty fixes the 2026-10-03 sweep confirmed were dated the day
/// their defect was recorded, and a strictly-later rule never offered one.
#[test]
fn an_earlier_change_is_not_offered_and_a_same_day_one_is() {
    let mut g = graph();
    defect(
        &mut g,
        "fact:defect-fires-wrong",
        "art:detect",
        Some("2026-09-04"),
    );
    change_on(
        &mut g,
        "chg:old",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-03"),
    );
    assert!(the_gaps(&g).is_empty(), "an earlier change is not offered");
    change_on(
        &mut g,
        "chg:same-day",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-04"),
    );
    let gaps = the_gaps(&g);
    assert_eq!(gaps.len(), 1, "a same-day repair is offered");
    assert_eq!(
        gaps[0].affected_ids,
        vec!["chg:same-day", "fact:defect-fires-wrong"]
    );
    assert!(
        gaps[0]
            .description
            .contains("the day the defect was recorded"),
        "{}",
        gaps[0].description
    );
}

/// An undated change that is only NEAR the defect cannot be ordered. It is
/// named in the evidence, never assumed later and never offered as the fix.
#[test]
fn an_undated_change_is_counted_not_assumed() {
    let mut g = graph();
    defect(
        &mut g,
        "fact:defect-fires-wrong",
        "art:detect",
        Some("2026-09-04"),
    );
    change_on(&mut g, "chg:undated", node::ARTIFACT, "art:detect", None);
    assert!(
        the_gaps(&g).is_empty(),
        "one undated change alone raises nothing"
    );

    change_on(
        &mut g,
        "chg:dated",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-06"),
    );
    let gaps = the_gaps(&g);
    assert_eq!(gaps.len(), 1);
    assert!(
        gaps[0]
            .evidence
            .contains("1 undated repair(s) on the subject could not be ordered against the fact"),
        "{}",
        gaps[0].evidence
    );
    assert!(
        gaps[0].evidence.contains("chg:undated"),
        "an undated repair is REPORTED by name, not only counted: {}",
        gaps[0].evidence
    );
    assert!(!gaps[0].affected_ids.iter().any(|id| id == "chg:undated"));
}

/// Only a REPAIR can be the fix. A feature or a refactor that touched the
/// subject later is counted in the evidence and never offered as the answer —
/// over every later change, one hub subject on reflow2's own design carried 39
/// members, which is a question nobody can answer.
#[test]
fn a_later_change_that_is_not_a_repair_is_counted_not_offered() {
    let mut g = graph();
    defect(
        &mut g,
        "fact:defect-fires-wrong",
        "art:detect",
        Some("2026-09-04"),
    );
    change_of_kind(
        &mut g,
        "chg:feature",
        "new_feature",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-05"),
    );
    change_of_kind(
        &mut g,
        "chg:tidy",
        "refactor",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-05"),
    );
    assert!(
        the_gaps(&g).is_empty(),
        "two later non-repairs raise nothing"
    );

    change_on(
        &mut g,
        "chg:the-fix",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-06"),
    );
    let gaps = the_gaps(&g);
    assert_eq!(gaps.len(), 1);
    assert_eq!(
        gaps[0].affected_ids,
        vec!["chg:the-fix", "fact:defect-fires-wrong"]
    );
    assert!(
        gaps[0]
            .evidence
            .contains("2 later change(s) on the same subject were not repairs"),
        "{}",
        gaps[0].evidence
    );
}

/// A capability's defect is fixed by changing something that realizes it.
#[test]
fn a_change_on_an_artifact_realizing_the_subject_counts() {
    let mut g = graph();
    g.add_capability("cap:detect", "Detect", "finds gaps", Some("realized"))
        .expect("capability");
    g.create_edge(
        edge::REALIZES,
        node::ARTIFACT,
        "art:detect",
        node::CAPABILITY,
        "cap:detect",
        Props::new(),
    )
    .expect("realizes");
    defect(
        &mut g,
        "fact:defect-fires-wrong",
        "cap:detect",
        Some("2026-09-04"),
    );
    change_on(
        &mut g,
        "chg:the-fix",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-05"),
    );

    let gaps = the_gaps(&g);
    assert_eq!(gaps.len(), 1);
    assert!(gaps[0].affected_ids.iter().any(|id| id == "chg:the-fix"));
}

/// Closed and undated facts are out of scope, and say nothing.
#[test]
fn a_closed_or_undated_fact_is_skipped() {
    let mut g = graph();
    defect(&mut g, "fact:defect-undated", "art:detect", None);
    g.create_node(
        node::TEMPORAL_FACT,
        "fact:defect-closed",
        Props::new()
            .set("name", "closed")
            .set("fact_type", "defect")
            .set("statement", "was wrong, then fixed")
            .set("subject_id", "art:detect")
            .set("valid_from", "2026-09-01")
            .set("valid_to", "2026-09-02"),
    )
    .expect("fact");
    change_on(
        &mut g,
        "chg:the-fix",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-05"),
    );
    assert!(the_gaps(&g).is_empty());
}

/// A further change on the same subject is a fresh question: the id moves.
#[test]
fn a_further_change_asks_again_under_a_new_id() {
    let mut g = graph();
    defect(
        &mut g,
        "fact:defect-fires-wrong",
        "art:detect",
        Some("2026-09-04"),
    );
    change_on(
        &mut g,
        "chg:first",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-05"),
    );
    let first = the_gaps(&g)[0].id.clone();
    change_on(
        &mut g,
        "chg:second",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-06"),
    );
    let second = the_gaps(&g)[0].id.clone();
    assert_ne!(first, second);
}

// ─── Which change it names ─────────────────────────────────────────────────
//
// Each test below failed on main 73cf37b, before the detector read joins.

fn edge(g: &mut DesignGraph, ty: &str, from: (&str, &str), to: (&str, &str)) {
    g.create_edge(ty, from.0, from.1, to.0, to.1, Props::new())
        .expect("edge");
}

/// A capability realized by `art:detect` alone, with an open defect on it.
fn capability_with_defect(g: &mut DesignGraph) {
    g.add_artifact("art:other", "elsewhere.rs", None, None)
        .expect("artifact");
    g.add_capability("cap:detect", "Detect", "finds gaps", Some("realized"))
        .expect("capability");
    edge(
        g,
        edge::REALIZES,
        (node::ARTIFACT, "art:detect"),
        (node::CAPABILITY, "cap:detect"),
    );
    defect(
        g,
        "fact:defect-fires-wrong",
        "cap:detect",
        Some("2026-09-04"),
    );
}

/// A change the DEFECT CAUSES — made because of it — is the strongest
/// candidate there is, whatever its type and whether or not it carries a
/// date: the join places it after the defect it answers. Three of the twenty
/// confirmed fixes were joined this way and never offered (one undated, one
/// same-day, one typed `new_feature`). It is named FIRST, ahead of a later
/// repair that merely touched the subject.
#[test]
fn a_change_the_defect_caused_is_named_first_whatever_its_type_or_date() {
    let mut g = graph();
    capability_with_defect(&mut g);
    change_on(
        &mut g,
        "chg:near",
        node::CAPABILITY,
        "cap:detect",
        Some("2026-09-05"),
    );
    change_of_kind(
        &mut g,
        "chg:the-fix",
        "new_feature",
        node::ARTIFACT,
        "art:other",
        None,
    );
    edge(
        &mut g,
        edge::CAUSES,
        (node::TEMPORAL_FACT, "fact:defect-fires-wrong"),
        (node::CHANGE_EVENT, "chg:the-fix"),
    );
    let gaps = the_gaps(&g);
    assert_eq!(gaps.len(), 1);
    assert!(
        gaps[0].title.contains("did “chg:the-fix”"),
        "the joined change is named first: {}",
        gaps[0].title
    );
    assert!(
        gaps[0].description.contains("the defect CAUSES it"),
        "{}",
        gaps[0].description
    );
    let at = |id: &str| gaps[0].description.find(id).expect(id);
    assert!(
        at("chg:the-fix") < at("chg:near"),
        "{}",
        gaps[0].description
    );
}

/// MITIGATES from the change, and a repair whose CHANGED set names the defect
/// itself, are joins too. Neither closes the defect — only INVALIDATES does —
/// and the question says so, naming the one call that would.
#[test]
fn a_change_that_mitigates_or_names_the_defect_is_offered_and_the_question_names_the_close() {
    let mut g = graph();
    capability_with_defect(&mut g);
    change_of_kind(
        &mut g,
        "chg:eases-it",
        "defect_fix",
        node::ARTIFACT,
        "art:other",
        None,
    );
    edge(
        &mut g,
        edge::MITIGATES,
        (node::CHANGE_EVENT, "chg:eases-it"),
        (node::TEMPORAL_FACT, "fact:defect-fires-wrong"),
    );
    change_of_kind(
        &mut g,
        "chg:names-it",
        "test_failure_fix",
        node::TEMPORAL_FACT,
        "fact:defect-fires-wrong",
        Some("2026-09-04"),
    );
    let gaps = the_gaps(&g);
    assert_eq!(gaps.len(), 1);
    assert_eq!(
        gaps[0].affected_ids,
        vec!["chg:eases-it", "chg:names-it", "fact:defect-fires-wrong"]
    );
    let d = &gaps[0].description;
    assert!(d.contains("it MITIGATES this defect"), "{d}");
    assert!(
        d.contains("a repair whose CHANGED set names this defect"),
        "{d}"
    );
    assert!(
        d.contains("only INVALIDATES") && d.contains("`invalidates`"),
        "the question names the one edge that closes it: {d}"
    );
}

/// A change that CAUSES the defect introduced it. It is never offered as the
/// defect's fix, however near it is.
#[test]
fn a_change_that_caused_the_defect_is_never_offered_as_its_fix() {
    let mut g = graph();
    capability_with_defect(&mut g);
    change_on(
        &mut g,
        "chg:regression",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-05"),
    );
    edge(
        &mut g,
        edge::CAUSES,
        (node::CHANGE_EVENT, "chg:regression"),
        (node::TEMPORAL_FACT, "fact:defect-fires-wrong"),
    );
    assert!(the_gaps(&g).is_empty(), "{:?}", the_gaps(&g));
}

/// A repair that already says which finding it fixed — it INVALIDATES or
/// MITIGATES another one, or another finding CAUSES it — is attributed. It is
/// not offered for a different defect merely because it touched the same
/// subject: on the sweep, 195 of the 363 near-but-wrong candidates were such
/// repairs, against one of the eight near fixes. It is named in the evidence.
#[test]
fn a_repair_recorded_as_fixing_another_finding_is_not_offered_by_nearness() {
    let mut g = graph();
    capability_with_defect(&mut g);
    defect(
        &mut g,
        "fact:another-defect",
        "art:other",
        Some("2026-09-01"),
    );
    change_on(
        &mut g,
        "chg:fixed-the-other",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-05"),
    );
    g.invalidates(
        node::CHANGE_EVENT,
        "chg:fixed-the-other",
        node::TEMPORAL_FACT,
        "fact:another-defect",
        Some("this was the fix"),
        Some("2026-09-05"),
    )
    .expect("invalidates");
    assert!(the_gaps(&g).is_empty(), "{:?}", the_gaps(&g));

    change_on(
        &mut g,
        "chg:unattributed",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-06"),
    );
    let gaps = the_gaps(&g);
    assert_eq!(gaps.len(), 1);
    assert_eq!(
        gaps[0].affected_ids,
        vec!["chg:unattributed", "fact:defect-fires-wrong"]
    );
    assert!(
        gaps[0]
            .evidence
            .contains("recorded as fixing another finding")
            && gaps[0].evidence.contains("chg:fixed-the-other"),
        "{}",
        gaps[0].evidence
    );
}

/// A file that realizes several capabilities says nothing about which of them
/// a change to it touched. A repair on a SHARED file counts only for a defect
/// whose subject it CHANGED itself; a file realizing the subject alone still
/// counts.
#[test]
fn a_repair_on_a_shared_file_counts_only_where_it_changed_the_subject() {
    let mut g = graph();
    capability_with_defect(&mut g);
    g.add_artifact("art:hub", "service.rs", None, None)
        .expect("artifact");
    g.add_capability("cap:other", "Other", "does other things", Some("realized"))
        .expect("capability");
    for cap in ["cap:detect", "cap:other"] {
        edge(
            &mut g,
            edge::REALIZES,
            (node::ARTIFACT, "art:hub"),
            (node::CAPABILITY, cap),
        );
    }
    change_on(
        &mut g,
        "chg:hub-only",
        node::ARTIFACT,
        "art:hub",
        Some("2026-09-05"),
    );
    let gaps = the_gaps(&g);
    assert!(
        gaps.is_empty(),
        "a shared file alone is not enough: {gaps:?}"
    );

    edge(
        &mut g,
        edge::CHANGED,
        (node::CHANGE_EVENT, "chg:hub-only"),
        (node::CAPABILITY, "cap:detect"),
    );
    assert_eq!(the_gaps(&g).len(), 1, "it CHANGED the subject itself");
}

/// A defect with no `valid_from` cannot be ordered against anything — but a
/// change JOINED to it needs no order, so it is still asked about.
#[test]
fn an_undated_defect_is_asked_about_a_change_joined_to_it() {
    let mut g = graph();
    capability_with_defect(&mut g);
    defect(&mut g, "fact:undated-defect", "cap:detect", None);
    change_of_kind(
        &mut g,
        "chg:the-fix",
        "defect_fix",
        node::ARTIFACT,
        "art:other",
        None,
    );
    edge(
        &mut g,
        edge::CAUSES,
        (node::TEMPORAL_FACT, "fact:undated-defect"),
        (node::CHANGE_EVENT, "chg:the-fix"),
    );
    let gaps = the_gaps(&g);
    assert!(
        gaps.iter()
            .any(|x| x.affected_ids == vec!["chg:the-fix", "fact:undated-defect"]),
        "{gaps:?}"
    );
}

/// INVALIDATES closes the defect even when a weaker join exists: the
/// question is never asked about a defect a change has closed.
#[test]
fn invalidates_closes_it_whatever_else_joins_it() {
    let mut g = graph();
    capability_with_defect(&mut g);
    change_on(
        &mut g,
        "chg:the-fix",
        node::ARTIFACT,
        "art:detect",
        Some("2026-09-04"),
    );
    edge(
        &mut g,
        edge::CAUSES,
        (node::TEMPORAL_FACT, "fact:defect-fires-wrong"),
        (node::CHANGE_EVENT, "chg:the-fix"),
    );
    assert_eq!(the_gaps(&g).len(), 1);
    g.invalidates(
        node::CHANGE_EVENT,
        "chg:the-fix",
        node::TEMPORAL_FACT,
        "fact:defect-fires-wrong",
        Some("this was the fix"),
        Some("2026-09-04"),
    )
    .expect("invalidates");
    assert!(the_gaps(&g).is_empty());
}

/// A judgement is about REPAIRS, so it is remembered per repair. An
/// acknowledgement records that each change it named did not fix the defect;
/// when the set of candidates changes for another reason — a repair drops out,
/// the rule is refined, another PR re-keys the question — a change already
/// judged is not asked about again. Only a change nobody has judged asks.
/// Before, the acknowledgement was keyed on the whole set and expired with it:
/// one PR re-keyed 23 answered questions in a single merge.
#[test]
fn a_repair_already_judged_for_the_defect_is_not_asked_about_again() {
    let mut g = graph();
    capability_with_defect(&mut g);
    change_on(
        &mut g,
        "chg:first",
        node::CAPABILITY,
        "cap:detect",
        Some("2026-09-05"),
    );
    change_on(
        &mut g,
        "chg:second",
        node::CAPABILITY,
        "cap:detect",
        Some("2026-09-06"),
    );
    let gap = the_gaps(&g).remove(0);
    g.acknowledge_gap(&gap.id, &gap.affected_ids, "neither was the fix")
        .expect("acknowledge");
    assert!(the_gaps(&g).is_empty());

    // `chg:first` stops touching the subject: the set is now {chg:second},
    // a new key — and every change in it has already been judged.
    assert!(
        g.delete_edge(edge::CHANGED, "chg:first", "cap:detect")
            .expect("delete")
    );
    assert!(
        the_gaps(&g).is_empty(),
        "a repair already judged is not asked about again: {:?}",
        the_gaps(&g)
    );

    // A NEW repair asks — and only it is new.
    change_on(
        &mut g,
        "chg:third",
        node::CAPABILITY,
        "cap:detect",
        Some("2026-09-07"),
    );
    let gaps = the_gaps(&g);
    assert_eq!(gaps.len(), 1);
    assert!(
        gaps[0].title.contains("did “chg:third” fix it?"),
        "the change nobody judged is the one asked about: {}",
        gaps[0].title
    );
    assert!(
        gaps[0].description.contains("ALREADY JUDGED"),
        "{}",
        gaps[0].description
    );
}
