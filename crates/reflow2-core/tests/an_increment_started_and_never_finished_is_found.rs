//! AN INCREMENT STARTED AND NEVER FINISHED IS FOUND, AND EVERY OPEN ONE IS
//! LISTED — the two requirements accepted 2026-11-14:
//!
//! 1. A planned increment (a planned epoch) is reported when any of four
//!    signals holds, each computed from edges and statuses alone: DONE BUT NOT
//!    CLOSED (everything scheduled for it is delivered and it still reads
//!    planned), OVERTAKEN (a later epoch in its sequence has arrived while it
//!    still reads planned), EMPTY (nothing is scheduled for it, so it can never
//!    be judged), STALLED (nothing scheduled for it has moved over the design's
//!    last N recorded changes).
//! 2. `loop_status`, `graph_report` and the where-am-i skill list every open
//!    increment oldest first, each with how much of what was scheduled for it
//!    is delivered and which unfinished signals it shows, until it is closed.
//!
//! SETTLED BY THE OWNER THE SAME DAY, and pinned here:
//! - OVERTAKEN reads the DRAWN order only: an epoch this one PRECEDES,
//!   directly or down a chain, has arrived. `sequence` is never read, and
//!   where no order was drawn nothing is said.
//! - STALLED IS NOT CALLED YET. Each open increment carries a COUNT instead:
//!   how many of the design's most recent recorded changes have passed since
//!   anything scheduled for it moved. An item moved when a change names it, or
//!   names a capability or a file that delivers it. No threshold; and where
//!   the count cannot be worked out it reads "never", not a number.
//! - No clock: the count is a number of changes, never an age.
//!
//! One computation (`DesignGraph::open_increments`) answers all of it, and
//! `loop_status` and `graph_report` read that one answer. How much is
//! delivered is `arrival_delta`'s own reading, not a second one.
//!
//! OBSERVED FAILING first: written before `open_increments` existed, on
//! origin/main 84eca9422 (v0.99.0), where none of it compiles.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use reflow2_core::foundation::store::{StampClock, format_stamp_time};
use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, node};
use reflow2_core::open_increments::{
    DONE_BUT_NOT_CLOSED, EMPTY, OPEN_INCREMENTS_LISTED, OVERTAKEN, OpenIncrement, SinceMoved,
};
use reflow2_core::temporal::{ChangeAction, ChangeType, EpochType, ScheduleOutcome};

/// A clock that moves one second per reading, so the order the store recorded
/// things in is the order this file wrote them in.
fn ticking() -> StampClock {
    let at = Arc::new(AtomicI64::new(1_793_145_600));
    Arc::new(move || format_stamp_time(at.fetch_add(1, Ordering::SeqCst), 0))
}

fn design() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.set_stamp_clock(ticking()).expect("clock");
    g.add_project("proj:1", "Thing").expect("project");
    g
}

fn plan(g: &mut DesignGraph, id: &str) {
    g.plan_epoch(id, &format!("Increment {id}"), EpochType::Milestone, 1)
        .expect("plan");
}

fn schedule(g: &mut DesignGraph, item_type: &str, item: &str, epoch: &str) {
    g.schedule_for(item_type, item, node::DESIGN_EPOCH, epoch, "expected", None)
        .expect("schedule");
}

/// A requirement scheduled for `epoch`, not delivered.
fn need(g: &mut DesignGraph, req: &str, epoch: &str) {
    g.add_requirement(req, req, "it must hold")
        .expect("requirement");
    schedule(g, node::REQUIREMENT, req, epoch);
}

/// Deliver `req` the way the delivery reading asks: a capability satisfies it,
/// a file realizes that capability, and a passing check verifies it.
fn deliver(g: &mut DesignGraph, req: &str, suffix: &str) {
    let (cap, art, ver) = (
        format!("cap:{suffix}"),
        format!("art:{suffix}"),
        format!("ver:{suffix}"),
    );
    g.add_capability(&cap, suffix, "does it", Some("realized"))
        .expect("capability");
    g.satisfies(&cap, req).expect("satisfies");
    g.add_artifact(
        &art,
        suffix,
        Some("code"),
        Some(&format!("src/{suffix}.rs")),
    )
    .expect("artifact");
    g.realizes(&art, node::CAPABILITY, &cap, None, None)
        .expect("realizes");
    g.add_verification(&ver, suffix, Some("test"), None, None)
        .expect("verification");
    g.verifies(&ver, node::CAPABILITY, &cap).expect("verifies");
    g.set_verification_status(&ver, "passing", None, None)
        .expect("passing");
}

/// A recorded change naming `affected` among what it changed.
fn change(g: &mut DesignGraph, id: &str, affected: &[(&str, &str)]) {
    g.add_change_event(id, id, ChangeType::NewFeature, None, None, None, None)
        .expect("change");
    for (t, target) in affected {
        g.changed(id, t, target, ChangeAction::Modified)
            .expect("changed");
    }
}

fn row(g: &DesignGraph, epoch: &str) -> OpenIncrement {
    g.open_increments()
        .expect("open_increments")
        .increments
        .into_iter()
        .find(|i| i.epoch_id == epoch)
        .unwrap_or_else(|| panic!("{epoch} is listed as open"))
}

// ─── Signal 1: done but not closed ─────────────────────────────────────────

#[test]
fn everything_delivered_and_still_planned_reads_done_but_not_closed() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    need(&mut g, "req:one", "epoch:a");
    need(&mut g, "req:two", "epoch:a");
    deliver(&mut g, "req:one", "one");

    let half = row(&g, "epoch:a");
    assert_eq!((half.scheduled, half.delivered), (2, 1));
    assert!(
        !half.signals.contains(&DONE_BUT_NOT_CLOSED),
        "one of two is still outstanding: {:?}",
        half.signals
    );

    deliver(&mut g, "req:two", "two");
    let done = row(&g, "epoch:a");
    assert_eq!((done.scheduled, done.delivered), (2, 2));
    assert_eq!(done.signals, vec![DONE_BUT_NOT_CLOSED]);
    let all = g.open_increments().unwrap();
    assert_eq!((all.open, all.done_but_not_closed), (1, 1));
}

/// How much is delivered is `arrival_delta`'s reading of the same schedule,
/// item for item — never a second derivation that could drift from it.
#[test]
fn how_much_is_delivered_is_what_arrival_delta_reads() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    need(&mut g, "req:one", "epoch:a");
    need(&mut g, "req:two", "epoch:a");
    need(&mut g, "req:three", "epoch:a");
    deliver(&mut g, "req:one", "one");
    deliver(&mut g, "req:three", "three");

    let delta = g.arrival_delta("epoch:a").expect("delta");
    let delivered = delta
        .items
        .iter()
        .filter(|i| i.outcome == ScheduleOutcome::Delivered)
        .count();
    let r = row(&g, "epoch:a");
    assert_eq!(r.scheduled, delta.items.len());
    assert_eq!(r.delivered, delivered);
    assert_eq!((r.scheduled, r.delivered), (3, 2));
}

/// A question somebody WITHDREW is settled: `arrival_delta` reads it
/// discontinued, never outstanding, so it does not hold an increment open
/// whose every other item is delivered. It is counted apart from delivered.
#[test]
fn a_withdrawn_question_does_not_keep_a_finished_increment_from_reading_done() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    need(&mut g, "req:one", "epoch:a");
    deliver(&mut g, "req:one", "one");
    g.create_node(
        node::QUESTION,
        "question:abc",
        Props::new()
            .set("question", "Where does packaging belong?")
            .set("gap_id", "gap:abc")
            .set("status", "asked"),
    )
    .expect("question");
    schedule(&mut g, node::QUESTION, "question:abc", "epoch:a");
    assert!(
        row(&g, "epoch:a").signals.is_empty(),
        "an asked question is outstanding"
    );

    g.withdraw_question("gap:abc").expect("withdraw");
    let r = row(&g, "epoch:a");
    assert_eq!((r.scheduled, r.delivered, r.withdrawn), (2, 1, 1));
    assert_eq!(r.signals, vec![DONE_BUT_NOT_CLOSED]);
}

/// An increment whose every item was withdrawn has nothing still intended:
/// it reads empty, never done, and the row keeps both numbers.
#[test]
fn an_increment_holding_only_withdrawn_questions_reads_empty() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    g.create_node(
        node::QUESTION,
        "question:abc",
        Props::new()
            .set("question", "Where does packaging belong?")
            .set("gap_id", "gap:abc")
            .set("status", "asked"),
    )
    .expect("question");
    schedule(&mut g, node::QUESTION, "question:abc", "epoch:a");
    g.withdraw_question("gap:abc").expect("withdraw");
    let r = row(&g, "epoch:a");
    assert_eq!((r.scheduled, r.delivered, r.withdrawn), (1, 0, 1));
    assert_eq!(r.signals, vec![EMPTY]);
}

// ─── Signal 3: empty ───────────────────────────────────────────────────────

#[test]
fn nothing_scheduled_reads_empty_and_is_never_done() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    let r = row(&g, "epoch:a");
    assert_eq!((r.scheduled, r.delivered), (0, 0));
    assert_eq!(
        r.signals,
        vec![EMPTY],
        "nothing scheduled is empty, and never 'everything delivered'"
    );
    let all = g.open_increments().unwrap();
    assert_eq!((all.open, all.empty, all.done_but_not_closed), (1, 1, 0));
}

/// An increment that only ORDERS other increments is still reported empty —
/// never suppressed — and says how many epochs it precedes and is preceded
/// by, so a reader can tell an umbrella from a forgotten one.
#[test]
fn an_empty_increment_that_only_orders_others_is_still_reported_with_what_it_orders() {
    let mut g = design();
    for e in ["epoch:before", "epoch:umbrella", "epoch:x", "epoch:y"] {
        plan(&mut g, e);
    }
    g.precedes("epoch:before", "epoch:umbrella").unwrap();
    g.precedes("epoch:umbrella", "epoch:x").unwrap();
    g.precedes("epoch:umbrella", "epoch:y").unwrap();
    let r = row(&g, "epoch:umbrella");
    assert!(r.signals.contains(&EMPTY));
    assert_eq!((r.precedes, r.preceded_by), (2, 1));
}

// ─── Signal 2: overtaken ───────────────────────────────────────────────────

#[test]
fn an_arrived_epoch_down_a_chain_of_precedes_reads_overtaken() {
    let mut g = design();
    for e in ["epoch:a", "epoch:b", "epoch:c"] {
        plan(&mut g, e);
        need(&mut g, &format!("req:{}", &e[6..]), e);
    }
    g.precedes("epoch:a", "epoch:b").unwrap();
    g.precedes("epoch:b", "epoch:c").unwrap();
    assert!(row(&g, "epoch:a").signals.is_empty(), "nothing has arrived");

    g.set_epoch_status("epoch:c", "arrived").unwrap();
    let a = row(&g, "epoch:a");
    assert_eq!(a.signals, vec![OVERTAKEN], "through epoch:b");
    assert_eq!(a.overtaken_by.as_deref(), Some("epoch:c"));
    let b = row(&g, "epoch:b");
    assert_eq!(b.signals, vec![OVERTAKEN], "directly");
    let all = g.open_increments().unwrap();
    assert_eq!((all.open, all.overtaken), (2, 2));
}

/// THE OWNER'S RULING: the drawn order only. A later epoch BY `sequence` that
/// has arrived says nothing about this one when no PRECEDES was drawn.
#[test]
fn a_later_arrived_epoch_by_sequence_alone_is_not_overtaken() {
    let mut g = design();
    g.plan_epoch("epoch:early", "Early", EpochType::Milestone, 1)
        .unwrap();
    need(&mut g, "req:early", "epoch:early");
    g.plan_epoch("epoch:late", "Late", EpochType::Milestone, 99)
        .unwrap();
    g.set_epoch_status("epoch:late", "arrived").unwrap();
    // And one that arrived without ever being planned, later still.
    g.add_epoch("epoch:later", "Later", EpochType::Revision, 100)
        .unwrap();

    let r = row(&g, "epoch:early");
    assert!(
        !r.signals.contains(&OVERTAKEN),
        "no order was drawn, so nothing is said: {:?}",
        r.signals
    );
    assert_eq!(r.overtaken_by, None);
    assert_eq!(g.open_increments().unwrap().overtaken, 0);
}

/// The order runs one way: an arrived epoch that precedes this one is the
/// ordinary case (the earlier increment finished), never "overtaken".
#[test]
fn an_arrived_epoch_that_comes_before_it_does_not_overtake_it() {
    let mut g = design();
    plan(&mut g, "epoch:first");
    plan(&mut g, "epoch:second");
    need(&mut g, "req:second", "epoch:second");
    g.precedes("epoch:first", "epoch:second").unwrap();
    g.set_epoch_status("epoch:first", "arrived").unwrap();
    assert!(row(&g, "epoch:second").signals.is_empty());
}

#[test]
fn a_cycle_of_precedes_terminates() {
    let mut g = design();
    for e in ["epoch:a", "epoch:b", "epoch:c", "epoch:d"] {
        plan(&mut g, e);
        need(&mut g, &format!("req:{}", &e[6..]), e);
    }
    g.precedes("epoch:a", "epoch:b").unwrap();
    g.precedes("epoch:b", "epoch:c").unwrap();
    g.precedes("epoch:c", "epoch:a").unwrap();
    // Nothing in the cycle has arrived: every walk ends, and says nothing.
    let all = g.open_increments().expect("the walk ends");
    assert_eq!((all.open, all.overtaken), (4, 0));

    // An arrived epoch hanging off the cycle is reached from every member.
    g.precedes("epoch:c", "epoch:d").unwrap();
    g.set_epoch_status("epoch:d", "arrived").unwrap();
    let all = g.open_increments().expect("the walk ends");
    assert_eq!((all.open, all.overtaken), (3, 3));
}

// ─── The count that stands where "stalled" would ───────────────────────────

#[test]
fn the_count_is_the_changes_recorded_since_one_named_a_scheduled_item() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    need(&mut g, "req:one", "epoch:a");
    change(&mut g, "chg:1", &[(node::REQUIREMENT, "req:one")]);
    assert_eq!(
        row(&g, "epoch:a").changes_since_it_moved,
        SinceMoved::Changes(0),
        "the most recent change named it"
    );

    g.add_requirement("req:elsewhere", "Elsewhere", "another matter")
        .unwrap();
    change(&mut g, "chg:2", &[(node::REQUIREMENT, "req:elsewhere")]);
    change(&mut g, "chg:3", &[]);
    change(&mut g, "chg:4", &[(node::REQUIREMENT, "req:elsewhere")]);
    let r = row(&g, "epoch:a");
    assert_eq!(r.changes_since_it_moved, SinceMoved::Changes(3));
    assert!(!r.or_more);
    assert_eq!(r.why_never, None);

    // Naming it again starts the count over.
    change(&mut g, "chg:5", &[(node::REQUIREMENT, "req:one")]);
    assert_eq!(
        row(&g, "epoch:a").changes_since_it_moved,
        SinceMoved::Changes(0)
    );
}

#[test]
fn a_change_naming_a_capability_that_delivers_the_item_moves_it() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    need(&mut g, "req:one", "epoch:a");
    g.add_capability("cap:one", "One", "does it", None).unwrap();
    g.satisfies("cap:one", "req:one").unwrap();
    change(&mut g, "chg:1", &[(node::CAPABILITY, "cap:one")]);
    change(&mut g, "chg:2", &[]);
    assert_eq!(
        row(&g, "epoch:a").changes_since_it_moved,
        SinceMoved::Changes(1)
    );
}

#[test]
fn a_change_naming_a_file_that_delivers_the_item_moves_it() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    // A requirement, delivered through a capability's file …
    need(&mut g, "req:one", "epoch:a");
    g.add_capability("cap:one", "One", "does it", None).unwrap();
    g.satisfies("cap:one", "req:one").unwrap();
    g.add_artifact("art:one", "One", Some("code"), Some("src/one.rs"))
        .unwrap();
    g.realizes("art:one", node::CAPABILITY, "cap:one", None, None)
        .unwrap();
    // … and a capability scheduled itself, with its own file.
    plan(&mut g, "epoch:b");
    g.add_capability("cap:two", "Two", "does it", None).unwrap();
    schedule(&mut g, node::CAPABILITY, "cap:two", "epoch:b");
    g.add_artifact("art:two", "Two", Some("code"), Some("src/two.rs"))
        .unwrap();
    g.realizes("art:two", node::CAPABILITY, "cap:two", None, None)
        .unwrap();

    change(&mut g, "chg:1", &[(node::ARTIFACT, "art:one")]);
    change(&mut g, "chg:2", &[(node::ARTIFACT, "art:two")]);
    change(&mut g, "chg:3", &[]);
    assert_eq!(
        row(&g, "epoch:a").changes_since_it_moved,
        SinceMoved::Changes(2)
    );
    assert_eq!(
        row(&g, "epoch:b").changes_since_it_moved,
        SinceMoved::Changes(1)
    );
}

/// Where the count cannot be worked out it reads "never", with the reason —
/// never a number that would look like a measurement.
#[test]
fn an_increment_nothing_ever_moved_reads_never_and_says_why() {
    let mut g = design();
    plan(&mut g, "epoch:empty");
    plan(&mut g, "epoch:untouched");
    need(&mut g, "req:one", "epoch:untouched");
    g.add_requirement("req:elsewhere", "Elsewhere", "another matter")
        .unwrap();
    change(&mut g, "chg:1", &[(node::REQUIREMENT, "req:elsewhere")]);

    let empty = row(&g, "epoch:empty");
    assert_eq!(empty.changes_since_it_moved, SinceMoved::Never);
    assert!(
        empty
            .why_never
            .is_some_and(|w| w.contains("nothing is scheduled")),
        "{:?}",
        empty.why_never
    );
    let untouched = row(&g, "epoch:untouched");
    assert_eq!(untouched.changes_since_it_moved, SinceMoved::Never);
    assert!(
        untouched
            .why_never
            .is_some_and(|w| w.contains("no recorded change names")),
        "{:?}",
        untouched.why_never
    );
    assert_eq!(g.open_increments().unwrap().never_moved, 2);

    // On the wire it is the word, not a number.
    let wire = serde_json::to_value(&untouched).unwrap();
    assert_eq!(wire["changes_since_it_moved"], "never");
    let moved = {
        change(&mut g, "chg:2", &[(node::REQUIREMENT, "req:one")]);
        serde_json::to_value(row(&g, "epoch:untouched")).unwrap()
    };
    assert_eq!(moved["changes_since_it_moved"], 0);
    assert!(moved.get("why_never").is_none());
}

/// A change the store recorded no time for (written before it kept one) is
/// older than every change it did record, and where it stands among its own
/// kind is not known: a count resting on one is a FLOOR and says so.
#[test]
fn a_count_resting_on_a_change_with_no_recorded_time_is_a_floor_and_says_so() {
    let mut old = design();
    plan(&mut old, "epoch:a");
    need(&mut old, "req:one", "epoch:a");
    old.add_requirement("req:elsewhere", "Elsewhere", "another matter")
        .unwrap();
    change(&mut old, "chg:old-1", &[(node::REQUIREMENT, "req:one")]);
    change(
        &mut old,
        "chg:old-2",
        &[(node::REQUIREMENT, "req:elsewhere")],
    );
    // The same design as a store that never recorded when anything was written.
    let mut doc = old.export_graph_with_stamps().expect("export");
    for n in &mut doc.nodes {
        n.stamps.clear();
    }
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.set_stamp_clock(ticking()).expect("clock");
    g.import_graph(&doc).expect("import");
    change(&mut g, "chg:new-1", &[(node::REQUIREMENT, "req:elsewhere")]);
    change(&mut g, "chg:new-2", &[]);

    let all = g.open_increments().unwrap();
    assert_eq!(
        (all.changes_recorded, all.changes_with_no_recorded_time),
        (4, 2)
    );
    let r = row(&g, "epoch:a");
    assert_eq!(
        r.changes_since_it_moved,
        SinceMoved::Changes(2),
        "both changes the store has a time for are newer"
    );
    assert!(
        r.or_more,
        "chg:old-2 may be newer too; the store cannot say"
    );
}

/// Nothing is CALLED stalled: the only signals are the three, whatever the
/// count reads.
#[test]
fn nothing_is_called_stalled() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    need(&mut g, "req:one", "epoch:a");
    change(&mut g, "chg:0", &[(node::REQUIREMENT, "req:one")]);
    for i in 1..=60 {
        change(&mut g, &format!("chg:{i}"), &[]);
    }
    let r = row(&g, "epoch:a");
    assert_eq!(r.changes_since_it_moved, SinceMoved::Changes(60));
    assert!(r.signals.is_empty(), "{:?}", r.signals);
    let wire = serde_json::to_value(g.open_increments().unwrap()).unwrap();
    let keys: Vec<&String> = wire.as_object().unwrap().keys().collect();
    assert!(
        !keys.iter().any(|k| k.contains("stalled")),
        "no count of stalled increments: {keys:?}"
    );
}

// ─── The list: every open increment, oldest first, until it is closed ──────

#[test]
fn an_arrived_epoch_is_not_listed_and_an_open_one_stays_until_it_arrives() {
    let mut g = design();
    plan(&mut g, "epoch:a");
    plan(&mut g, "epoch:b");
    g.add_epoch(
        "epoch:past",
        "A point that happened",
        EpochType::Revision,
        0,
    )
    .unwrap();
    let ids = |g: &DesignGraph| -> Vec<String> {
        g.open_increments()
            .unwrap()
            .increments
            .into_iter()
            .map(|i| i.epoch_id)
            .collect()
    };
    assert_eq!(ids(&g), ["epoch:a", "epoch:b"]);

    // It keeps showing through any number of reads and unrelated writes …
    change(&mut g, "chg:1", &[]);
    assert_eq!(ids(&g), ["epoch:a", "epoch:b"]);
    // … and leaves the list when it is closed by arrival.
    g.set_epoch_status("epoch:a", "arrived").unwrap();
    assert_eq!(ids(&g), ["epoch:b"]);
    assert_eq!(g.open_increments().unwrap().open, 1);
}

#[test]
fn the_list_is_oldest_first_by_when_the_store_recorded_each_one() {
    let mut g = design();
    // Planned in an order no id and no `sequence` would give.
    g.plan_epoch("epoch:m", "Planned first", EpochType::Milestone, 30)
        .unwrap();
    g.plan_epoch("epoch:z", "Planned second", EpochType::Milestone, 10)
        .unwrap();
    g.plan_epoch("epoch:a", "Planned third", EpochType::Milestone, 20)
        .unwrap();
    // Revising the oldest does not make it new.
    g.plan_epoch(
        "epoch:m",
        "Planned first, reworded",
        EpochType::Milestone,
        30,
    )
    .unwrap();
    let all = g.open_increments().unwrap();
    let ids: Vec<&str> = all.increments.iter().map(|i| i.epoch_id.as_str()).collect();
    assert_eq!(ids, ["epoch:m", "epoch:z", "epoch:a"]);
    assert_eq!(all.increments_with_no_recorded_time, 0);
}

/// An increment the store recorded no creation time for is older than every
/// one it did, and is listed first; the answer says how many those are.
#[test]
fn an_increment_with_no_recorded_creation_time_is_listed_first_and_counted() {
    let mut old = design();
    plan(&mut old, "epoch:old");
    let mut doc = old.export_graph_with_stamps().expect("export");
    for n in &mut doc.nodes {
        n.stamps.clear();
    }
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.set_stamp_clock(ticking()).expect("clock");
    g.import_graph(&doc).expect("import");
    plan(&mut g, "epoch:a-new-one");
    let all = g.open_increments().unwrap();
    let ids: Vec<&str> = all.increments.iter().map(|i| i.epoch_id.as_str()).collect();
    assert_eq!(ids, ["epoch:old", "epoch:a-new-one"]);
    assert_eq!(all.increments_with_no_recorded_time, 1);

    // The document says "oldest first", so it says where that is not known.
    let md = g.graph_report().unwrap().to_markdown();
    assert!(
        md.contains("kept no creation time for 1 of the 2 open increment(s)"),
        "{md}"
    );
    // And a design whose every record has its time says nothing of the kind.
    let mut stamped = design();
    plan(&mut stamped, "epoch:one");
    let md = stamped.graph_report().unwrap().to_markdown();
    assert!(md.contains("## Open increments"), "{md}");
    assert!(!md.contains("kept no creation time"), "{md}");
}

// ─── One computation, read by every surface, and bounded ───────────────────

fn busy(g: &mut DesignGraph, n: usize) {
    for i in 0..n {
        let e = format!("epoch:{i:02}");
        plan(g, &e);
        if i % 3 != 0 {
            need(g, &format!("req:{i:02}"), &e);
        }
        if i % 3 == 1 {
            deliver(g, &format!("req:{i:02}"), &format!("{i:02}"));
        }
    }
    g.add_epoch("epoch:landed", "Landed", EpochType::Revision, 0)
        .unwrap();
    g.precedes("epoch:02", "epoch:landed").unwrap();
    change(g, "chg:1", &[(node::REQUIREMENT, "req:02")]);
    change(g, "chg:2", &[]);
}

#[test]
fn loop_status_and_graph_report_read_the_one_answer() {
    let mut g = design();
    busy(&mut g, 6);
    let all = g.open_increments().unwrap();
    assert_eq!(
        (all.open, all.done_but_not_closed, all.overtaken, all.empty),
        (6, 2, 1, 2)
    );
    let listed = all.listed(OPEN_INCREMENTS_LISTED);
    let status = g.loop_status().expect("loop_status");
    let report = g.graph_report().expect("graph_report");
    assert_eq!(status.open_increments, listed);
    assert_eq!(report.open_increments, listed);

    // The document a person reads says the same counts and the same rows.
    let md = report.to_markdown();
    assert!(md.contains("## Open increments"), "{md}");
    assert!(
        md.contains("**6** open") && md.contains("2 done but not closed"),
        "{md}"
    );
    assert!(md.contains("1 overtaken") && md.contains("2 empty"), "{md}");
    for i in &listed.increments {
        assert!(md.contains(&format!("`{}`", i.epoch_id)), "{md}");
    }

    // And loop_status says it in one line a session reads at its end, which
    // is never counted against `clean`.
    let line = status
        .next
        .iter()
        .find(|l| l.contains("open increment"))
        .expect("a line in next");
    assert!(line.contains("6 open increment"), "{line}");
    assert!(line.contains("Not counted against `clean`"), "{line}");
}

#[test]
fn open_increments_do_not_make_a_clean_design_unclean() {
    let mut g = design();
    let before = g.loop_status().unwrap();
    plan(&mut g, "epoch:a");
    let after = g.loop_status().unwrap();
    assert_eq!(after.clean, before.clean);
    assert_eq!(after.open_increments.open, 1);
    assert_eq!(before.open_increments.open, 0);
    assert!(
        !before.next.iter().any(|l| l.contains("open increment")),
        "a design with none says nothing about them in next"
    );
}

#[test]
fn a_long_list_is_bounded_and_keeps_its_counts() {
    let mut g = design();
    busy(&mut g, 25);
    let all = g.open_increments().unwrap();
    assert_eq!(all.open, 25);
    assert_eq!(all.increments.len(), 25, "the whole list is the answer");
    assert_eq!(all.full_list, None, "nothing is left to point at");

    let listed = g.loop_status().unwrap().open_increments;
    assert_eq!(listed.increments.len(), OPEN_INCREMENTS_LISTED);
    assert_eq!(
        listed.full_list.as_deref(),
        Some("graph_report with include_open_increments: true lists all 25."),
        "the rest are counted by the whole count, and the call that lists them is named"
    );
    // Every count is the whole design's, not the listed rows'.
    assert_eq!(
        (
            listed.open,
            listed.done_but_not_closed,
            listed.overtaken,
            listed.empty,
            listed.never_moved
        ),
        (
            all.open,
            all.done_but_not_closed,
            all.overtaken,
            all.empty,
            all.never_moved
        )
    );
    // The rows listed are the OLDEST, in order.
    assert_eq!(
        listed.increments,
        all.listed(OPEN_INCREMENTS_LISTED).increments
    );
    assert_eq!(listed.increments[0].epoch_id, "epoch:00");
    // And it names the call that shows the rest.
    let full = listed.full_list.as_deref().expect("names the call");
    assert!(
        full.contains("graph_report") && full.contains("include_open_increments"),
        "{full}"
    );
    let md = g.graph_report().unwrap().to_markdown();
    assert!(md.contains("15 more"), "{md}");
}

/// A long name is cut in the bounded list, where the row says so; the whole
/// list keeps it whole.
#[test]
fn a_long_name_is_cut_in_the_bounded_list_and_the_row_says_so() {
    let mut g = design();
    let long = "A very long name for an increment ".repeat(8);
    g.plan_epoch("epoch:long", long.trim(), EpochType::Milestone, 1)
        .unwrap();
    plan(&mut g, "epoch:short");
    let all = g.open_increments().unwrap();
    assert_eq!(all.increments[0].name, long.trim());
    assert!(!all.increments[0].name_cut);
    let listed = all.listed(OPEN_INCREMENTS_LISTED);
    assert!(listed.increments[0].name.chars().count() < 80);
    assert!(listed.increments[0].name_cut);
    assert!(!listed.increments[1].name_cut);
}

#[test]
fn it_is_worked_out_once_per_state_of_the_design() {
    let mut g = design();
    busy(&mut g, 6);
    g.loop_status().unwrap();
    g.graph_report().unwrap();
    g.open_increments().unwrap();
    assert_eq!(g.kept_builds("open_increments"), 1);
    change(&mut g, "chg:late", &[]);
    g.graph_report().unwrap();
    g.loop_status().unwrap();
    assert_eq!(g.kept_builds("open_increments"), 2, "a write drops it");
}
