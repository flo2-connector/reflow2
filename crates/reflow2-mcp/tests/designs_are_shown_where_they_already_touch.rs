//! TWO DESIGNS ARE SHOWN WHERE THEY ALREADY TOUCH, BEFORE ANYONE DECLARES IT
//! — `req:designs-are-shown-where-they-already-touch-before-anyone-declares-it`,
//! governed by `dec:two-designs-are-linked-on-exact-evidence-and-a-similar-name-is-only-a-candidate`
//! (phase 7, step 4).
//!
//! WHAT FAILED. Measured 2026-10-21 on main (e6d4b5a4f): composing a hub with
//! its members, each under its own namespace, showed a seam only where a
//! check happened to read two designs at once. Nothing read the evidence the
//! records already hold — the same file in two designs, a pin in one naming
//! the other — so in hxm_program's hub of fifteen designs the seam bucket
//! stayed empty exactly where the cross-design bugs are, and an empty answer
//! read like agreement.
//!
//! THE CLASS, pinned across the evidence kinds (a location qualified by a
//! shared repository, an equal checksum, a pin naming a graph_id, a pin
//! naming a repository, a bare common filename, a name alone), across two and
//! several designs (the one-other-design call, `pair_designs`,
//! `other_graph_ids`, `tracked_designs`) and across pin states:
//!
//! 1. exact evidence is a `computed` link, drawn as a real edge in the
//!    combined graph, so the ordinary checks see across it: a member joined
//!    by one no longer reads as a cut-off cluster;
//! 2. a bare relative path (README.md, LICENSE, Cargo.toml) and a stock
//!    file's equal checksum are never evidence, and are counted as set aside;
//! 3. a similar name is a `candidate`, never a link, never an edge;
//! 4. every pair has a `link_status`: `linked`, `candidates_only`,
//!    `never_touch` (checked) or `not_checked` (a design of it was not read);
//! 5. the one-other-design reply keeps its old keys and gains only the link
//!    fields.

use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use rmcp::model::CallToolRequestParams;
use serde_json::{Value, json};

struct TestClient;

impl rmcp::ClientHandler for TestClient {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "designs-are-shown-where-they-touch".to_string();
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

async fn call(c: &Client, tool: &str, args: Value) -> Value {
    let Value::Object(arguments) = args.clone() else {
        panic!("arguments for {tool} must be an object");
    };
    match c
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
    {
        Ok(r) if r.is_error == Some(true) => panic!("{tool} {args} refused: {:?}", r.content),
        Ok(r) => r.structured_content.unwrap_or(Value::Null),
        Err(e) => panic!("{tool} {args} refused: {e}"),
    }
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "reflow2-where-designs-touch-{name}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// The evidence kinds, each a way two designs P and Q might touch.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    /// Both register the same relative path, and pins say both live in ONE
    /// repository: the same file.
    SameLocation,
    /// Both register content with one checksum, under different names.
    SameChecksum,
    /// P pins Q by Q's graph_id (and wires the pin into its own structure).
    PinByGraphId,
    /// P pins a repository by URL, no graph_id; Q is pinned elsewhere as
    /// living there (spelled another way: scp form, `.git`).
    PinByRepository,
    /// Both register README.md, LICENSE and Cargo.toml (LICENSE with one
    /// checksum), in DIFFERENT repositories: nothing.
    BareCommonName,
    /// One interface each, closely matching names, the same medium: a
    /// candidate, nothing more.
    NameOnly,
}

const KINDS: [Kind; 6] = [
    Kind::SameLocation,
    Kind::SameChecksum,
    Kind::PinByGraphId,
    Kind::PinByRepository,
    Kind::BareCommonName,
    Kind::NameOnly,
];

impl Kind {
    /// The evidence a link of this kind carries, or none.
    fn evidence(self) -> Option<&'static str> {
        match self {
            Kind::SameLocation => Some("same_location"),
            Kind::SameChecksum => Some("same_checksum"),
            Kind::PinByGraphId => Some("pin_names_graph_id"),
            Kind::PinByRepository => Some("pin_names_repository"),
            Kind::BareCommonName | Kind::NameOnly => None,
        }
    }
    /// The repository each of P and Q lives in, as pins state it.
    fn repositories(self) -> (&'static str, &'static str) {
        match self {
            Kind::SameLocation => (
                "https://github.com/NSLS2/ansible",
                "https://github.com/NSLS2/ansible.git",
            ),
            Kind::PinByRepository => (
                "https://github.com/NSLS2/ansible",
                "git@github.com:bluesky/bluesky-tiled-plugins.git",
            ),
            _ => (
                "https://github.com/NSLS2/ansible",
                "https://github.com/bluesky/bluesky-tiled-plugins",
            ),
        }
    }
}

/// One design with a threaded body: a Component, a Capability allocated to
/// it, an Artifact realizing it (a cluster of three, four for the larger).
async fn body(c: &Client, me: &str, larger: bool) {
    call(
        c,
        "add_project",
        json!({"id": format!("proj:{me}"), "name": me}),
    )
    .await;
    call(
        c,
        "add_component",
        json!({"id": "cmp:role", "name": format!("{me} role"), "description": "deploys it"}),
    )
    .await;
    call(
        c,
        "add_capability",
        json!({"id": "cap:serve", "name": format!("{me} serves"), "description": "serves"}),
    )
    .await;
    call(
        c,
        "create_edge",
        json!({"edge_type": "ALLOCATED_TO", "from_id": "cap:serve", "to_id": "cmp:role"}),
    )
    .await;
    call(
        c,
        "add_artifact",
        json!({"id": "art:main", "name": format!("{me} main"), "location": format!("src/{me}_main.py")}),
    )
    .await;
    call(
        c,
        "create_edge",
        json!({"edge_type": "REALIZES", "from_id": "art:main", "to_id": "cap:serve"}),
    )
    .await;
    if larger {
        call(
            c,
            "add_capability",
            json!({"id": "cap:more", "name": format!("{me} more"), "description": "more"}),
        )
        .await;
        call(
            c,
            "create_edge",
            json!({"edge_type": "ALLOCATED_TO", "from_id": "cap:more", "to_id": "cmp:role"}),
        )
        .await;
    }
}

/// P's or Q's own records for one evidence kind.
async fn evidence(c: &Client, kind: Kind, me: &str, other: &str, is_p: bool) {
    let art = |id: &str, location: &str, checksum: Option<&str>| {
        let mut a = json!({"id": id, "name": id, "location": location});
        if let Some(h) = checksum {
            a["checksum"] = json!(h);
        }
        a
    };
    match kind {
        Kind::SameLocation => {
            call(
                c,
                "add_artifact",
                art("art:tasks", "./roles/pdfstream/tasks/main.yml", None),
            )
            .await;
            call(
                c,
                "create_edge",
                json!({"edge_type": "REALIZES", "from_id": "art:tasks", "to_id": "cmp:role"}),
            )
            .await;
        }
        Kind::SameChecksum => {
            let loc = if is_p {
                "lib/stream.py"
            } else {
                "src/pdf/stream_copy.py"
            };
            call(
                c,
                "add_artifact",
                art(
                    "art:stream",
                    loc,
                    Some("sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08"),
                ),
            )
            .await;
            call(
                c,
                "create_edge",
                json!({"edge_type": "REALIZES", "from_id": "art:stream", "to_id": "cmp:role"}),
            )
            .await;
        }
        Kind::PinByGraphId if is_p => {
            call(
                c,
                "external_dependency",
                json!({"id": format!("dep:{other}"), "name": other, "version": "v1",
                       "source": "https://example.org/somewhere-else", "graph_id": other}),
            )
            .await;
            call(
                c,
                "create_edge",
                json!({"edge_type": "DEPENDS_ON", "from_id": "cmp:role", "to_id": format!("dep:{other}")}),
            )
            .await;
        }
        Kind::PinByRepository if is_p => {
            call(
                c,
                "external_dependency",
                json!({"id": "dep:tiled-plugins", "name": "tiled plugins", "version": "v1",
                       "source": "https://github.com/bluesky/bluesky-tiled-plugins"}),
            )
            .await;
            call(
                c,
                "create_edge",
                json!({"edge_type": "DEPENDS_ON", "from_id": "cmp:role", "to_id": "dep:tiled-plugins"}),
            )
            .await;
        }
        Kind::BareCommonName => {
            for (id, loc, h) in [
                ("art:readme", "README.md", None),
                (
                    "art:license",
                    "LICENSE",
                    Some("sha256:2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881"),
                ),
                ("art:cargo", "./Cargo.toml", None),
            ] {
                call(c, "add_artifact", art(id, loc, h)).await;
                call(
                    c,
                    "create_edge",
                    json!({"edge_type": "REALIZES", "from_id": id, "to_id": "cmp:role"}),
                )
                .await;
            }
        }
        Kind::NameOnly => {
            let name = if is_p {
                "Tiled stream feed"
            } else {
                "tiled stream feeds"
            };
            call(
                c,
                "create_node",
                json!({"node_type": "Interface", "id": "ifc:feed",
                       "props": {"name": name, "medium": "REST"}}),
            )
            .await;
            call(
                c,
                "create_edge",
                json!({"edge_type": "PROVIDES", "from_id": "cmp:role", "to_id": "ifc:feed"}),
            )
            .await;
        }
        _ => {}
    }
    let _ = me;
}

/// A design's export, written to `dir`, and the document.
async fn export(c: &Client, dir: &std::path::Path, me: &str) -> (String, Value) {
    let path = dir.join(format!("{me}.json"));
    call(c, "export_graph", json!({"path": path.to_string_lossy()})).await;
    let doc = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    (path.to_string_lossy().into_owned(), doc)
}

struct World {
    hub: Client,
    p: Client,
    p_doc: Value,
    q_doc: Value,
}

/// P and Q touching (or not) by `kind`, each exported; and a hub pinning
/// both by their exports (stating the repositories), plus Z, a design that
/// touches neither, and two designs it cannot read: an export missing, and
/// one at an address.
async fn world(kind: Kind, tag: &str) -> World {
    let dir = scratch(&format!("{tag}-{kind:?}"));
    let (p_repo, q_repo) = kind.repositories();
    let p = connect(ReflowService::in_memory_as("pp").expect("p")).await;
    body(&p, "pp", true).await;
    evidence(&p, kind, "pp", "qq", true).await;
    let q = connect(ReflowService::in_memory_as("qq").expect("q")).await;
    body(&q, "qq", false).await;
    evidence(&q, kind, "qq", "pp", false).await;
    let (p_path, p_doc) = export(&p, &dir, "pp").await;
    let (q_path, q_doc) = export(&q, &dir, "qq").await;
    let z = connect(ReflowService::in_memory_as("zz").expect("z")).await;
    body(&z, "zz", false).await;
    let (z_path, _) = export(&z, &dir, "zz").await;

    let hub = connect(ReflowService::in_memory_as("hub").expect("hub")).await;
    call(
        &hub,
        "add_requirement",
        json!({"id": "req:effort", "name": "an effort", "statement": "The effort spans designs."}),
    )
    .await;
    for (id, source, extra) in [
        ("pp", p_repo, json!({"design_export": p_path})),
        ("qq", q_repo, json!({"design_export": q_path})),
        (
            "zz",
            "https://github.com/elsewhere/zz",
            json!({"design_export": z_path}),
        ),
        (
            "gone",
            "https://github.com/elsewhere/gone",
            json!({"design_export": dir.join("gone.json").to_string_lossy()}),
        ),
        (
            "hosted",
            "https://github.com/elsewhere/hosted",
            json!({"design_address": "http://127.0.0.1:9/g/hosted/mcp"}),
        ),
    ] {
        let mut pin = json!({"id": format!("dep:{id}"), "name": id, "version": "v1",
                             "source": source, "graph_id": id});
        for (k, v) in extra.as_object().unwrap() {
            pin[k] = v.clone();
        }
        call(&hub, "external_dependency", pin).await;
    }
    World {
        hub,
        p,
        p_doc,
        q_doc,
    }
}

/// The links of a reply between designs `a` and `b`, either way round.
fn links_between<'a>(reply: &'a Value, list: &str, a: &str, b: &str) -> Vec<&'a Value> {
    reply[list]
        .as_array()
        .unwrap_or_else(|| panic!("`{list}` is a list: {reply}"))
        .iter()
        .filter(|l| {
            let (f, t) = (l["from_design"].as_str(), l["to_design"].as_str());
            (f == Some(a) && t == Some(b)) || (f == Some(b) && t == Some(a))
        })
        .collect()
}

/// The status of pair a×b in a several-design reply.
fn status(reply: &Value, a: &str, b: &str) -> String {
    let ls = &reply["link_status"];
    for key in [format!("{a}×{b}"), format!("{b}×{a}")] {
        if ls["linked"].get(&key).is_some() {
            return "linked".into();
        }
        if ls["candidates_only"].get(&key).is_some() {
            return "candidates_only".into();
        }
        if ls["not_checked"]
            .as_array()
            .is_some_and(|l| l.iter().any(|k| k.as_str() == Some(key.as_str())))
        {
            return "not_checked".into();
        }
        if ls["never_touch"]
            .as_array()
            .is_some_and(|l| l.iter().any(|k| k.as_str() == Some(key.as_str())))
        {
            return "never_touch".into();
        }
    }
    panic!("pair {a}×{b} has no link_status: {reply}")
}

fn expected_status(kind: Kind) -> &'static str {
    match kind {
        Kind::BareCommonName => "never_touch",
        Kind::NameOnly => "candidates_only",
        _ => "linked",
    }
}

/// The ids every cut-off-cluster finding names.
fn cut_off_ids(reply: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let mut each = |f: &Value| {
        if f["source"]
            .as_str()
            .is_some_and(|s| s.contains("UnthreadedCluster"))
        {
            for id in f["affected_ids"].as_array().unwrap() {
                out.push(id.as_str().unwrap().to_string());
            }
        }
    };
    for list in reply["by_design"].as_object().unwrap().values() {
        list.as_array().unwrap().iter().for_each(&mut each);
    }
    for list in reply["by_pair"].as_object().unwrap().values() {
        list.as_array().unwrap().iter().for_each(&mut each);
    }
    reply["across"]
        .as_array()
        .unwrap()
        .iter()
        .for_each(&mut each);
    out
}

#[tokio::test]
async fn exact_evidence_links_two_designs_and_a_name_or_a_bare_path_never_does_across_every_shape()
{
    for kind in KINDS {
        let w = world(kind, "shapes").await;
        // SEVERAL designs: every design the hub pins, and two named.
        let tracked = call(
            &w.hub,
            "compose_and_analyse",
            json!({"tracked_designs": true}),
        )
        .await;
        let named = call(
            &w.hub,
            "compose_and_analyse",
            json!({"other_graph_ids": ["pp", "qq"]}),
        )
        .await;
        for (shape, reply) in [("tracked_designs", &tracked), ("other_graph_ids", &named)] {
            let computed = links_between(reply, "computed_links", "pp", "qq");
            match kind.evidence() {
                Some(e) => {
                    assert!(
                        computed.iter().any(|l| l["evidence"] == json!(e)
                            && l["link"] == json!("computed")
                            && l["drawn"] == json!(true)),
                        "{kind:?} via {shape}: a drawn `{e}` link between pp and qq: {reply}"
                    );
                }
                None => assert!(
                    computed.is_empty()
                        && links_between(reply, "declared_links", "pp", "qq").is_empty(),
                    "{kind:?} via {shape}: NO link between pp and qq: {reply}"
                ),
            }
            assert_eq!(
                status(reply, "pp", "qq"),
                expected_status(kind),
                "{kind:?} via {shape}: {}",
                reply["link_status"]
            );
            // Name matches are candidates, never links.
            let named_pair = reply["candidates"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["a_design"] == json!("pp") && c["b_design"] == json!("qq"));
            assert_eq!(
                named_pair,
                kind == Kind::NameOnly,
                "{kind:?} via {shape}: {reply}"
            );
            // The hub's pins name every member it reads: linked, computed.
            let members: &[&str] = if shape == "tracked_designs" {
                &["pp", "qq", "zz"]
            } else {
                &["pp", "qq"]
            };
            for m in members {
                assert!(
                    links_between(reply, "computed_links", "hub", m)
                        .iter()
                        .any(|l| l["evidence"] == json!("pin_names_graph_id")),
                    "{kind:?} via {shape}: the hub's pin names {m}: {reply}"
                );
            }
            // Z touches neither P nor Q: checked, and said so.
            if shape == "tracked_designs" {
                assert_eq!(status(reply, "pp", "zz"), "never_touch", "{kind:?} {shape}");
            }
        }
        // Designs not read: every pair with them is `not_checked`, never
        // `never_touch`.
        for d in ["hub", "pp", "qq", "zz"] {
            for m in ["gone", "hosted"] {
                assert_eq!(status(&tracked, d, m), "not_checked", "{kind:?}: {d}×{m}");
            }
        }
        assert_eq!(status(&tracked, "gone", "hosted"), "not_checked");

        // A BARE PATH and A STOCK FILE'S CHECKSUM are counted as set aside.
        if kind == Kind::BareCommonName {
            let aside = &tracked["set_aside"];
            assert!(
                aside["bare_path_matches"].as_u64().unwrap_or(0) >= 3,
                "README.md, LICENSE, Cargo.toml in two repositories: {aside}"
            );
            assert!(
                aside["common_content_checksums"].as_u64().unwrap_or(0) >= 1,
                "one LICENSE text in both: {aside}"
            );
        }

        // TWO designs: P composes Q (the one-other-design call) and pairs it.
        // Here only P's and Q's own records speak, so the repository-qualified
        // kinds, whose repositories only the hub states, are not evidence —
        // and the reply says `never_touch`, not agreement.
        let two = call(
            &w.p,
            "compose_and_analyse",
            json!({"design": w.q_doc, "namespace": "qq"}),
        )
        .await;
        let paired = call(&w.p, "pair_designs", json!({"design": w.q_doc})).await;
        let two_evidence: Option<&str> = match kind {
            Kind::SameLocation | Kind::PinByRepository => None,
            k => k.evidence(),
        };
        for (shape, reply) in [("compose, one other", &two), ("pair_designs", &paired)] {
            let links = reply["computed_links"].as_array().unwrap();
            match two_evidence {
                Some(e) => assert!(
                    links.iter().any(|l| l["evidence"] == json!(e)),
                    "{kind:?} via {shape}: `{e}`: {reply}"
                ),
                None => assert!(links.is_empty(), "{kind:?} via {shape}: no link: {reply}"),
            }
            let want = match (two_evidence, kind) {
                (Some(_), _) => "linked",
                (None, Kind::NameOnly) if shape != "pair_designs" => "candidates_only",
                _ => "never_touch",
            };
            assert_eq!(
                reply["link_status"]["status"],
                json!(want),
                "{kind:?} via {shape}"
            );
        }
        assert_eq!(
            two["candidates"].as_array().unwrap().is_empty(),
            kind != Kind::NameOnly,
            "{kind:?}: {two}"
        );
        let _ = &w.p_doc;
    }
}

#[tokio::test]
async fn a_computed_link_is_a_real_edge_the_checks_see_across_and_a_name_is_not() {
    // Compose P and Q alone (two designs, named) under each kind: Q's body is
    // a cut-off cluster beside P unless a drawn link threads it to P.
    for kind in KINDS {
        let w = world(kind, "threads").await;
        let reply = call(
            &w.hub,
            "compose_and_analyse",
            json!({"other_graph_ids": ["pp", "qq"]}),
        )
        .await;
        let q_cut_off = cut_off_ids(&reply).iter().any(|id| id == "qq:cmp:role");
        let threads = matches!(
            kind,
            Kind::SameLocation | Kind::SameChecksum | Kind::PinByGraphId | Kind::PinByRepository
        );
        assert_eq!(
            q_cut_off,
            !threads,
            "{kind:?}: Q's body is cut off from P exactly when nothing exact links them: {}",
            serde_json::to_string(&reply["by_design"]).unwrap()
        );
        // The note says what a design with no link reads as.
        assert!(
            reply["composed_note"]
                .as_str()
                .is_some_and(|n| n.contains("reads, beside them, as a cut-off cluster")),
            "{reply}"
        );
    }
}

#[tokio::test]
async fn composing_with_links_writes_nothing_and_the_one_other_design_reply_only_gains_link_fields()
{
    let w = world(Kind::SameChecksum, "writes").await;
    let hash = |v: Value| v["content_hash"].as_str().unwrap().to_string();
    let before = hash(call(&w.hub, "export_graph", json!({})).await);
    let p_before = hash(call(&w.p, "export_graph", json!({})).await);
    call(
        &w.hub,
        "compose_and_analyse",
        json!({"tracked_designs": true}),
    )
    .await;
    let old = call(
        &w.p,
        "compose_and_analyse",
        json!({"design": w.q_doc, "namespace": "qq"}),
    )
    .await;
    assert_eq!(hash(call(&w.hub, "export_graph", json!({})).await), before);
    assert_eq!(hash(call(&w.p, "export_graph", json!({})).await), p_before);
    // THE OLD SHAPES: inline design + namespace, with and without a budget.
    for args in [
        json!({"design": w.q_doc, "namespace": "qq"}),
        json!({"design": w.q_doc, "namespace": "qq", "tracked_designs": false}),
        json!({"design": w.q_doc, "namespace": "q2", "budget_chars": 100000}),
    ] {
        let got = call(&w.p, "compose_and_analyse", args.clone()).await;
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
            "{args}"
        );
        // Ids keep the old `<namespace>::<id>` form.
        let ns = args["namespace"].as_str().unwrap();
        for l in got["computed_links"].as_array().unwrap() {
            assert!(
                l["to_id"].as_str().unwrap().starts_with(&format!("{ns}::")),
                "{l}"
            );
        }
    }
    let _ = old;
}
