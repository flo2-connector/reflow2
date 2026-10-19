//! Every constructor that takes a `status` stores exactly the status it was
//! given, or refuses it saying why — and never draws an approver signature on
//! a status it did not store.
//!
//! # The finding this pins
//!
//! `fact:add-decision-given-status-rejected-and-an-approver-writes-proposed-and-says-nothing-2026-10-18`.
//! Measured on the published 0.87.0: `add_decision` with an approver stored
//! `accepted` and `deferred` as given, but stored `rejected` and `superseded`
//! as `proposed` — with the AUTHORED_BY role=approver edge drawn all the same,
//! so a proposed Decision carried the owner's signature, and the only word in
//! the reply was the loop hint "a Decision lands `proposed`".
//!
//! # The class
//!
//! The constructors with an approver decided "is the status written?" with
//! the same boolean that decides "does this call need the owner's name?" —
//! the settle table (`crate::settles`). On 2026-09-06 (#440) the two coincided
//! (any status but `proposed`). On 2026-09-29 (#635) the Decision row was
//! narrowed to the statuses that DECIDE, so that an unsigned `rejected` was no
//! longer refused; the write was narrowed with it, silently, while the
//! signature kept being drawn unconditionally. `add_requirement` stayed right
//! only because its row still reads "anything but `proposed`".
//!
//! So the test walks EVERY constructor that takes a `status`, EVERY value its
//! schema allows, on create and on revise, with and without an approver where
//! the tool takes one — not the two values that were reported.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

async fn svc() -> ReflowService {
    let s = ReflowService::in_memory().expect("in-memory service");
    s.add_contributor(Parameters(ContributorReq {
        id: "who:ann".into(),
        name: Some("Ann".into()),
        kind: None,
        handle: None,
        description: None,
    }))
    .await
    .expect("contributor");
    s
}

/// Call one constructor by name. `Ok` carries the reply, `Err` the refusal.
async fn call(s: &ReflowService, tool: &str, args: Value) -> Result<Value, String> {
    macro_rules! go {
        ($m:ident) => {
            s.$m(Parameters(serde_json::from_value(args).expect("args fit")))
                .await
                .map(|r| r.structured_content.unwrap_or(Value::Null))
                .map_err(|e| e.message.to_string())
        };
    }
    match tool {
        "add_decision" => go!(add_decision),
        "add_requirement" => go!(add_requirement),
        "add_project" => go!(add_project),
        "add_capability" => go!(add_capability),
        "add_component" => go!(add_component),
        "add_artifact" => go!(add_artifact),
        "add_verification" => go!(add_verification),
        "add_release" => go!(add_release),
        "add_epoch" => go!(add_epoch),
        other => panic!("no constructor {other} in this test"),
    }
}

async fn doc(s: &ReflowService) -> Value {
    s.export_graph(Parameters(ExportGraphToReq {
        path: None,
        overwrite: None,
        accept_divergence: None,
    }))
    .await
    .expect("export")
    .structured_content
    .expect("structured")
}

/// (stored status, carries an approver signature)
async fn state(s: &ReflowService, id: &str) -> (Option<String>, bool) {
    let d = doc(s).await;
    let status = d["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|n| n["node_id"] == id || n["id"] == id)
        .and_then(|n| n["properties"]["status"].as_str().map(str::to_string));
    let signed = d["edges"].as_array().expect("edges").iter().any(|e| {
        e["edge_type"] == "AUTHORED_BY"
            && e["from_id"] == id
            && e["properties"]["roles"]
                .as_array()
                .is_some_and(|r| r.iter().any(|x| x == "approver"))
    });
    (status, signed)
}

struct Ctor {
    tool: &'static str,
    prefix: &'static str,
    /// Every value the schema allows for `status`.
    statuses: &'static [&'static str],
    /// Takes an `approver`, and the status it lands at without one.
    landing: Option<&'static str>,
    /// The content fields a create needs.
    content: fn() -> Value,
}

const CTORS: &[Ctor] = &[
    Ctor {
        tool: "add_decision",
        prefix: "dec",
        statuses: &[
            "proposed",
            "accepted",
            "deferred",
            "contested",
            "superseded",
            "rejected",
        ],
        landing: Some("proposed"),
        content: || json!({"name": "A choice faced", "decision": "Option one over option two."}),
    },
    Ctor {
        tool: "add_requirement",
        prefix: "req",
        statuses: &["proposed", "accepted", "deferred", "dropped", "met"],
        landing: Some("proposed"),
        content: || json!({"name": "A need", "statement": "Something must hold."}),
    },
    Ctor {
        tool: "add_project",
        prefix: "proj",
        statuses: &["active", "paused", "archived"],
        landing: None,
        content: || json!({"name": "A project", "description": "What it is for."}),
    },
    Ctor {
        tool: "add_capability",
        prefix: "cap",
        statuses: &["planned", "in_progress", "realized", "verified"],
        landing: None,
        content: || json!({"name": "A function", "description": "Does the thing."}),
    },
    Ctor {
        tool: "add_component",
        prefix: "cmp",
        statuses: &["planned", "in_progress", "realized", "verified"],
        landing: None,
        content: || json!({"name": "A part", "description": "Holds the thing."}),
    },
    Ctor {
        tool: "add_artifact",
        prefix: "art",
        statuses: &["planned", "in_progress", "realized", "verified"],
        landing: None,
        content: || json!({"name": "a file", "artifact_type": "code"}),
    },
    Ctor {
        tool: "add_verification",
        prefix: "ver",
        statuses: &[
            "planned",
            "passing",
            "failing",
            "skipped",
            "blocked",
            "superseded",
        ],
        landing: None,
        content: || json!({"name": "A check", "method": "test", "description": "Checks it."}),
    },
    Ctor {
        tool: "add_release",
        prefix: "rel",
        statuses: &["planned", "built", "deployed", "retired"],
        landing: None,
        content: || json!({"name": "A release", "version": "1.0.0"}),
    },
    Ctor {
        tool: "add_epoch",
        prefix: "epoch",
        statuses: &["planned", "arrived"],
        landing: None,
        content: || json!({"name": "An epoch", "epoch_type": "milestone"}),
    },
];

fn args(c: &Ctor, id: &str, status: Option<&str>, approver: bool, create: bool) -> Value {
    let mut a = if create { (c.content)() } else { json!({}) };
    // Every node is unlike the last on purpose: the near-duplicate guard is
    // not what this test is about.
    if create {
        for k in ["name", "statement", "description", "decision"] {
            if let Some(v) = a.get(k).and_then(Value::as_str) {
                a[k] = json!(format!("{v} ({id})"));
            }
        }
    }
    a["id"] = json!(id);
    if create && c.tool == "add_epoch" {
        // An epoch is a position on the time axis, and each needs its own.
        static NEXT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(1);
        a["sequence"] = json!(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    }
    if let Some(st) = status {
        a["status"] = json!(st);
    }
    if approver {
        a["approver"] = json!("who:ann");
    }
    a
}

/// One call, judged against the invariant. Returns a violation, if any.
async fn judge(
    s: &ReflowService,
    c: &Ctor,
    id: &str,
    status: Option<&str>,
    approver: bool,
    create: bool,
) -> Option<String> {
    let before = state(s, id).await;
    let what = format!(
        "{} {} {id} status={status:?} approver={approver}",
        c.tool,
        if create { "create" } else { "revise" }
    );
    match call(s, c.tool, args(c, id, status, approver, create)).await {
        Err(msg) => {
            // A refusal must say why, and must leave the node as it was.
            let after = state(s, id).await;
            if after != before {
                return Some(format!(
                    "{what}: refused AND wrote: {before:?} -> {after:?}"
                ));
            }
            let names = msg.contains("status") || msg.contains("approver");
            (!names).then(|| format!("{what}: refused without naming why: {msg}"))
        }
        Ok(_) => {
            let (stored, signed) = state(s, id).await;
            if let Some(given) = status
                && stored.as_deref() != Some(given)
            {
                return Some(format!(
                    "{what}: stored {stored:?}, not the status it was given (signed={signed})"
                ));
            }
            // The signature this call drew must sign a status this call stored
            // past the landing one: never a status it did not store, and never
            // the landing state, which is nobody's word.
            let newly_signed = signed && !before.1;
            if newly_signed {
                let landed = c.landing.is_some_and(|l| stored.as_deref() == Some(l));
                if status.is_none() || landed {
                    return Some(format!(
                        "{what}: drew an approver signature on {stored:?}, a status this call \
                         did not settle"
                    ));
                }
            }
            None
        }
    }
}

#[tokio::test]
async fn every_constructor_stores_every_status_it_is_given_on_create() {
    let mut bad = Vec::new();
    for c in CTORS {
        let s = svc().await;
        for (i, st) in c.statuses.iter().enumerate() {
            let approvers: &[bool] = if c.landing.is_some() {
                &[false, true]
            } else {
                &[false]
            };
            for &ap in approvers {
                let id = format!("{}:create-{i}-{ap}", c.prefix);
                bad.extend(judge(&s, c, &id, Some(st), ap, true).await);
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} violations:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

#[tokio::test]
async fn every_constructor_stores_every_status_it_is_given_on_revise() {
    let mut bad = Vec::new();
    for c in CTORS {
        let s = svc().await;
        for (i, from) in c.statuses.iter().enumerate() {
            for (k, to) in c.statuses.iter().enumerate() {
                let id = format!("{}:revise-{i}-{k}", c.prefix);
                // Created where it starts, signed whenever the tool takes a
                // signature and the start is past its landing status.
                let sign_first = c.landing.is_some_and(|l| l != *from);
                call(&s, c.tool, args(c, &id, Some(from), sign_first, true))
                    .await
                    .unwrap_or_else(|e| panic!("{} create {id} at {from}: {e}", c.tool));
                let sign = c.landing.is_some_and(|l| l != *to);
                bad.extend(judge(&s, c, &id, Some(to), sign, false).await);
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} violations:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// The reported shape, named: an approver and a retiring status.
#[tokio::test]
async fn a_decision_recorded_rejected_or_superseded_by_its_owner_lands_there() {
    let s = svc().await;
    for st in ["rejected", "superseded"] {
        let id = format!("dec:retired-{st}");
        let c = &CTORS[0];
        let v = call(&s, "add_decision", args(c, &id, Some(st), true, true))
            .await
            .expect("recorded");
        assert_eq!(v["properties"]["status"], st, "{v}");
        assert_eq!(state(&s, &id).await, (Some(st.to_string()), true));
        let hint = v["loop_hint"].as_str().unwrap_or("");
        assert!(
            !hint.contains("lands `proposed`"),
            "the hint says it landed proposed: {v}"
        );
    }
}

/// An approver with nothing to sign is not drawn — the call says so rather
/// than signing the landing status.
#[tokio::test]
async fn an_approver_with_no_status_to_sign_draws_no_signature() {
    let mut bad = Vec::new();
    for c in CTORS.iter().filter(|c| c.landing.is_some()) {
        let s = svc().await;
        let id = format!("{}:no-status", c.prefix);
        bad.extend(judge(&s, c, &id, None, true, true).await);
        let id = format!("{}:explicit-landing", c.prefix);
        bad.extend(judge(&s, c, &id, c.landing, true, true).await);
        // And the reply SAYS it was not drawn, as the tool's field text claims.
        // Not refused: a client that passes its approver on every call keeps
        // working.
        for (st, id) in [(None, "noted-none"), (c.landing, "noted-landing")] {
            let id = format!("{}:{id}", c.prefix);
            match call(&s, c.tool, args(c, &id, st, true, true)).await {
                Ok(v)
                    if v["approver_not_drawn"]
                        .as_str()
                        .is_some_and(|n| n.contains("NOT drawn")) => {}
                other => bad.push(format!("{} {id}: not said: {other:?}", c.tool)),
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} violations:\n{}",
        bad.len(),
        bad.join("\n")
    );
}

/// A status the schema does not know is refused, naming the legal ones, and
/// leaves nothing behind. Before 2026-10-19 `add_decision` stored such a
/// Decision `proposed` without a word: the status did not settle, so it was
/// never written.
#[tokio::test]
async fn an_unknown_status_is_refused_and_nothing_is_written() {
    for c in CTORS.iter().filter(|c| c.landing.is_some()) {
        let s = svc().await;
        let id = format!("{}:unknown-status", c.prefix);
        let err = call(&s, c.tool, args(c, &id, Some("pondered"), false, true))
            .await
            .expect_err("refused");
        assert!(
            err.contains("pondered") && err.contains("proposed"),
            "{err}"
        );
        assert_eq!(
            state(&s, &id).await,
            (None, false),
            "{}: nothing written",
            c.tool
        );
    }
}
