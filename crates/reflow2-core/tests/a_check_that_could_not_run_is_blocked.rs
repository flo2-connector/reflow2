//! A check that COULD NOT RUN lands as `blocked`, and `blocked` is visible
//! without claiming the part failed.
//!
//! # The failure this pins
//!
//! `fact:root-cause-a-check-that-did-not-run-reads-did-not-work-as-designed-because-only-failing-is-loud-2026-10-02`
//! (the VS Code `--call` field report, tool friction 3, measured on 0.77.0): a
//! pytest COLLECTION error (no test body ran) was recorded as `failing`, and
//! detect_gaps then said the part "did not work as designed". The vocabulary
//! could already say "did not run" — `Verification.status` has `blocked` — but
//! the three routes that matter could not use it:
//! - `reconcile_verification` took only passed / failed / skipped;
//! - `tools/run_to_files.py` mapped a JUnit `<error>` to failed (pinned in
//!   `tools/test_run_to_files.py`);
//! - a `blocked` check raised no gap at all.
//!
//! So the honest status was the quiet one, and the dishonest one was loud.
//!
//! OBSERVED FAILING on main at 293f957 before the fix: `blocked` was refused
//! by the reconcile as "not one of passed/failed/skipped", the file resolver
//! ranked it as unknown (above a failure), and no gap named a blocked check.

use reflow2_core::detect::GapSource;
use reflow2_core::graph::DesignGraph;
use reflow2_core::nodes::{Props, node};
use reflow2_core::verify::{ObservedFile, ObservedVerification, VerifyReconcileOptions};

fn world() -> DesignGraph {
    let mut g = DesignGraph::open_in_memory().expect("open");
    g.add_project("proj:1", "Thing").expect("project");
    g.add_requirement("req:works", "It works", "The thing must work.")
        .expect("req");
    g.add_capability(
        "cap:parse",
        "Parser reads config files",
        "reads them",
        Some("realized"),
    )
    .expect("cap");
    g.satisfies("cap:parse", "req:works").expect("sat");
    g.add_verification(
        "ver:parse",
        "parser test suite",
        Some("test"),
        Some("unit"),
        None,
    )
    .expect("ver");
    g.upsert_node(
        node::VERIFICATION,
        "ver:parse",
        Props::new().set("location", "tests/test_parse.py"),
    )
    .expect("location");
    g.verifies("ver:parse", node::CAPABILITY, "cap:parse")
        .expect("verifies");
    g
}

fn obs(id: &str, outcome: &str) -> ObservedVerification {
    ObservedVerification {
        verification_id: id.to_string(),
        outcome: outcome.to_string(),
    }
}

fn stamp(g: &DesignGraph) -> Option<String> {
    g.get_node(node::VERIFICATION, "ver:parse")
        .unwrap()
        .unwrap()
        .properties
        .get("last_reconciled_outcome")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

#[test]
fn a_run_can_report_that_it_could_not_run_a_check() {
    let mut g = world();
    g.set_verification_status("ver:parse", "blocked", None, None)
        .expect("status");
    let r = g
        .reconcile_verification(
            &[obs("ver:parse", "blocked")],
            &VerifyReconcileOptions {
                record_events: true,
                exhaustive: false,
                detected_at: Some("2026-10-03T00:00:00Z".into()),
            },
        )
        .expect("reconcile");
    assert!(r.rejected.is_empty(), "`blocked` is an outcome: {r:?}");
    assert_eq!(r.agreements, 1, "blocked agrees with blocked: {r:?}");
    assert!(r.findings.is_empty(), "{r:?}");
    assert_eq!(stamp(&g).as_deref(), Some("blocked"));
}

#[test]
fn a_check_believed_passing_that_could_not_run_is_a_divergence_but_not_a_breakage() {
    let mut g = world();
    g.set_verification_status("ver:parse", "passing", None, None)
        .expect("status");
    let r = g
        .reconcile_verification(
            &[obs("ver:parse", "blocked")],
            &VerifyReconcileOptions {
                record_events: true,
                exhaustive: false,
                detected_at: Some("2026-10-03T00:00:00Z".into()),
            },
        )
        .expect("reconcile");
    assert_eq!(r.findings.len(), 1, "{r:?}");
    assert_eq!(r.findings[0].observed, "blocked");
    let event = r.findings[0].event_id.clone().expect("recorded");
    let severity = g
        .get_node(node::DRIFT_EVENT, &event)
        .unwrap()
        .unwrap()
        .properties
        .get("severity")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    assert_eq!(
        severity.as_deref(),
        Some("medium"),
        "only believed-passing-actually-FAILED is the high, broken direction"
    );
    // The stamp never touches the claim.
    let status = g
        .get_node(node::VERIFICATION, "ver:parse")
        .unwrap()
        .unwrap()
        .properties
        .get("status")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    assert_eq!(status.as_deref(), Some("passing"));
}

#[test]
fn a_file_that_could_not_run_outranks_a_pass_and_never_hides_a_failure() {
    let mut g = world();
    g.add_artifact(
        "art:parse-helpers",
        "parser helper tests",
        Some("test"),
        Some("tests/test_helpers.py"),
    )
    .expect("artifact");
    g.create_edge(
        "IMPLEMENTS",
        node::ARTIFACT,
        "art:parse-helpers",
        node::VERIFICATION,
        "ver:parse",
        std::collections::HashMap::new(),
    )
    .expect("implements");
    let file = |l: &str, o: &str| ObservedFile {
        location: l.into(),
        outcome: o.into(),
    };
    let worst = |files: &[ObservedFile]| {
        g.resolve_observed_files(files).expect("resolve").observed[0]
            .outcome
            .clone()
    };
    assert_eq!(
        worst(&[
            file("tests/test_parse.py", "passed"),
            file("tests/test_helpers.py", "blocked")
        ]),
        "blocked",
        "a check one of whose files never ran cannot read as passed"
    );
    assert_eq!(
        worst(&[
            file("tests/test_parse.py", "failed"),
            file("tests/test_helpers.py", "blocked")
        ]),
        "failed",
        "a failure is evidence; a blocked file is not, and never hides it"
    );
}

#[test]
fn a_blocked_check_raises_a_gap_that_never_says_the_part_failed() {
    let mut g = world();
    g.set_verification_status(
        "ver:parse",
        "blocked",
        Some("2026-10-02T09:00:00Z"),
        Some("pytest collection error: ImportError in conftest.py; 0 tests ran"),
    )
    .expect("status");
    let gaps = g.detect_gaps().expect("gaps");
    assert!(
        !gaps
            .iter()
            .any(|x| x.gap_source == GapSource::FailingVerification),
        "a check that never ran is not a failing one"
    );
    let blocked = gaps
        .iter()
        .find(|x| x.gap_source == GapSource::BlockedVerification)
        .unwrap_or_else(|| {
            panic!(
                "a blocked check must be visible: {:?}",
                gaps.iter().map(|x| x.gap_source).collect::<Vec<_>>()
            )
        });
    assert_eq!(blocked.affected_ids, ["cap:parse", "ver:parse"]);
    assert!(
        blocked.severity < 0.8,
        "below the failing gap and the build-stopping line: {}",
        blocked.severity
    );
    let text = format!(
        "{} {} {}",
        blocked.title, blocked.description, blocked.evidence
    );
    assert!(text.contains("could not run"), "{text}");
    assert!(text.contains("2026-10-02T09:00:00Z"), "says when: {text}");
    assert!(
        text.contains("Parser reads config files"),
        "names what it checks: {text}"
    );
    for overstated in ["did not work", "is failing", "broken"] {
        assert!(
            !text.contains(overstated),
            "never claims the part failed (`{overstated}`): {text}"
        );
    }
    assert_eq!(
        GapSource::BlockedVerification.as_str(),
        "blocked_verification"
    );
}

#[test]
fn a_failing_check_still_says_the_part_did_not_work() {
    // Regression cover: the blocked gap must not have taken the failing one's
    // place.
    let mut g = world();
    g.set_verification_status("ver:parse", "failing", None, None)
        .expect("status");
    let gaps = g.detect_gaps().expect("gaps");
    assert!(
        gaps.iter()
            .any(|x| x.gap_source == GapSource::FailingVerification)
    );
    assert!(
        !gaps
            .iter()
            .any(|x| x.gap_source == GapSource::BlockedVerification)
    );
}
