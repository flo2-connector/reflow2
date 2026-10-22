//! ONE COMPOSED CHECK RUNS OVER A HUB AND SEVERAL OF ITS DESIGNS —
//! `req:one-composed-check-runs-over-a-hub-and-several-of-its-designs`
//! (phase 7, step 3).
//!
//! WHAT FAILED. hxm_program's hub pins fourteen member designs by their
//! committed exports and is reached only through the `--call` door. An effort
//! there spans three to six designs, and `compose_and_analyse` took ONE other
//! design: measured 2026-10-21 on 0.90.0, `designs` and `tracked_designs` were
//! refused by its schema, and `other_graph_id` was refused off a registry. So
//! the hub ran it pairwise and stitched the replies by hand, and the seam
//! between two members was never analysed unless one of them was the hub.
//!
//! THE CLASS, pinned here across every pin state (an export present, missing,
//! a pin at an address, one with nowhere to read it, a file that is not a
//! design, an export of another design), across one, two and several other
//! designs, and across the node types a finding names:
//!
//! 1. every finding is attributed to the designs its ids belong to —
//!    `by_design`, `by_pair` ('a×b'), `across`, `of_the_whole` — and every id
//!    it names opens with get_node in the design it is attributed to;
//! 2. every pinned design not composed is named as the tracked search names
//!    it, with the same `state`;
//! 3. with one other design it finds what the one-other-design composition
//!    finds, bucket for bucket;
//! 4. nothing is written: this design's export content hash and every
//!    member's export bytes are the same afterwards;
//! 5. the one-other-design call keeps its exact reply, refusals included,
//!    across its old argument shapes;
//! 6. the reply is bounded, and its counts survive the bound.
//!
//! Driven through `call_tool`, where a session and the `--call` door reach the
//! tool (the door's own run is
//! `tests/a_local_hub_composes_its_pinned_designs_through_the_door.rs`).

use std::collections::BTreeMap;

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "one-composed-check-over-a-hub".to_string();
        cfg.client_info.version = "test".to_string();
        cfg
    }
}

type Client = rmcp::service::RunningService<rmcp::RoleClient, TestClient>;

async fn connect(service: ReflowService) -> Client {
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = service.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    TestClient
        .serve((client_rx, client_tx))
        .await
        .expect("the in-process handshake")
}

/// The reply of one call: `Ok(structured)` or `Err(the refusal's words)`.
async fn answer(c: &Client, tool: &str, args: Value) -> Result<Value, String> {
    let Value::Object(arguments) = args else {
        panic!("arguments for {tool} must be an object");
    };
    match c
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
    {
        Ok(r) if r.is_error == Some(true) => Err(r
            .content
            .iter()
            .filter_map(|b| b.as_text().map(|t| t.text.clone()))
            .collect::<Vec<_>>()
            .join("\n")),
        Ok(r) => Ok(r.structured_content.unwrap_or(Value::Null)),
        Err(e) => Err(e.to_string()),
    }
}

async fn call(c: &Client, tool: &str, args: Value) -> Value {
    answer(c, tool, args.clone())
        .await
        .unwrap_or_else(|e| panic!("{tool} {args} refused: {e}"))
}

async fn refused(c: &Client, tool: &str, args: Value) -> String {
    match answer(c, tool, args.clone()).await {
        Err(why) => why,
        Ok(v) => panic!("{tool} {args} should have been refused, answered {v}"),
    }
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-composed-check-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A member design's committed export, holding one record of each node type
/// a finding commonly names: a Project, a Requirement nothing satisfies, a
/// Capability allocated nowhere, a Component, a Decision, and `cmp:store`, an
/// id every member and the hub hold.
async fn member_export(dir: &std::path::Path, graph_id: &str) -> String {
    let m = connect(ReflowService::in_memory_as(graph_id).expect("member")).await;
    call(
        &m,
        "add_project",
        json!({"id": format!("proj:{graph_id}"), "name": graph_id}),
    )
    .await;
    call(
        &m,
        "add_requirement",
        json!({"id": "req:feed", "name": format!("{graph_id} feed"),
               "statement": format!("{graph_id} is fed from the stream.")}),
    )
    .await;
    call(
        &m,
        "add_capability",
        json!({"id": "cap:serve", "name": format!("{graph_id} serves"),
               "description": "serves the stream"}),
    )
    .await;
    call(
        &m,
        "add_component",
        json!({"id": "cmp:store", "name": format!("{graph_id} store"),
               "description": "the member's own store"}),
    )
    .await;
    call(
        &m,
        "add_decision",
        json!({"id": "dec:route", "name": format!("{graph_id} route"),
               "decision": "Read the stream through the role."}),
    )
    .await;
    let path = dir.join(format!("{graph_id}.json"));
    call(&m, "export_graph", json!({"path": path.to_string_lossy()})).await;
    path.to_string_lossy().into_owned()
}

struct Hub {
    c: Client,
    ansible: String,
    tiled: String,
    other: String,
}

/// A hub that pins one design of every kind a pin can be — two readable
/// members among them — plus a code dependency that names no design.
async fn hub(name: &str) -> Hub {
    let dir = scratch(name);
    let ansible = member_export(&dir, "ansible").await;
    let tiled = member_export(&dir, "tiled").await;
    let other = member_export(&dir, "other").await;
    let garbled = dir.join("garbled.json");
    std::fs::write(&garbled, "this is not a design").expect("garbled");
    let c = connect(ReflowService::in_memory_as("hub").expect("hub")).await;
    call(
        &c,
        "add_requirement",
        json!({"id": "req:pdf-firewall", "name": "PDF firewall fix",
               "statement": "The PDF firewall admits the stream."}),
    )
    .await;
    call(
        &c,
        "add_component",
        json!({"id": "cmp:store", "name": "hub store", "description": "the hub's own"}),
    )
    .await;
    let pin = |id: &str, extra: Value| {
        let mut p = json!({"id": format!("dep:{id}"), "name": id, "version": "abc123",
                           "source": format!("https://example.org/{id}"), "graph_id": id});
        for (k, v) in extra.as_object().unwrap() {
            p[k] = v.clone();
        }
        p
    };
    for p in [
        pin("ansible", json!({"design_export": ansible})),
        pin("tiled", json!({"design_export": tiled})),
        pin(
            "gone",
            json!({"design_export": dir.join("gone.json").to_string_lossy()}),
        ),
        pin(
            "hosted",
            json!({"design_address": "http://127.0.0.1:9/g/hosted/mcp"}),
        ),
        pin("nowhere", json!({})),
        pin(
            "garbled",
            json!({"design_export": garbled.to_string_lossy()}),
        ),
        pin("wrong", json!({"design_export": other})),
        json!({"id": "dep:serde", "name": "serde", "source": "crates.io", "version": "1"}),
    ] {
        call(&c, "external_dependency", p).await;
    }
    Hub {
        c,
        ansible,
        tiled,
        other,
    }
}

/// Every finding in a composed reply, with the designs it was filed under.
fn filed(reply: &Value) -> Vec<(Vec<String>, Value)> {
    let mut out = Vec::new();
    for (design, list) in reply["by_design"].as_object().expect("by_design") {
        for f in list.as_array().expect("a list") {
            out.push((vec![design.clone()], f.clone()));
        }
    }
    for (pair, list) in reply["by_pair"].as_object().expect("by_pair") {
        let designs: Vec<String> = pair.split('×').map(str::to_string).collect();
        assert_eq!(designs.len(), 2, "a pair key names two designs: {pair}");
        for f in list.as_array().expect("a list") {
            out.push((designs.clone(), f.clone()));
        }
    }
    for f in reply["across"].as_array().expect("across") {
        let designs: Vec<String> = f["designs"]
            .as_array()
            .expect("an across finding names its designs")
            .iter()
            .map(|d| d.as_str().unwrap().to_string())
            .collect();
        assert!(designs.len() >= 3, "across is three or more: {f}");
        out.push((designs, f.clone()));
    }
    for f in reply["of_the_whole"].as_array().expect("of_the_whole") {
        out.push((Vec::new(), f.clone()));
    }
    out
}

/// The design an id in a composed reply belongs to: a composed design's ids
/// carry `<graph_id>:`; this design's do not.
fn design_of<'a>(id: &str, this: &'a str, others: &'a [String]) -> &'a str {
    others
        .iter()
        .find(|d| id.starts_with(&format!("{d}:")))
        .map_or(this, String::as_str)
}

/// THE ATTRIBUTION CLASS, asked of one reply: each finding is filed under
/// exactly the designs its ids belong to, every count agrees with its list,
/// and every id opens with get_node in its own design.
async fn assert_attributed(c: &Client, reply: &Value) {
    let this = reply["this_design"].as_str().expect("this_design");
    let others: Vec<String> = reply["designs_composed"]
        .as_array()
        .expect("designs_composed")
        .iter()
        .skip(1)
        .map(|d| d.as_str().unwrap().to_string())
        .collect();
    let all = filed(reply);
    assert_eq!(
        reply["counts"]["total"].as_u64(),
        Some(all.len() as u64),
        "the total counts every finding filed: {reply}"
    );
    for (designs, f) in &all {
        let ids: Vec<&str> = f["affected_ids"]
            .as_array()
            .expect("affected_ids")
            .iter()
            .map(|i| i.as_str().unwrap())
            .collect();
        let mut named: Vec<&str> = ids.iter().map(|i| design_of(i, this, &others)).collect();
        named.sort_unstable();
        named.dedup();
        let mut want: Vec<&str> = designs.iter().map(String::as_str).collect();
        want.sort_unstable();
        assert_eq!(named, want, "filed under the designs it names: {f}");
        for id in ids {
            let opened = call(c, "get_node", json!({"id": id})).await;
            assert!(
                !opened["node"].is_null(),
                "{id} opens in its own design ({f}): {opened}"
            );
        }
    }
    for (key, n) in reply["counts"]["by_pair"].as_object().expect("pair counts") {
        assert_eq!(
            reply["by_pair"][key].as_array().map(Vec::len),
            n.as_u64().map(|n| n as usize),
            "{key}: {reply}"
        );
    }
}

#[tokio::test]
async fn a_composed_check_over_every_pinned_design_attributes_each_finding_and_names_each_it_could_not_read()
 {
    let h = hub("tracked").await;
    let composed = call(
        &h.c,
        "compose_and_analyse",
        json!({"tracked_designs": true}),
    )
    .await;
    assert_eq!(
        composed["designs_composed"],
        json!(["hub", "ansible", "tiled"]),
        "this design first, then every readable pin in order: {composed}"
    );
    assert_eq!(composed["read_from"]["ansible"], json!(h.ansible));
    assert_eq!(composed["read_from"]["tiled"], json!(h.tiled));
    assert_attributed(&h.c, &composed).await;
    // Each member's own findings are its own, and the seam between the two
    // members is there although neither is the design calling.
    for d in ["hub", "ansible", "tiled"] {
        assert!(
            composed["counts"]["by_design"][d].as_u64().unwrap_or(0) > 0,
            "{d} has findings of its own: {composed}"
        );
    }
    assert!(
        composed["by_pair"]["ansible×tiled"]
            .as_array()
            .is_some_and(|l| !l.is_empty()),
        "a seam between two members, neither of them the hub: {composed}"
    );
    assert_eq!(composed["counts"]["pairs_composed"], json!(3), "{composed}");

    // EVERY PINNED DESIGN NOT COMPOSED IS NAMED AS THE TRACKED SEARCH NAMES IT.
    let searched = call(
        &h.c,
        "search_design",
        json!({"query": "stream", "tracked_designs": true}),
    )
    .await;
    let states = |list: &Value| -> BTreeMap<String, String> {
        list.as_array()
            .expect("a list")
            .iter()
            .map(|n| {
                (
                    n["design"].as_str().unwrap().to_string(),
                    n["state"].as_str().unwrap().to_string(),
                )
            })
            .collect()
    };
    assert_eq!(
        states(&composed["not_composed"]),
        states(&searched["not_searched"]),
        "the same designs, in the same states: {composed}"
    );
    assert_eq!(
        states(&composed["not_composed"]),
        [
            ("garbled", "unreadable"),
            ("gone", "missing"),
            ("hosted", "not_reached"),
            ("nowhere", "nowhere"),
            ("wrong", "unreadable"),
        ]
        .into_iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect::<BTreeMap<_, _>>()
    );
    for n in composed["not_composed"].as_array().unwrap() {
        assert!(
            n["why"].as_str().is_some_and(|w| w.len() > 20),
            "says why: {n}"
        );
    }
    assert!(
        composed["composed_note"]
            .as_str()
            .is_some_and(|n| n.contains("5 design(s) asked for were NOT composed")),
        "{composed}"
    );
}

#[tokio::test]
async fn one_two_and_several_designs_are_composed_alike_and_one_finds_what_the_pairwise_check_finds()
 {
    let h = hub("one-two-several").await;
    // ONE other design: the same findings as the one-other-design call, filed
    // ours -> this design (or the whole), theirs -> the member, seam -> the pair.
    let one = call(
        &h.c,
        "compose_and_analyse",
        json!({"other_graph_ids": ["ansible"]}),
    )
    .await;
    assert_attributed(&h.c, &one).await;
    let export: Value =
        serde_json::from_str(&std::fs::read_to_string(&h.ansible).unwrap()).unwrap();
    let pairwise = call(
        &h.c,
        "compose_and_analyse",
        json!({"design": export, "namespace": "ansible"}),
    )
    .await;
    let titles = |list: &Value| -> Vec<String> {
        let mut t: Vec<String> = list
            .as_array()
            .map(|l| {
                l.iter()
                    .map(|f| format!("{} {}", f["source"], f["title"]))
                    .collect()
            })
            .unwrap_or_default();
        t.sort();
        t
    };
    let mut ours = titles(&one["by_design"]["hub"]);
    ours.extend(titles(&one["of_the_whole"]));
    ours.sort();
    assert_eq!(ours, titles(&pairwise["our_findings"]), "ours: {one}");
    assert_eq!(
        titles(&one["by_design"]["ansible"]),
        titles(&pairwise["their_findings"]),
        "theirs: {one}"
    );
    assert_eq!(
        titles(&one["by_pair"]["hub×ansible"]),
        titles(&pairwise["seam_findings"]),
        "the seam: {one}"
    );
    assert_eq!(one["counts"]["pairs_composed"], json!(1));

    // TWO and SEVERAL: the same class holds, and naming the readable pins is
    // the tracked composition without the ones it could not read.
    let two = call(
        &h.c,
        "compose_and_analyse",
        json!({"other_graph_ids": ["ansible", "tiled"]}),
    )
    .await;
    assert_attributed(&h.c, &two).await;
    assert_eq!(two["not_composed"], json!([]), "{two}");
    let tracked = call(
        &h.c,
        "compose_and_analyse",
        json!({"tracked_designs": true}),
    )
    .await;
    for key in [
        "designs_composed",
        "counts",
        "by_design",
        "by_pair",
        "across",
    ] {
        let (mut a, mut b) = (two[key].clone(), tracked[key].clone());
        // The pairs with a pin that could not be read are `not_checked` in
        // the tracked reply only: it is the one that asked for them.
        if key == "counts" {
            a["link_status"]["not_checked"] = json!(null);
            b["link_status"]["not_checked"] = json!(null);
        }
        assert_eq!(a, b, "{key}");
    }
    // Naming a pin that cannot be read: composed without it, and named.
    let with_missing = call(
        &h.c,
        "compose_and_analyse",
        json!({"other_graph_ids": ["ansible", "gone"]}),
    )
    .await;
    assert_eq!(with_missing["designs_composed"], json!(["hub", "ansible"]));
    assert_eq!(with_missing["not_composed"][0]["state"], json!("missing"));

    // NONE: a design that pins nothing composes itself alone, and says so.
    let alone = connect(ReflowService::in_memory_as("alone").expect("alone")).await;
    call(
        &alone,
        "add_requirement",
        json!({"id": "req:x", "name": "x", "statement": "x."}),
    )
    .await;
    let solo = call(
        &alone,
        "compose_and_analyse",
        json!({"tracked_designs": true}),
    )
    .await;
    assert_eq!(solo["designs_composed"], json!(["alone"]), "{solo}");
    assert!(
        solo["tracked_note"]
            .as_str()
            .is_some_and(|n| n.contains("pins no other design")),
        "{solo}"
    );
    assert_attributed(&alone, &solo).await;
}

#[tokio::test]
async fn composing_writes_nothing_to_this_design_or_any_export() {
    let h = hub("writes-nothing").await;
    let read = |p: &str| std::fs::read(p).expect("export");
    let before = (read(&h.ansible), read(&h.tiled), read(&h.other));
    let hash = |v: Value| v["content_hash"].as_str().unwrap().to_string();
    let hub_before = hash(call(&h.c, "export_graph", json!({})).await);
    for args in [
        json!({"tracked_designs": true}),
        json!({"other_graph_ids": ["tiled", "ansible"]}),
        json!({"other_graph_ids": ["ansible"], "budget_chars": 2000}),
    ] {
        call(&h.c, "compose_and_analyse", args).await;
    }
    assert_eq!(
        hash(call(&h.c, "export_graph", json!({})).await),
        hub_before,
        "this design's export content hash is the same afterwards"
    );
    assert_eq!(
        (read(&h.ansible), read(&h.tiled), read(&h.other)),
        before,
        "no member's export is written"
    );
}

#[tokio::test]
async fn the_one_other_design_call_keeps_its_exact_reply_across_its_old_shapes() {
    let h = hub("old-shapes").await;
    let export: Value =
        serde_json::from_str(&std::fs::read_to_string(&h.ansible).unwrap()).unwrap();
    // The success: exactly the core's report, under exactly its old keys,
    // whatever spelling of "not several" the call uses.
    let doc: reflow2_core::GraphExport = serde_json::from_value(export.clone()).unwrap();
    let core = {
        let g = reflow2_core::DesignGraph::open_in_memory_as("hub").unwrap();
        // The hub's own content, as the service holds it.
        let held: reflow2_core::GraphExport =
            serde_json::from_value(call(&h.c, "export_graph", json!({})).await).unwrap();
        let mut g = g;
        g.import_graph(&held).unwrap();
        serde_json::to_value(g.compose_and_analyse(&doc, "ansible").unwrap()).unwrap()
    };
    for args in [
        json!({"design": export, "namespace": "ansible"}),
        json!({"design": export, "namespace": "ansible", "tracked_designs": false}),
        json!({"design": export, "namespace": "ansible", "tracked_designs": null,
               "other_graph_ids": null}),
    ] {
        let got = call(&h.c, "compose_and_analyse", args.clone()).await;
        assert_eq!(
            got,
            core,
            "the old reply, exactly, for {}",
            args.as_object().unwrap().len()
        );
        let mut keys: Vec<&str> = got
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "candidates",
                "computed_links",
                "declared_links",
                "imported_edges",
                "imported_graph_id",
                "imported_nodes",
                "link_status",
                "links_note",
                "namespace",
                "our_findings",
                "seam_findings",
                "set_aside",
                "skipped_edges",
                "their_findings"
            ],
            "the old keys, and only the link fields phase 7 step 4 added \
             (req:designs-are-shown-where-they-already-touch-before-anyone-declares-it)"
        );
    }
    // The old refusals, word for word.
    for (args, words) in [
        (
            json!({"design": export, "other_graph_id": "x"}),
            "pass design OR other_graph_id, not both",
        ),
        (
            json!({}),
            "nothing to compose: pass design (an export document) or other_graph_id",
        ),
        (
            json!({"design": export}),
            "a namespace is required with an inline design",
        ),
        (
            json!({"other_graph_id": "ansible"}),
            "is served only by a registry server",
        ),
        (json!({"tracked_designs": false}), "nothing to compose"),
    ] {
        let why = refused(&h.c, "compose_and_analyse", args.clone()).await;
        assert!(why.contains(words), "{args}: {why}");
    }
}

#[tokio::test]
async fn several_designs_are_refused_in_any_shape_that_cannot_say_which_and_nothing_runs() {
    let h = hub("refusals").await;
    let export: Value =
        serde_json::from_str(&std::fs::read_to_string(&h.ansible).unwrap()).unwrap();
    for (args, words) in [
        (
            json!({"tracked_designs": true, "design": export, "namespace": "x"}),
            "`design` composes ONE other design",
        ),
        (
            json!({"other_graph_ids": ["ansible"], "other_graph_id": "tiled"}),
            "`other_graph_id` composes ONE other design",
        ),
        (
            json!({"other_graph_ids": ["ansible"], "namespace": "x"}),
            "`namespace` composes ONE other design",
        ),
        (
            json!({"other_graph_ids": ["ansible"], "tracked_designs": true}),
            "not both",
        ),
        (json!({"other_graph_ids": []}), "names no design"),
        (json!({"other_graph_ids": ["ansible", "ansible"]}), "twice"),
        (json!({"other_graph_ids": ["hub"]}), "this design itself"),
        (json!({"other_graph_ids": [" "]}), "empty graph_id"),
    ] {
        let why = refused(&h.c, "compose_and_analyse", args.clone()).await;
        assert!(
            why.contains(words) && why.contains("nothing ran"),
            "{args}: {why}"
        );
    }
    let why = refused(
        &h.c,
        "compose_and_analyse",
        json!({"other_graph_ids": ["elsewhere"]}),
    )
    .await;
    assert!(
        why.contains("does not pin") && why.contains("ansible") && why.contains("tiled"),
        "a design not pinned is refused naming the pinned ones: {why}"
    );
}

#[tokio::test]
async fn the_composed_reply_is_bounded_and_its_counts_survive() {
    let h = hub("bounded").await;
    let full = call(
        &h.c,
        "compose_and_analyse",
        json!({"tracked_designs": true, "budget_chars": 1_000_000}),
    )
    .await;
    // 6,000 since phase 7 step 4 (4,500 before): every pair's `link_status`
    // is never sampled, so the whole a bounded reply keeps grew by its size.
    let budget = 6_000;
    let bounded = call(
        &h.c,
        "compose_and_analyse",
        json!({"tracked_designs": true, "budget_chars": budget}),
    )
    .await;
    let size = serde_json::to_string(&bounded).unwrap().len();
    assert!(
        full.to_string().len() > budget,
        "the fixture overflows the budget, so this asks something"
    );
    assert!(size <= budget, "{size} > {budget}: {bounded}");
    assert_eq!(bounded["counts"], full["counts"], "every count survives");
    // What was ASKED stays whole: which designs were composed, and which not.
    assert_eq!(bounded["designs_composed"], full["designs_composed"]);
    assert_eq!(bounded["read_from"], full["read_from"]);
    assert_eq!(
        bounded["link_status"], full["link_status"],
        "every pair's status survives the bound"
    );
    assert_eq!(
        bounded["not_composed"].as_array().map(Vec::len),
        Some(5),
        "not_composed is never sampled: {bounded}"
    );
    let shortened = bounded["budget"]["lists_shortened"]
        .as_array()
        .unwrap_or_else(|| panic!("the finding lists were sampled, and say so: {bounded}"));
    assert!(!shortened.is_empty(), "{bounded}");
    for l in shortened {
        let path = l["path"].as_str().unwrap();
        assert!(
            path.starts_with("by_design.")
                || path.starts_with("by_pair.")
                || path == "across"
                || path == "of_the_whole"
                || path == "declared_links"
                || path == "computed_links"
                || path == "candidates",
            "only finding and link lists are sampled, never what was asked or a pair's \
             status: {path}"
        );
    }
    // Unasked, the default bound is the 30,000 the gate holds every reply to.
    let unasked = call(
        &h.c,
        "compose_and_analyse",
        json!({"tracked_designs": true}),
    )
    .await;
    assert_eq!(unasked["counts"], full["counts"]);
}

#[tokio::test]
async fn the_served_text_says_when_to_compose_over_a_hub() {
    let c = connect(ReflowService::in_memory_as("text").expect("text")).await;
    let tools = c.list_all_tools().await.expect("tools/list");
    let compose = tools
        .iter()
        .find(|t| t.name == "compose_and_analyse")
        .expect("served");
    let description = compose.description.as_deref().unwrap_or_default();
    for words in [
        "other_graph_ids",
        "tracked_designs",
        "by_pair",
        "not_composed",
    ] {
        assert!(description.contains(words), "{words}: {description}");
    }
    assert!(description.chars().count() <= 1500, "{}", description.len());
    let skill = call(&c, "get_skill", json!({"name": "hub"}))
        .await
        .to_string();
    assert!(
        skill.contains("compose_and_analyse") && skill.contains("tracked_designs"),
        "the hub skill says when to run the composed check over a hub"
    );
}
