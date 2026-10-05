//! What the store holds that its last export does not, grouped by who wrote it.
//!
//! WRITTEN BEFORE THE IMPLEMENTATION, from the VS Code field log of 2026-10-05
//! (`art:vscode-call-field-log-2026-10-05`): one store held another session's
//! uncommitted work, so an export to commit one session's changes either swept
//! the other's in or was skipped, and twice in one day it was skipped. The
//! reporter's idea: list the unexported changes so the person can see whose
//! they are before exporting.
//!
//! What is pinned here is the computation, on the in-memory backend:
//!   · two sessions' writes since the record come back as two groups, each
//!     with its own writer, agent, counts and ids;
//!   · a write the store recorded no author for is counted under
//!     `written_by: None`, never guessed into somebody's group;
//!   · removals are counted (nothing records who removed an item);
//!   · an epoch a ChangeEvent pins a change to is the group's epoch;
//!   · an up-to-date store reports nothing;
//!   · on a large store the reply stays bounded and still counts everything.

use reflow2_core::DesignGraph;
use reflow2_core::acting::Acting;
use reflow2_core::temporal::{ChangeAction, ChangeType, EpochType};
use reflow2_core::unexported::{MAX_GROUPS, NAMED_PER_GROUP, UnexportedChanges, UnexportedGroup};

const RECORD: &str = "docs/design/proj.json";

/// A design with three people and two agents, exported: the record.
fn exported_design() -> (DesignGraph, reflow2_core::GraphExport) {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("proj:p", "P").unwrap();
    for (id, name, kind) in [
        ("who:alice", "Alice", "person"),
        ("who:bob", "Bob", "person"),
        ("agent:claude", "Claude Code", "automated_agent"),
        ("agent:copilot", "Copilot", "automated_agent"),
    ] {
        g.add_contributor(id, name, Some(kind), None, None).unwrap();
    }
    g.add_requirement("req:base", "Base", "Was in the record.")
        .unwrap();
    g.add_requirement("req:gone", "Gone", "Was in the record, then removed.")
        .unwrap();
    let record = g.export_graph().unwrap();
    (g, record)
}

/// Write `ids` as requirements for `who`, through `agent`, the way the server
/// credits a session that declared `writes_for`: recording on, the writes,
/// then the credit while the agent is still in force.
fn write_as(g: &mut DesignGraph, who: &str, agent: &str, ids: &[&str]) {
    g.begin_acting(Acting {
        agent: agent.to_string(),
        route: "session".to_string(),
    })
    .unwrap();
    g.begin_touch_log();
    for id in ids {
        g.add_requirement(id, id, "Written since the record.")
            .unwrap();
    }
    let touched = g.take_touch_log();
    g.credit_writes(&touched, who).unwrap();
    g.end_acting();
}

fn group<'a>(u: &'a UnexportedChanges, who: Option<&str>) -> &'a UnexportedGroup {
    u.groups
        .iter()
        .find(|gr| gr.written_by.as_deref() == who)
        .unwrap_or_else(|| panic!("no group for {who:?} in {:#?}", u.groups))
}

#[test]
fn two_sessions_writes_since_the_record_come_back_as_two_groups() {
    let (mut g, record) = exported_design();

    write_as(&mut g, "who:alice", "agent:claude", &["req:a1", "req:a2"]);
    write_as(&mut g, "who:bob", "agent:copilot", &["req:b1"]);

    let u = g.unexported_changes(&record, RECORD).unwrap();
    assert!(!u.identical, "the store moved on from the record");

    let alice = group(&u, Some("who:alice"));
    assert_eq!(alice.via, vec!["agent:claude".to_string()]);
    assert_eq!(alice.counts.nodes_added, 2, "{alice:#?}");
    assert!(alice.ids.contains(&"req:a1".to_string()), "{alice:#?}");
    assert!(alice.ids.contains(&"req:a2".to_string()), "{alice:#?}");

    let bob = group(&u, Some("who:bob"));
    assert_eq!(bob.via, vec!["agent:copilot".to_string()]);
    assert_eq!(bob.counts.nodes_added, 1, "{bob:#?}");
    assert_eq!(bob.ids.first().map(String::as_str), Some("req:b1"));

    // Neither session's work leaks into the other's group.
    assert!(!alice.ids.iter().any(|i| i == "req:b1"));
    assert!(!bob.ids.iter().any(|i| i.starts_with("req:a")));

    // Each group's own credit edges ride with it: the AUTHORED_BY edges are
    // part of what an export would carry for that session.
    assert!(alice.counts.edges_added >= 2, "{alice:#?}");
    assert!(bob.counts.edges_added >= 1, "{bob:#?}");

    // The pointer to the full list is the diff that already exists.
    assert!(
        u.full_list.contains("compare_designs") && u.full_list.contains(RECORD),
        "{}",
        u.full_list
    );
}

#[test]
fn a_write_nobody_was_credited_with_is_counted_and_never_guessed() {
    let (mut g, record) = exported_design();
    write_as(&mut g, "who:alice", "agent:claude", &["req:a1"]);
    // A write with no session declared — the --call door's normal case.
    g.add_requirement("req:anon", "Anon", "Nobody was credited.")
        .unwrap();
    // A change to a node the record holds, also uncredited.
    g.add_requirement("req:base", "Base", "Changed since the record.")
        .unwrap();
    // A removal — nothing records who removes a node.
    g.delete_node("Requirement", "req:gone").unwrap();

    let u = g.unexported_changes(&record, RECORD).unwrap();

    let nobody = group(&u, None);
    assert_eq!(nobody.counts.nodes_added, 1, "{nobody:#?}");
    assert_eq!(nobody.counts.nodes_changed, 1, "{nobody:#?}");
    assert_eq!(nobody.counts.nodes_removed, 1, "{nobody:#?}");
    for id in ["req:anon", "req:base", "req:gone"] {
        assert!(nobody.ids.iter().any(|i| i == id), "{id} in {nobody:#?}");
    }
    assert!(
        nobody.summary.contains("no record of who"),
        "the group says why it has no writer: {}",
        nobody.summary
    );

    let alice = group(&u, Some("who:alice"));
    assert!(
        !alice.ids.iter().any(|i| i == "req:anon" || i == "req:base"),
        "uncredited work must never land in a person's group: {alice:#?}"
    );

    assert_eq!(u.totals.nodes_added, 2);
    assert_eq!(u.totals.nodes_changed, 1);
    assert_eq!(u.totals.nodes_removed, 1);
    assert!(
        u.note.contains("writes_for"),
        "the note says how a write comes to be credited: {}",
        u.note
    );
}

#[test]
fn a_change_pinned_to_an_epoch_is_grouped_under_that_epoch() {
    let (mut g, record) = exported_design();
    g.add_epoch("epoch:inc-2", "Increment 2", EpochType::Milestone, 2)
        .unwrap();
    g.add_requirement("req:planned", "Planned", "Delivered in increment 2.")
        .unwrap();
    g.add_change_event(
        "chg:inc-2-work",
        "Increment 2 work",
        ChangeType::NewFeature,
        None,
        None,
        None,
        Some("2026-10-04"),
    )
    .unwrap();
    g.pin_at_epoch("ChangeEvent", "chg:inc-2-work", "epoch:inc-2")
        .unwrap();
    g.changed(
        "chg:inc-2-work",
        "Requirement",
        "req:planned",
        ChangeAction::Added,
    )
    .unwrap();
    // Unpinned work beside it.
    g.add_requirement("req:loose", "Loose", "No epoch.")
        .unwrap();

    let u = g.unexported_changes(&record, RECORD).unwrap();
    let in_epoch = u
        .groups
        .iter()
        .find(|gr| gr.epoch.as_deref() == Some("epoch:inc-2"))
        .unwrap_or_else(|| panic!("no epoch group in {:#?}", u.groups));
    assert!(
        in_epoch.ids.iter().any(|i| i == "req:planned"),
        "{in_epoch:#?}"
    );
    assert!(
        in_epoch.ids.iter().any(|i| i == "chg:inc-2-work"),
        "{in_epoch:#?}"
    );
    assert!(!in_epoch.ids.iter().any(|i| i == "req:loose"));
    let window = in_epoch.window.as_ref().expect("the change event is dated");
    assert_eq!(window.first, "2026-10-04");
    assert_eq!(window.last, "2026-10-04");
}

#[test]
fn an_up_to_date_store_reports_nothing() {
    let (g, record) = exported_design();
    let u = g.unexported_changes(&record, RECORD).unwrap();
    assert!(u.identical, "{u:#?}");
    assert!(u.groups.is_empty(), "{u:#?}");
    assert_eq!(u.groups_not_shown, 0);
    assert_eq!(u.totals.total(), 0);
}

#[test]
fn the_list_stays_bounded_on_a_large_store_and_still_counts_everything() {
    let (mut g, record) = exported_design();
    let people = 30usize;
    let per_person = 20usize;
    for p in 0..people {
        let who = format!("who:p{p:02}");
        g.add_contributor(&who, &who, Some("person"), None, None)
            .unwrap();
        let ids: Vec<String> = (0..per_person)
            .map(|i| format!("req:p{p:02}-{i:02}"))
            .collect();
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        write_as(&mut g, &who, "agent:claude", &refs);
    }

    let u = g.unexported_changes(&record, RECORD).unwrap();
    assert!(
        u.groups.len() <= MAX_GROUPS,
        "{} groups shown, bound {MAX_GROUPS}",
        u.groups.len()
    );
    assert!(
        u.groups_not_shown > 0,
        "the rest are counted, never dropped"
    );
    for gr in &u.groups {
        assert!(
            gr.ids.len() <= NAMED_PER_GROUP,
            "{} ids named in one group, bound {NAMED_PER_GROUP}",
            gr.ids.len()
        );
        assert_eq!(gr.ids.len() + gr.ids_not_shown, gr.counts.total());
    }
    // Every added requirement is still counted in the totals.
    assert!(
        u.totals.nodes_added >= people * per_person,
        "{:?}",
        u.totals
    );
    // And the whole answer is small enough to ride on loop_status.
    let size = serde_json::to_string(&u).unwrap().len();
    assert!(size < 12_000, "the unexported list is {size} chars");
}
