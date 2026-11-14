//! A PASS OVER MANY ITEMS READS WHAT THEY ALL SHARE ONCE, NOT ONCE PER ITEM
//! (`con:the-whole-loop-check-after-a-write-fits-the-default-time-budget-on-the-hosted-box`).
//!
//! MEASURED 2026-11-13 on reflow2's own design at 8,916 nodes, main at
//! ef67e0828: one loop check straight after a write took 8.7 s, and two parts
//! of it were a read repeated for every item of a pass.
//!
//! - Asking whether each check ever came back with a real result read all 833
//!   file records for each of 200 checks, to find the one record at the
//!   check's location: 1.8 s
//!   (`fact:root-cause-a-fifth-of-the-loop-check-after-a-write-is-200-whole-reads-of-the-file-records-one-per-check-2026-11-13`).
//! - The overtaken-defect rule read every change near every defect, and
//!   counted everything each had changed, 7,751 times for 550 defects, to rank
//!   the few it then offered: 1.2 s
//!   (`fact:root-cause-a-sixth-of-the-loop-check-after-a-write-is-the-overtaken-defect-rule-counting-what-each-change-changed-again-per-defect-2026-11-13`).
//!
//! THIS ASSERTS STRUCTURE, NOT DURATION, as
//! `a_repeat_loop_check_answers_from_what_it_kept.rs` does and for its reason:
//! how many times the shared thing was READ, which no load on the machine
//! moves. The class the two findings name is that nothing counted what a pass
//! reads, so [`DesignGraph::store_reads`] counts it and these tests hold two
//! passes to it.
//!
//! And the other half: what is kept is NEVER SERVED STALE. A write drops it,
//! and the next read sees the record the write added or moved.

use reflow2_core::DesignGraph;
use reflow2_core::detect::GapSource;
use reflow2_core::kept_record::ARTIFACTS_BY_LOCATION;
use reflow2_core::nodes::{Props, edge, node};

/// A graph that counts its reads of the store.
fn graph() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory_as("d").unwrap();
    g.count_store_reads();
    g
}

/// A check set passing whose `location` is `location`, exactly.
fn check_at(g: &mut DesignGraph, id: &str, location: &str) {
    g.add_verification(id, id, Some("analysis"), None, None)
        .unwrap();
    g.set_verification_status(id, "passing", Some("2026-11-13"), Some("exact"))
        .unwrap();
    g.upsert_node("Verification", id, Props::new().set("location", location))
        .unwrap();
}

/// A file record with a checksum whose own `location` is `location`.
fn record_at(g: &mut DesignGraph, id: &str, location: &str) {
    g.add_artifact(id, id, Some("data"), Some(location))
        .unwrap();
    g.upsert_node("Artifact", id, Props::new().set("checksum", "sha256:b2fa"))
        .unwrap();
}

/// The ids a check's records come back with, and how each was named.
fn records(g: &DesignGraph, check: &str) -> Vec<(String, Vec<&'static str>)> {
    g.records_of_check(check)
        .unwrap()
        .into_iter()
        .map(|r| (r.id, r.named_by))
        .collect()
}

/// THE CASE. Twelve checks, each naming its record by path: the file records
/// are read into the index ONCE, every check finds its own record, and a
/// whole loop check asked on top builds nothing more.
#[test]
fn many_checks_named_by_path_read_the_file_records_once() {
    let mut g = graph();
    for i in 0..12 {
        record_at(&mut g, &format!("art:r{i}"), &format!("r{i}.calc.json"));
        check_at(&mut g, &format!("ver:c{i}"), &format!("r{i}.calc.json"));
    }
    assert_eq!(g.kept_builds(ARTIFACTS_BY_LOCATION), 0, "not until asked");

    let scans_before = g.store_reads().scans_of("Artifact");
    for i in 0..12 {
        assert_eq!(
            records(&g, &format!("ver:c{i}")),
            [(format!("art:r{i}"), vec!["location"])]
        );
        assert!(
            g.computed_with_a_kept_record(
                &format!("ver:c{i}"),
                &g.get_node("Verification", &format!("ver:c{i}"))
                    .unwrap()
                    .unwrap()
                    .properties
            )
            .unwrap()
        );
    }
    assert_eq!(g.kept_builds(ARTIFACTS_BY_LOCATION), 1);
    assert_eq!(
        g.store_reads().scans_of("Artifact") - scans_before,
        1,
        "the file records are read once for twelve checks, not twelve times"
    );

    // The loop check asks the same of every check, twice over (the check
    // list and loop closure): still the one index.
    let status = g.loop_status().unwrap();
    assert!(status.not_computed.is_empty(), "{:?}", status.not_computed);
    assert_eq!(g.kept_builds(ARTIFACTS_BY_LOCATION), 1);
}

/// What is kept is never served stale: a write drops the index, and the next
/// read sees a record the write added, and no longer sees one it moved away.
#[test]
fn a_write_drops_the_index_and_the_next_read_is_of_the_design_as_it_now_is() {
    let mut g = graph();
    check_at(&mut g, "ver:late", "late.calc.json");
    check_at(&mut g, "ver:first", "first.calc.json");
    record_at(&mut g, "art:first", "first.calc.json");
    assert_eq!(records(&g, "ver:late"), []);
    assert_eq!(
        records(&g, "ver:first"),
        [("art:first".to_string(), vec!["location"])]
    );
    assert_eq!(g.kept_builds(ARTIFACTS_BY_LOCATION), 1);

    record_at(&mut g, "art:late", "late.calc.json");
    assert_eq!(
        records(&g, "ver:late"),
        [("art:late".to_string(), vec!["location"])],
        "the record added after the index was built is found"
    );
    assert_eq!(g.kept_builds(ARTIFACTS_BY_LOCATION), 2, "a write drops it");

    g.upsert_node(
        "Artifact",
        "art:first",
        Props::new().set("location", "moved.calc.json"),
    )
    .unwrap();
    assert_eq!(
        records(&g, "ver:first"),
        [],
        "a record moved to another location is no longer this check's"
    );
    assert_eq!(g.kept_builds(ARTIFACTS_BY_LOCATION), 3);
}

/// A check that names its record by id never pays for the index.
#[test]
fn a_check_that_names_its_record_by_id_never_reads_the_file_records() {
    let mut g = graph();
    record_at(&mut g, "art:x", "x.calc.json");
    check_at(&mut g, "ver:x", "x.calc.json (art:x)");
    let scans_before = g.store_reads().scans_of("Artifact");
    assert_eq!(
        records(&g, "ver:x"),
        [("art:x".to_string(), vec!["location"])]
    );
    assert_eq!(g.kept_builds(ARTIFACTS_BY_LOCATION), 0);
    assert_eq!(g.store_reads().scans_of("Artifact"), scans_before);
}

/// The matching is what it was: a record whose location is the check's whole
/// location, or its path (the first word), is the check's record, and one
/// that is both is named once.
#[test]
fn the_whole_location_and_its_path_each_match_and_no_record_is_named_twice() {
    let mut g = graph();
    record_at(&mut g, "art:a", "x.calc.json");
    record_at(&mut g, "art:b", "x.calc.json run 2");
    record_at(&mut g, "art:c", "y.calc.json");
    record_at(&mut g, "art:twin", "x.calc.json");
    check_at(&mut g, "ver:whole", "x.calc.json run 2");
    check_at(&mut g, "ver:path", "x.calc.json");
    check_at(&mut g, "ver:none", "z.calc.json");

    assert_eq!(
        records(&g, "ver:whole"),
        [
            ("art:a".to_string(), vec!["location"]),
            ("art:b".to_string(), vec!["location"]),
            ("art:twin".to_string(), vec!["location"]),
        ]
    );
    assert_eq!(
        records(&g, "ver:path"),
        [
            ("art:a".to_string(), vec!["location"]),
            ("art:twin".to_string(), vec!["location"]),
        ]
    );
    assert_eq!(records(&g, "ver:none"), []);
}

/// A defect recorded on `subject` on 2026-09-04.
fn defect(g: &mut DesignGraph, id: &str, subject: &str) {
    g.create_node(
        node::TEMPORAL_FACT,
        id,
        Props::new()
            .set("name", id)
            .set("fact_type", "defect")
            .set("statement", "it fires on the wrong thing")
            .set("subject_id", subject)
            .set("valid_from", "2026-09-04"),
    )
    .unwrap();
}

/// A change of `change_type` dated 2026-09-05 that CHANGED each of `targets`.
fn change(g: &mut DesignGraph, id: &str, change_type: &str, targets: &[(&str, &str)]) {
    g.create_node(
        node::CHANGE_EVENT,
        id,
        Props::new()
            .set("name", id)
            .set("change_type", change_type)
            .set("subject", "system")
            .set("detected_at", "2026-09-05"),
    )
    .unwrap();
    for (target_type, target) in targets {
        g.create_edge(
            edge::CHANGED,
            node::CHANGE_EVENT,
            id,
            target_type,
            target,
            Props::new(),
        )
        .unwrap();
    }
}

/// The overtaken-defect questions, in the order they are asked.
fn overtaken(g: &DesignGraph) -> Vec<String> {
    g.detect_gaps()
        .unwrap()
        .into_iter()
        .filter(|x| x.gap_source == GapSource::DefectOvertakenByChange)
        .map(|x| x.title)
        .collect()
}

/// THE CASE. A subject with one repair to offer and fifteen changes near it
/// that are never offered (ten that are not repairs, five on a file it shares
/// with another capability). How many records a change CHANGED is counted for
/// the one offered, per defect, and for none of the fifteen: three more
/// defects on the subject cost three more counts, not forty-eight.
#[test]
fn what_a_change_changed_is_counted_for_the_changes_offered_and_no_other() {
    let mut g = graph();
    g.add_capability("cap:s", "S", "Does the thing.", Some("realized"))
        .unwrap();
    g.add_capability("cap:other", "Other", "Does another.", Some("realized"))
        .unwrap();
    g.add_artifact("art:shared", "shared.rs", None, None)
        .unwrap();
    for cap in ["cap:s", "cap:other"] {
        g.create_edge(
            edge::REALIZES,
            node::ARTIFACT,
            "art:shared",
            node::CAPABILITY,
            cap,
            Props::new(),
        )
        .unwrap();
    }
    change(
        &mut g,
        "chg:the-repair",
        "defect_fix",
        &[(node::CAPABILITY, "cap:s")],
    );
    for i in 0..10 {
        change(
            &mut g,
            &format!("chg:feature-{i}"),
            "new_feature",
            &[(node::CAPABILITY, "cap:s")],
        );
    }
    for i in 0..5 {
        change(
            &mut g,
            &format!("chg:shared-{i}"),
            "defect_fix",
            &[(node::ARTIFACT, "art:shared")],
        );
    }

    defect(&mut g, "fact:d0", "cap:s");
    let before = g.store_reads().outgoing_of(edge::CHANGED);
    assert_eq!(overtaken(&g).len(), 1);
    let one_defect = g.store_reads().outgoing_of(edge::CHANGED) - before;

    for i in 1..4 {
        defect(&mut g, &format!("fact:d{i}"), "cap:s");
    }
    let before = g.store_reads().outgoing_of(edge::CHANGED);
    let asked = overtaken(&g);
    assert_eq!(asked.len(), 4);
    assert!(
        asked.iter().all(|t| t.contains("chg:the-repair")),
        "each defect is asked about the one repair: {asked:?}"
    );
    let four_defects = g.store_reads().outgoing_of(edge::CHANGED) - before;
    assert_eq!(
        four_defects - one_defect,
        3,
        "three more defects count what the one offered repair changed three more times; \
         counting it for all sixteen changes near each would be forty-eight"
    );
}

/// The ranking is what it was: of two repairs offered on the same day, the
/// one that changed fewer records is asked about first, whatever their ids.
#[test]
fn of_two_repairs_on_one_day_the_narrower_is_still_asked_about_first() {
    let mut g = graph();
    g.add_capability("cap:s", "S", "Does the thing.", Some("realized"))
        .unwrap();
    for other in ["art:one", "art:two"] {
        g.add_artifact(other, other, None, None).unwrap();
    }
    change(
        &mut g,
        "chg:a-broad",
        "defect_fix",
        &[
            (node::CAPABILITY, "cap:s"),
            (node::ARTIFACT, "art:one"),
            (node::ARTIFACT, "art:two"),
        ],
    );
    change(
        &mut g,
        "chg:z-narrow",
        "defect_fix",
        &[(node::CAPABILITY, "cap:s")],
    );
    defect(&mut g, "fact:d", "cap:s");

    let asked = overtaken(&g);
    assert_eq!(asked.len(), 1);
    assert!(
        asked[0].contains("did “chg:z-narrow”, or one other change, fix it?"),
        "the narrower repair is named, though its id sorts last: {}",
        asked[0]
    );
}
