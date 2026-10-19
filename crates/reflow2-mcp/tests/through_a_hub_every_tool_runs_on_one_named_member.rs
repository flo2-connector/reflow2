//! THROUGH A HUB, EVERY TOOL CAN RUN ON ONE NAMED MEMBER, AND EVERY ID IN A
//! ROUTED REPLY CARRIES ITS DESIGN'S PREFIX
//! (req:through-a-hub-every-tool-can-run-on-one-named-member, phase 4 of the
//! 2026-10-18 build list). Seen in the Arrow, cogito and trolley trials on
//! flo2.io: through the hub, a tool that takes no node id (loop_status,
//! reasoning_report with no claim, detect_gaps with no scope, describe_schema,
//! get_skill, claim_region, usage_report …) acted on the hub's own design or
//! fanned out over all of them, never on the one member asked for; a search
//! could not be kept to one member; a routed reply gave a step's `assumes` and
//! `conclusion` back without their prefix; and `add_release` naming
//! "<member>:rel:…" wrote that literal id into the hub.
//!
//! WHAT IS ASSERTED, over the real binary serving H, A and B under one root
//! behind a trusted gateway (the helpers are the hub view's own test file's):
//!   · THE LISTING: `tools/list` naming the view declares `member` on EVERY
//!     tool; the same listing with no view, or from a server that serves no
//!     view, declares it on none (a design alone publishes what it did)
//!   · AIMED: each tool the requirement names runs on A alone when the call
//!     passes `member: A` — no fan-out, the view note naming A — and the
//!     scan, the search, the lessons and the claim it answers with are A's
//!   · EVERY ID PREFIXED: every whole string in a routed reasoning_report that
//!     is an id A holds carries "A:", and a refusal's text gives back the id
//!     the call passed with its prefix
//!   · NO STRAY WRITE: `add_release` naming "A:rel:…" lands in A, and the hub
//!     holds nothing by either spelling
//!   · REFUSED, NOTHING RUN: `member` naming no design of the view, `member`
//!     beside an id of another member, `member` where no view is served

use serde_json::{Value, json};

use super::a_hubs_address_serves_a_view_over_its_members::{
    CALLER, GATEWAY, Session, VIEW, exchange, hub_and_members, ok, raw_reply, refused, serve,
};

/// `tools/list` on this session, its `_meta` naming `view` when given.
fn list(s: &mut Session, view: Option<&[&str]>) -> Vec<Value> {
    let id = s.next;
    s.next += 1;
    let mut body = json!({"jsonrpc": "2.0", "id": id, "method": "tools/list", "params": {}});
    if let Some(ids) = view {
        body["params"]["_meta"] = json!({ VIEW: ids });
    }
    let (_, reply) = exchange(s.port, &s.path, &body.to_string(), Some(&s.sid));
    let v: Value = serde_json::from_str(&raw_reply(&reply, id)).expect("a reply is JSON");
    v["result"]["tools"]
        .as_array()
        .unwrap_or_else(|| panic!("tools/list answers a list: {v}"))
        .clone()
}

fn declares_member(tool: &Value) -> bool {
    tool["inputSchema"]["properties"].get("member").is_some()
}

/// Every whole string anywhere in `v`.
fn strings(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => out.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|i| strings(i, out)),
        Value::Object(o) => o.values().for_each(|i| strings(i, out)),
        _ => {}
    }
}

/// The view note of a reply that ran on one design alone.
fn ran_alone_on(got: &Value, design: &str, what: &str) {
    assert!(
        got.get("by_design").is_none(),
        "{what}: one design's reply, not a fan-out: {got}"
    );
    let note = got["view_note"]
        .as_str()
        .unwrap_or_else(|| panic!("{what}: a view_note says where it ran: {got}"));
    assert!(
        note.contains(design),
        "{what}: the note names the design it ran on ({design}): {note}"
    );
}

#[test]
fn a_hubs_listing_declares_member_on_every_tool_and_a_design_alone_does_not() {
    let (root, h, a, b) = hub_and_members("member-listing");
    let server = serve(&root, Some(GATEWAY));
    let mut on_h = Session::open(server.port, &h);
    let view = [a.as_str(), b.as_str()];

    let as_hub = list(&mut on_h, Some(&view));
    assert!(
        as_hub.len() > 100,
        "the whole surface is listed: {}",
        as_hub.len()
    );
    let missing: Vec<&str> = as_hub
        .iter()
        .filter(|t| !declares_member(t))
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(
        missing.is_empty(),
        "through a hub's address every tool can be aimed at one member, so every tool \
         declares `member`; these do not: {missing:?}"
    );

    let alone = list(&mut on_h, None);
    assert_eq!(alone.len(), as_hub.len(), "the same tools either way");
    let declared: Vec<&str> = alone
        .iter()
        .filter(|t| declares_member(t))
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(
        declared.is_empty(),
        "a listing that names no view publishes exactly what it did: {declared:?}"
    );

    // A server that serves no view (no trusted host) lists no `member`
    // either, whatever the request names.
    drop(server);
    let untrusted = serve(&root, None);
    let mut on_h = Session::open(untrusted.port, &h);
    let declared = list(&mut on_h, Some(&view))
        .iter()
        .filter(|t| declares_member(t))
        .count();
    assert_eq!(
        declared, 0,
        "no view is served here, so no `member` is offered"
    );
}

#[test]
fn every_tool_the_trials_could_not_aim_runs_on_the_member_it_names() {
    let (root, h, a, b) = hub_and_members("member-aimed");
    let server = serve(&root, Some(GATEWAY));
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    // A holds a lesson on `add_requirement`, so a listing of that tool read
    // on A carries it and one read on H does not.
    let mut on_a = Session::open(server.port, &a);
    ok(
        &on_a.call(
            "record_finding",
            json!({"id": "fact:alpha-lesson", "subject_id": "req:alpha", "fact_type": "finding",
                   "statement": "lesson-only-in-alpha: name the member", "steps": ["add_requirement"]}),
            Some(&[a.as_str()]),
            None,
        ),
        "a lesson written on A",
    );

    let mut on_h = Session::open(server.port, &h);
    let aimed = |on_h: &mut Session, tool: &str, mut args: Value| -> Value {
        args["member"] = json!(a);
        let reply = on_h.call(tool, args.clone(), Some(&readable), Some(&view));
        let got = ok(&reply, &format!("{tool} {args} through H"));
        ran_alone_on(&got, &a, tool);
        got
    };

    // The tools with no node id, each on A alone.
    for (tool, args) in [
        ("loop_status", json!({})),
        ("reasoning_report", json!({})),
        ("detect_gaps", json!({})),
        ("describe_schema", json!({})),
        ("find_tools", json!({"query": "record a requirement"})),
        ("find_skills", json!({"query": "capture a requirement"})),
        ("get_skill", json!({"name": "gaps"})),
        ("get_instructions", json!({"budget_chars": 200})),
        ("usage_report", json!({"peek": true})),
    ] {
        aimed(&mut on_h, tool, args);
    }

    // The lessons served are A's.
    let got = aimed(
        &mut on_h,
        "describe_schema",
        json!({"tool": "add_requirement"}),
    );
    assert!(
        got.to_string().contains("lesson-only-in-alpha"),
        "describe_schema aimed at A serves A's lesson on the tool: {got}"
    );

    // A scan and a search cover A only.
    let got = aimed(
        &mut on_h,
        "scan_nodes",
        json!({"node_type": "Requirement", "brief": true}),
    );
    let mut ids = Vec::new();
    strings(&got, &mut ids);
    assert!(
        ids.contains(&format!("{a}:req:alpha")),
        "A's requirement, prefixed: {got}"
    );
    assert!(
        !ids.iter()
            .any(|s| s.ends_with("req:hub") || s.ends_with("req:beta")),
        "nothing from the hub or from B: {got}"
    );
    let got = aimed(&mut on_h, "search_design", json!({"query": "zanzibar"}));
    assert_eq!(
        got["hits"],
        json!([]),
        "B's word is not found when the search is kept to A: {got}"
    );
    let got = aimed(&mut on_h, "search_design", json!({"query": "aardvark"}));
    assert_eq!(got["hits"][0]["node_id"], format!("{a}:req:alpha"), "{got}");

    // A claim lands on A and is released on A; the hub holds none.
    let claim = json!({"contributor_id": CALLER, "seed_id": "req:alpha", "depth": 0,
                       "note": "aimed through the hub"});
    aimed(&mut on_h, "claim_region", claim);
    let on_a_claims = ok(
        &on_a.call("claim_report", json!({}), Some(&[a.as_str()]), None),
        "claim_report on A",
    );
    assert!(
        on_a_claims.to_string().contains("aimed through the hub"),
        "the claim is A's: {on_a_claims}"
    );
    let on_h_claims = ok(
        &on_h.call("claim_report", json!({}), Some(&readable), None),
        "claim_report on H",
    );
    assert!(
        !on_h_claims.to_string().contains("aimed through the hub"),
        "and not the hub's: {on_h_claims}"
    );
    aimed(
        &mut on_h,
        "release_claim",
        json!({"contributor_id": CALLER, "seed_id": "req:alpha"}),
    );
    let after = ok(
        &on_a.call("claim_report", json!({}), Some(&[a.as_str()]), None),
        "claim_report on A after the release",
    );
    assert!(
        !after.to_string().contains("aimed through the hub"),
        "released on A: {after}"
    );

    // `member` naming the hub runs there alone, with no fan-out.
    let reply = on_h.call(
        "scan_nodes",
        json!({"node_type": "Requirement", "brief": true, "member": h}),
        Some(&readable),
        Some(&view),
    );
    let got = ok(&reply, "scan_nodes aimed at H");
    ran_alone_on(&got, &h, "scan_nodes aimed at the hub");
    let mut ids = Vec::new();
    strings(&got, &mut ids);
    assert!(ids.iter().any(|s| s == "req:hub"), "{got}");
    assert!(!ids.iter().any(|s| s.contains("req:alpha")), "{got}");
}

#[test]
fn every_id_in_a_routed_reply_carries_its_designs_prefix() {
    let (root, h, a, b) = hub_and_members("member-prefix");
    let server = serve(&root, Some(GATEWAY));
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];

    // Euclid's reductio on A: an assumption taken up and discharged.
    let mut on_a = Session::open(server.port, &a);
    let mut write = |tool: &str, args: Value| {
        ok(
            &on_a.call(tool, args.clone(), Some(&[a.as_str()]), None),
            &format!("{tool} {args} on A"),
        );
    };
    let mut held: Vec<String> = vec!["req:alpha".into(), "who:tester".into()];
    for id in ["con:def-finite", "con:def-prime"] {
        write(
            "add_constraint",
            json!({"id": id, "name": id, "statement": "A definition.",
                   "statement_kind": "definition"}),
        );
        held.push(id.into());
    }
    for id in [
        "req:lemma-euclid",
        "req:suppose-finite",
        "req:contradiction",
        "req:theorem",
    ] {
        write(
            "add_requirement",
            json!({"id": id, "name": id, "statement": "A statement."}),
        );
        held.push(id.into());
    }
    write(
        "add_inference",
        json!({"id": "inf:euclid", "rule": "all", "premises": ["con:def-prime"],
               "conclusion": "req:lemma-euclid"}),
    );
    write(
        "add_inference",
        json!({"id": "inf:absurd", "rule": "all",
               "premises": ["req:lemma-euclid", "con:def-finite"],
               "assumed": ["req:suppose-finite"], "conclusion": "req:contradiction"}),
    );
    write(
        "add_inference",
        json!({"id": "inf:theorem", "rule": "all",
               "premises": ["req:contradiction", "con:def-finite"],
               "discharges": ["req:suppose-finite"], "conclusion": "req:theorem",
               "rule_name": "proof by contradiction"}),
    );
    held.extend(["inf:euclid", "inf:absurd", "inf:theorem"].map(String::from));

    let mut on_h = Session::open(server.port, &h);
    let bare_ids = |got: &Value| -> Vec<String> {
        let mut all = Vec::new();
        strings(got, &mut all);
        let mut bare: Vec<String> = all.into_iter().filter(|s| held.contains(s)).collect();
        bare.sort();
        bare.dedup();
        bare
    };

    // Routed by the claim's prefix, and aimed with `member` and no claim.
    for (what, args) in [
        (
            "reasoning_report on A:req:theorem",
            json!({"claim": format!("{a}:req:theorem")}),
        ),
        (
            "reasoning_report on A:req:contradiction",
            json!({"claim": format!("{a}:req:contradiction")}),
        ),
        ("reasoning_report aimed at A", json!({"member": a})),
        (
            "get_node A:inf:absurd",
            json!({"id": format!("{a}:inf:absurd")}),
        ),
    ] {
        let tool = what.split(' ').next().unwrap();
        let got = ok(&on_h.call(tool, args, Some(&readable), Some(&view)), what);
        let bare = bare_ids(&got);
        assert!(
            bare.is_empty(),
            "{what}: every id A holds comes back as \"{a}:<id>\", so it can be passed back; \
             these came back bare: {bare:?}\n{got:#}"
        );
    }

    // A refusal gives back the ids the call passed, and the ids A holds, with
    // their prefix: a write naming a premise A does not hold.
    let reply = on_h.call(
        "add_inference",
        json!({"id": format!("{a}:inf:bad"), "rule": "all",
               "premises": [format!("{a}:req:nowhere")],
               "conclusion": format!("{a}:req:theorem")}),
        Some(&readable),
        Some(&view),
    );
    let why = refused(&reply, "add_inference on a premise A does not hold");
    assert!(
        why.contains(&format!("{a}:req:nowhere")) && !why.contains(" req:nowhere"),
        "the refusal names the premise as the caller passed it: {why}"
    );
    let reply = on_h.call(
        "reasoning_report",
        json!({"claim": format!("{a}:req:nowhere")}),
        Some(&readable),
        Some(&view),
    );
    let why = refused(&reply, "reasoning_report on a claim A does not hold");
    assert!(
        why.contains(&format!("{a}:req:nowhere")),
        "the refusal names the claim as the caller passed it: {why}"
    );
}

#[test]
fn a_member_id_given_to_a_tool_the_view_does_not_serve_lands_in_that_member() {
    let (root, h, a, b) = hub_and_members("member-release");
    let server = serve(&root, Some(GATEWAY));
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];
    let mut on_h = Session::open(server.port, &h);

    let got = ok(
        &on_h.call(
            "add_release",
            json!({"id": format!("{a}:rel:v1"), "name": "v1", "version": "1.0.0"}),
            Some(&readable),
            Some(&view),
        ),
        "add_release naming A:rel:v1 through H",
    );
    ran_alone_on(&got, &a, "add_release routed by its id");

    let mut on_a = Session::open(server.port, &a);
    let in_a = ok(
        &on_a.call(
            "get_node",
            json!({"id": "rel:v1"}),
            Some(&[a.as_str()]),
            None,
        ),
        "get_node rel:v1 on A",
    );
    assert_eq!(
        in_a["node"]["node_id"], "rel:v1",
        "the release is A's: {in_a}"
    );
    for id in ["rel:v1".to_string(), format!("{a}:rel:v1")] {
        let in_h = ok(
            &on_h.call("get_node", json!({"id": id}), Some(&readable), None),
            "get_node on H",
        );
        assert!(
            in_h["node"].is_null(),
            "the hub holds no release by the id {id}: {in_h}"
        );
    }
}

#[test]
fn member_is_refused_where_it_cannot_be_honoured_and_nothing_runs() {
    let (root, h, a, b) = hub_and_members("member-refused");
    let server = serve(&root, Some(GATEWAY));
    let readable = [h.as_str(), a.as_str(), b.as_str()];
    let view = [a.as_str(), b.as_str()];
    let mut on_h = Session::open(server.port, &h);

    // A design that is not in the view.
    let why = refused(
        &on_h.call(
            "loop_status",
            json!({"member": "nosuchdesign"}),
            Some(&readable),
            Some(&view),
        ),
        "member naming no design of the view",
    );
    assert!(
        why.contains("nosuchdesign") && why.contains("member"),
        "{why}"
    );

    // `member` beside an id of another member: two designs, nothing written.
    let why = refused(
        &on_h.call(
            "add_requirement",
            json!({"id": format!("{b}:req:two"), "name": "two", "statement": "s",
                   "member": a}),
            Some(&readable),
            Some(&view),
        ),
        "member A beside an id of B",
    );
    assert!(why.contains("nothing ran"), "{why}");
    let mut on_b = Session::open(server.port, &b);
    let in_b = ok(
        &on_b.call(
            "get_node",
            json!({"id": "req:two"}),
            Some(&[b.as_str()]),
            None,
        ),
        "get_node on B",
    );
    assert!(in_b["node"].is_null(), "nothing was written in B: {in_b}");

    // A member outside the readable set: refused naming the id and the key.
    let why = refused(
        &on_h.call(
            "loop_status",
            json!({"member": b}),
            Some(&[h.as_str(), a.as_str()]),
            Some(&view),
        ),
        "member B, outside the readable set",
    );
    assert!(
        why.contains(&b) && why.contains("readable_graph_ids"),
        "{why}"
    );

    // No view on the call: `member` is refused by name.
    let reply = on_h.call("loop_status", json!({"member": a}), Some(&readable), None);
    let why = refused(&reply, "member with no view named");
    assert!(
        why.contains("member") && why.contains("hub"),
        "the refusal says `member` is honoured only through a hub's address: {why}"
    );

    // A server that serves no view: `member` naming a member is refused.
    drop(server);
    let untrusted = serve(&root, None);
    let mut on_h = Session::open(untrusted.port, &h);
    let why = refused(
        &on_h.call(
            "loop_status",
            json!({"member": a}),
            Some(&readable),
            Some(&view),
        ),
        "member on a server that serves no view",
    );
    assert!(why.contains("serves no hub's view"), "{why}");
}
