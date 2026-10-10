//! The confirmation ledger reads each artifact's history once, however many
//! capabilities that artifact realizes.
//!
//! Root cause, 2026-10-09
//! (fact:the-confirmation-ledger-re-read-every-artifacts-history-once-per-capability-2026-10-09):
//! - On reflow2's own design, `loop_status` took 23 s, and 17.4 s of it was the
//!   ledger.
//! - An artifact such as the MCP service realizes over a hundred capabilities,
//!   and every accept on it was re-read and re-classified for each one: 943,665
//!   node reads where 4,001 events' targets would do.
//! - On flo2.io that is over the gateway's 30 s door, which blocked moving
//!   reflow2's design there, and it broke the 2,000 ms limit accepted for a warm
//!   loop_status (con:loop-status-warm-latency-ms).
//!
//! This pins the CLASS, not the instance: it counts the ledger's store reads, so
//! a rollup that recomputes shared per-artifact or per-event facts per
//! capability fails it whatever the hardware.

use std::cell::Cell;
use std::collections::HashMap;

use reflow2_core::foundation::core::{DynoError, Value};
use reflow2_core::graph::DesignGraph;
use reflow2_core::graph_read::GraphRead;
use reflow2_core::nodes::{edge, node};
use reflow2_core::temporal::ChangeType;
use reflow2_core::{StoredEdge, StoredNode};

/// Counts every read the ledger makes through the contract it takes.
struct Counting<'a> {
    g: &'a DesignGraph,
    reads: Cell<usize>,
}

impl GraphRead for Counting<'_> {
    fn get_node(&self, node_type: &str, id: &str) -> Result<Option<StoredNode>, DynoError> {
        self.reads.set(self.reads.get() + 1);
        GraphRead::get_node(self.g, node_type, id)
    }
    fn scan_nodes(&self, node_type: &str) -> Result<Vec<StoredNode>, DynoError> {
        self.reads.set(self.reads.get() + 1);
        GraphRead::scan_nodes(self.g, node_type)
    }
    fn count_nodes(&self, node_type: &str) -> Result<usize, DynoError> {
        self.reads.set(self.reads.get() + 1);
        GraphRead::count_nodes(self.g, node_type)
    }
    fn outgoing(
        &self,
        from_id: &str,
        edge_type: Option<&str>,
    ) -> Result<Vec<StoredEdge>, DynoError> {
        self.reads.set(self.reads.get() + 1);
        GraphRead::outgoing(self.g, from_id, edge_type)
    }
    fn incoming(&self, to_id: &str, edge_type: Option<&str>) -> Result<Vec<StoredEdge>, DynoError> {
        self.reads.set(self.reads.get() + 1);
        GraphRead::incoming(self.g, to_id, edge_type)
    }
}

/// `caps` capabilities all realized by one shared artifact, which carries
/// `accepts` accepted changes, each of which also changed `others` other
/// artifacts and one requirement (so each accept moved the design).
fn shared_history(caps: usize, accepts: usize, others: usize) -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("prj:p", "P").unwrap();
    g.add_requirement("req:r", "R", "A requirement.").unwrap();
    g.add_artifact(
        "art:shared",
        "service.rs",
        Some("code"),
        Some("src/service.rs"),
    )
    .unwrap();
    for k in 0..others {
        g.add_artifact(&format!("art:o{k}"), "other.rs", Some("code"), None)
            .unwrap();
    }
    for c in 0..caps {
        let cap = format!("cap:c{c}");
        g.add_capability(&cap, "C", "A capability.", Some("realized"))
            .unwrap();
        g.create_edge(
            edge::REALIZES,
            node::ARTIFACT,
            "art:shared",
            node::CAPABILITY,
            &cap,
            HashMap::new(),
        )
        .unwrap();
    }
    for a in 0..accepts {
        let chg = format!("chg:a{a}");
        g.add_change_event(
            &chg,
            "An accepted change",
            ChangeType::NewFeature,
            None,
            None,
            None,
            Some(&format!("2026-10-{:02}", 1 + a % 28)),
        )
        .unwrap();
        let accepted = HashMap::from([("accepted_baseline".to_string(), Value::from(true))]);
        g.create_edge(
            edge::CHANGED,
            node::CHANGE_EVENT,
            &chg,
            node::ARTIFACT,
            "art:shared",
            accepted,
        )
        .unwrap();
        for k in 0..others {
            g.create_edge(
                edge::CHANGED,
                node::CHANGE_EVENT,
                &chg,
                node::ARTIFACT,
                &format!("art:o{k}"),
                HashMap::new(),
            )
            .unwrap();
        }
        g.create_edge(
            edge::CHANGED,
            node::CHANGE_EVENT,
            &chg,
            node::REQUIREMENT,
            "req:r",
            HashMap::new(),
        )
        .unwrap();
    }
    g
}

fn reads_for(
    caps: usize,
    accepts: usize,
    others: usize,
) -> (usize, reflow2_core::ConfirmationLedger) {
    let g = shared_history(caps, accepts, others);
    let counting = Counting {
        g: &g,
        reads: Cell::new(0),
    };
    let ledger = reflow2_core::confirm::confirmation_ledger(&counting).unwrap();
    (counting.reads.get(), ledger)
}

#[test]
fn every_capability_still_sees_the_shared_artifacts_whole_history() {
    let (_, ledger) = reads_for(20, 15, 4);
    assert_eq!(ledger.claims.len(), 20);
    for c in &ledger.claims {
        assert_eq!(
            c.design_updated_claims, 15,
            "each accept also changed a requirement, so each moved the design: {}",
            c.capability_id
        );
        assert_eq!(c.design_holds_claims, 0);
        assert_eq!(c.artifacts, vec!["art:shared".to_string()]);
    }
}

#[test]
fn the_ledgers_reads_grow_with_the_design_not_with_capabilities_times_history() {
    // Doubling the capabilities that share one artifact must not double the
    // work spent on that artifact's history. Measured before the fix: the
    // reads were proportional to capabilities × accepts × targets.
    let (small, _) = reads_for(40, 30, 5);
    let (twice_the_capabilities, _) = reads_for(80, 30, 5);
    let per_capability = 12; // its own edges and nodes, a fixed handful
    assert!(
        twice_the_capabilities <= small + 40 * per_capability,
        "40 more capabilities on the SAME artifact cost {} more reads ({small} -> \
         {twice_the_capabilities}): the shared history is being re-read per capability",
        twice_the_capabilities - small
    );
    // And the whole is bounded by the design's own size: one pass over the
    // history (30 accepts × 7 targets) plus a fixed handful per capability.
    let bound = 30 * (5 + 2) * 2 + 80 * per_capability;
    assert!(
        twice_the_capabilities <= bound,
        "{twice_the_capabilities} reads for 80 capabilities and 30 accepts, bound {bound}"
    );
}
