//! ONE SEARCH REACHES EVERY DESIGN A HUB TRACKS, and a hit in a pinned design
//! opens — `req:one-search-reaches-every-design-a-hub-tracks`,
//! `cap:search-design-reaches-the-designs-a-hub-tracks` (phase 7, step 2).
//!
//! WHAT FAILED. A work hub pins fourteen member designs by their
//! committed exports and is reached only through the `--call` door
//! (organisation policy blocks MCP there). Measured 2026-10-21 on 0.89.0:
//! `reflow2 read search_design '{"query":"x","tracked_designs":true}'` was
//! refused by the schema (`tracked_designs` is not a parameter), and
//! `get_node` with `member` was refused because `member` is honoured only
//! through a hub's address. Since 0.90.0+ (#785) a search names the pinned
//! designs it did NOT search; nothing searched them, and nothing could open a
//! node held only in one.
//!
//! THE CLASS, pinned here across every pin state — an export present, an
//! export missing, a pin at an address, a pin with nowhere to read it, a file
//! that is not a design, an export of a different design — and across node
//! types:
//!
//! 1. a tracked search returns, ranked together, this design's hits and every
//!    readable pinned design's, each naming its `design`, a pinned design's
//!    ids carrying its graph_id prefix; every pinned design NOT searched is
//!    named with a `state` and why;
//! 2. a read aimed at a pinned design (a prefixed id, or `member`) answers from
//!    that design's export; a write aimed at one is refused;
//! 3. nothing is written to any export, and without `tracked_designs` the
//!    reply is what it was.
//!
//! Driven through `call_tool`, where a session and the `--call` door reach the
//! tool (the door's own run is `tests/a_local_hub_searches_its_pinned_designs_through_the_door.rs`).

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "one-search-reaches-every-design-a-hub-tracks".to_string();
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
        "reflow2-tracked-search-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// A member design's committed export: a Component, a Requirement and a
/// Decision that carry the word, and `req:shared`, an id the hub also holds.
async fn member_export(dir: &std::path::Path, graph_id: &str) -> String {
    let m = ReflowService::in_memory_as(graph_id).expect("member");
    m.add_component(Parameters(
        serde_json::from_value(
            json!({"id": "cmp:role-datastream", "name": "datastream role",
            "description": "deploys the datastream servers"}),
        )
        .unwrap(),
    ))
    .await
    .expect("component");
    m.add_requirement(Parameters(
        serde_json::from_value(
            json!({"id": "req:datastream-feed", "name": "datastream feed",
            "statement": "datastream is fed from Storage streaming."}),
        )
        .unwrap(),
    ))
    .await
    .expect("requirement");
    m.add_decision(Parameters(
        serde_json::from_value(
            json!({"id": "dec:datastream-route", "name": "datastream route",
            "decision": "datastream reads the stream through the role."}),
        )
        .unwrap(),
    ))
    .await
    .expect("decision");
    m.add_requirement(Parameters(
        serde_json::from_value(json!({"id": "req:shared", "name": "member's shared",
            "statement": "The member's own record under an id the hub also uses."}))
        .unwrap(),
    ))
    .await
    .expect("shared");
    let path = dir.join(format!("{graph_id}.json"));
    m.export_graph(Parameters(
        serde_json::from_value(json!({"path": path.to_string_lossy()})).unwrap(),
    ))
    .await
    .expect("export");
    path.to_string_lossy().into_owned()
}

struct Hub {
    c: Client,
    deploy: String,
    other: String,
}

/// A hub that pins one design of every kind a pin can be, plus a code
/// dependency that names no design.
async fn hub(name: &str) -> Hub {
    let dir = scratch(name);
    let deploy = member_export(&dir, "deploy").await;
    let other = member_export(&dir, "other").await;
    let garbled = dir.join("garbled.json");
    std::fs::write(&garbled, "this is not a design").expect("garbled");
    let c = connect(ReflowService::in_memory_as("hub").expect("hub")).await;
    call(&c, "add_project", json!({"id": "proj:hub", "name": "Hub"})).await;
    call(
        &c,
        "add_requirement",
        json!({"id": "req:instrument-firewall", "name": "instrument firewall fix",
               "statement": "The instrument firewall admits the datastream stream."}),
    )
    .await;
    call(
        &c,
        "add_requirement",
        json!({"id": "req:shared", "name": "hub's shared",
               "statement": "The hub's own record under an id a member also uses."}),
    )
    .await;
    let pin = |id: &str, graph_id: &str, extra: Value| {
        let mut p = json!({"id": format!("dep:{id}"), "name": id, "version": "abc123",
                           "source": format!("https://example.org/{id}"), "graph_id": graph_id});
        for (k, v) in extra.as_object().unwrap() {
            p[k] = v.clone();
        }
        p
    };
    for p in [
        pin("deploy", "deploy", json!({"design_export": deploy})),
        pin(
            "storage",
            "storage",
            json!({"design_export": dir.join("gone.json").to_string_lossy()}),
        ),
        pin(
            "hosted",
            "hosted",
            json!({"design_address": "http://127.0.0.1:9/g/hosted/mcp"}),
        ),
        pin("nowhere", "nowhere", json!({})),
        pin(
            "garbled",
            "garbled",
            json!({"design_export": garbled.to_string_lossy()}),
        ),
        pin("wrong", "wrong", json!({"design_export": other})),
        json!({"id": "dep:serde", "name": "serde", "source": "crates.io", "version": "1"}),
    ] {
        call(&c, "external_dependency", p).await;
    }
    Hub { c, deploy, other }
}

fn not_searched<'a>(reply: &'a Value, design: &str) -> &'a Value {
    reply["not_searched"]
        .as_array()
        .and_then(|a| a.iter().find(|n| n["design"] == json!(design)))
        .unwrap_or_else(|| panic!("{design} is named in not_searched: {reply}"))
}

#[tokio::test]
async fn a_tracked_search_finds_what_only_a_pinned_design_holds_and_names_each_it_could_not_reach()
{
    let h = hub("all-pins").await;
    let before = (
        std::fs::read(&h.deploy).expect("export"),
        std::fs::read(&h.other).expect("export"),
    );
    let found = call(
        &h.c,
        "search_design",
        json!({"query": "datastream", "tracked_designs": true}),
    )
    .await;
    let hits = found["hits"].as_array().expect("hits");
    // Every node type the member holds the word in comes back, under its
    // design and with its design's prefix.
    for (id, ty) in [
        ("deploy:cmp:role-datastream", "Component"),
        ("deploy:req:datastream-feed", "Requirement"),
        ("deploy:dec:datastream-route", "Decision"),
    ] {
        let hit = hits
            .iter()
            .find(|h| h["node_id"] == json!(id))
            .unwrap_or_else(|| panic!("{id} is a hit: {found}"));
        assert_eq!(hit["design"], json!("deploy"), "{found}");
        assert_eq!(hit["node_type"], json!(ty), "{found}");
    }
    // This design's own hit is there, unprefixed, under its own design.
    let own = hits
        .iter()
        .find(|h| h["node_id"] == json!("req:instrument-firewall"))
        .unwrap_or_else(|| panic!("the hub's own hit stays: {found}"));
    assert_eq!(own["design"], json!("hub"), "{found}");
    assert_eq!(
        found["designs_searched"],
        json!(["hub", "deploy"]),
        "{found}"
    );
    assert_eq!(found["read_from"]["deploy"], json!(h.deploy), "{found}");

    // Every pinned design it could not search: named, with a state and why.
    for (design, state) in [
        ("storage", "missing"),
        ("hosted", "not_reached"),
        ("nowhere", "nowhere"),
        ("garbled", "unreadable"),
        ("wrong", "unreadable"),
    ] {
        let n = not_searched(&found, design);
        assert_eq!(n["state"], json!(state), "{design}: {found}");
        assert!(
            n["why"].as_str().is_some_and(|w| w.len() > 20),
            "{design} says why: {found}"
        );
    }
    assert!(
        not_searched(&found, "wrong")["why"]
            .as_str()
            .is_some_and(|w| w.contains("holds design other")),
        "an export of another design is not read as the pinned one: {found}"
    );
    let named: Vec<&str> = found["not_searched"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["design"].as_str())
        .collect();
    assert!(
        !named.contains(&"deploy") && named.len() == 5,
        "only the unreached are named, a code dependency never: {found}"
    );
    assert!(
        found["view_note"]
            .as_str()
            .is_some_and(|n| n.contains("5 design(s) it pins were NOT searched")),
        "{found}"
    );

    // Nothing was written: not to an export, not to this design.
    assert_eq!(
        (
            std::fs::read(&h.deploy).expect("export"),
            std::fs::read(&h.other).expect("export")
        ),
        before,
        "an export searched is never written"
    );
    let plain = call(&h.c, "search_design", json!({"query": "datastream"})).await;
    assert_eq!(
        plain["hits"].as_array().map(Vec::len),
        Some(1),
        "the hub still holds one hit of its own: {plain}"
    );
}

#[tokio::test]
async fn without_tracked_designs_the_reply_is_what_it_was() {
    let h = hub("unchanged").await;
    let plain = call(&h.c, "search_design", json!({"query": "datastream"})).await;
    let off = call(
        &h.c,
        "search_design",
        json!({"query": "datastream", "tracked_designs": false}),
    )
    .await;
    assert_eq!(plain, off, "`tracked_designs: false` is the default");
    for key in [
        "designs_searched",
        "read_from",
        "found_by_design",
        "tracked_note",
    ] {
        assert!(plain.get(key).is_none(), "{key} absent: {plain}");
    }
    assert!(
        plain["hits"]
            .as_array()
            .is_some_and(|h| h.iter().all(|h| h.get("design").is_none())),
        "{plain}"
    );
}

#[tokio::test]
async fn a_design_that_tracks_nothing_says_so() {
    let c = connect(ReflowService::in_memory_as("alone").expect("alone")).await;
    call(
        &c,
        "add_requirement",
        json!({"id": "req:x", "name": "datastream x", "statement": "datastream."}),
    )
    .await;
    let found = call(
        &c,
        "search_design",
        json!({"query": "datastream", "tracked_designs": true}),
    )
    .await;
    assert!(
        found["tracked_note"]
            .as_str()
            .is_some_and(|n| n.contains("pins no other design")),
        "{found}"
    );
    assert_eq!(found["hits"].as_array().map(Vec::len), Some(1), "{found}");
}

#[tokio::test]
async fn the_one_limit_bounds_the_merged_list_and_says_what_it_withheld() {
    let h = hub("limit").await;
    let found = call(
        &h.c,
        "search_design",
        json!({"query": "datastream", "tracked_designs": true, "limit": 2}),
    )
    .await;
    assert_eq!(found["hits"].as_array().map(Vec::len), Some(2), "{found}");
    assert_eq!(
        found["hits_withheld"],
        json!(1),
        "each design asked for 2: 1 + 2 ranked, 2 shown: {found}"
    );
    assert_eq!(found["found_by_design"]["deploy"], json!(2), "{found}");

    let none = call(
        &h.c,
        "search_design",
        json!({"query": "zzqqxx", "tracked_designs": true}),
    )
    .await;
    assert!(
        none["empty_because"]
            .as_str()
            .is_some_and(|e| e.contains("NOT searched")),
        "an empty answer says what was not looked in: {none}"
    );
}

#[tokio::test]
async fn a_hit_in_a_pinned_design_opens_by_its_prefixed_id_or_by_member() {
    let h = hub("open").await;
    for id in [
        "cmp:role-datastream",
        "req:datastream-feed",
        "dec:datastream-route",
    ] {
        let by_prefix = call(&h.c, "get_node", json!({"id": format!("deploy:{id}")})).await;
        assert!(!by_prefix["node"].is_null(), "{id}: {by_prefix}");
        assert!(
            by_prefix["view_note"]
                .as_str()
                .is_some_and(|n| n.contains("committed export")),
            "the reply says it read the export: {by_prefix}"
        );
        let by_member = call(&h.c, "get_node", json!({"id": id, "member": "deploy"})).await;
        assert_eq!(by_prefix["node"], by_member["node"], "{id}");
    }
    // An id both hold: unprefixed is this design's own, prefixed the member's.
    let own = call(&h.c, "get_node", json!({"id": "req:shared"})).await;
    let theirs = call(&h.c, "get_node", json!({"id": "deploy:req:shared"})).await;
    assert_eq!(
        own["node"]["properties"]["name"],
        json!("hub's shared"),
        "{own}"
    );
    assert_eq!(
        theirs["node"]["properties"]["name"],
        json!("member's shared"),
        "{theirs}"
    );
    // Absent from the export: said, never a bare null.
    let absent = call(&h.c, "get_node", json!({"id": "deploy:cmp:nope"})).await;
    assert!(absent["node"].is_null(), "{absent}");
    assert!(absent["empty_because"].is_string(), "{absent}");
    // The member's own name: this design alone.
    let aimed_home = call(
        &h.c,
        "get_node",
        json!({"id": "req:shared", "member": "hub"}),
    )
    .await;
    assert_eq!(aimed_home["node"], own["node"]);

    // Aimed where nothing can be read: refused with the pin's state.
    for (design, state) in [
        ("storage", "missing"),
        ("hosted", "not_reached"),
        ("garbled", "unreadable"),
    ] {
        let why = refused(&h.c, "get_node", json!({"id": format!("{design}:req:x")})).await;
        assert!(why.contains(state), "{design}: {why}");
    }
    let why = refused(
        &h.c,
        "get_node",
        json!({"id": "req:x", "member": "elsewhere"}),
    )
    .await;
    assert!(
        why.contains("does not pin") && why.contains("deploy"),
        "a design not pinned is refused naming the pinned ones: {why}"
    );
    let why = refused(
        &h.c,
        "get_node",
        json!({"id": "deploy:req:x", "member": "wrong"}),
    )
    .await;
    assert!(why.contains("2 designs"), "{why}");
}

#[tokio::test]
async fn a_write_aimed_at_a_pinned_design_is_refused_and_its_export_is_untouched() {
    let h = hub("write").await;
    let before = std::fs::read(&h.deploy).expect("export");
    let why = refused(
        &h.c,
        "add_requirement",
        json!({"id": "req:new", "name": "new", "statement": "A new one.", "member": "deploy"}),
    )
    .await;
    assert!(why.contains("never writes"), "{why}");
    assert_eq!(std::fs::read(&h.deploy).expect("export"), before);
    // A write naming a member's id in its words is still this design's write.
    call(
        &h.c,
        "add_requirement",
        json!({"id": "req:note", "name": "note",
               "statement": "The fix is in deploy:cmp:role-datastream."}),
    )
    .await;
    let note = call(&h.c, "get_node", json!({"id": "req:note"})).await;
    assert!(!note["node"].is_null(), "{note}");
    assert_eq!(std::fs::read(&h.deploy).expect("export"), before);
}

#[tokio::test]
async fn the_served_text_asks_for_a_tracked_search_and_where_an_absence_was_checked() {
    let c = connect(ReflowService::in_memory_as("text").expect("text")).await;
    let instructions = call(&c, "get_instructions", json!({})).await.to_string();
    assert!(
        instructions.contains("say where you looked"),
        "the instructions carry the absence-claim rule"
    );
    assert!(
        instructions.contains("tracked_designs"),
        "and name the tracked search"
    );
    let skill = call(&c, "get_skill", json!({"name": "hub"}))
        .await
        .to_string();
    assert!(
        skill.contains("tracked_designs") && skill.contains("cite"),
        "the hub skill's search step names the tracked search and citing a sibling's precedent"
    );
}
