//! The hub's check shows, for every pinned member, the source revision the hub
//! last reviewed it at beside the state of the member's design record
//! (`req:a-hubs-check-shows-each-members-reviewed-revision`).
//!
//! WHAT FAILED. A hub keeping a fast-moving member in step (Example/deploy
//! merges 2-5 PRs a day) needs to see, per member, "reviewed at X, its design
//! record reads Y". The pin already carries the revision (`version`, required),
//! but measured 2026-10-21 on 0.89.0 `upstream_status` printed it only inside
//! the `moved` sentence, so the hub kept a `fact:<member>-reviewed-revision`
//! convention of its own. reflow2 still fetches nothing; the hub's tooling
//! does, and re-declares the pin.
//!
//! THE CLASS: every member `upstream_status` lists carries the pin's reviewed
//! revision and the state of its record, whatever that state is, and the
//! state agrees with the finding about the same pin.

use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

macro_rules! j {
    ($call:expr) => {
        $call
            .await
            .expect("tool ok")
            .structured_content
            .expect("structured content present")
    };
}

macro_rules! call {
    ($s:expr, $tool:ident, $args:expr) => {
        j!($s.$tool(Parameters(serde_json::from_value($args).unwrap())))
    };
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("reflow2-reviewed-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

async fn export_of(dir: &std::path::Path, graph_id: &str, extra: Option<&str>) -> String {
    let m = ReflowService::in_memory_as(graph_id).expect("member");
    call!(
        m,
        add_component,
        json!({"id": "cmp:role", "name": "a role", "description": "deploys a thing"})
    );
    if let Some(id) = extra {
        call!(
            m,
            add_component,
            json!({"id": id, "name": "later", "description": "work done afterwards"})
        );
    }
    let path = dir.join(format!("{graph_id}.json"));
    let _ = std::fs::remove_file(&path);
    call!(m, export_graph, json!({"path": path.to_string_lossy()}));
    path.to_string_lossy().into_owned()
}

#[tokio::test]
async fn every_member_carries_its_reviewed_revision_beside_its_records_state() {
    let dir = scratch("all-states");
    let still = export_of(&dir, "still", None).await;
    let moving = export_of(&dir, "moving", None).await;
    let s = ReflowService::in_memory_as("hub").expect("hub");
    call!(s, add_project, json!({"id": "proj:hub", "name": "Hub"}));
    let pins = [
        (
            "dep:still",
            "still",
            "aaa111",
            json!({"design_export": still}),
        ),
        (
            "dep:moving",
            "moving",
            "bbb222",
            json!({"design_export": moving}),
        ),
        (
            "dep:gone",
            "gone",
            "ccc333",
            json!({"design_export": dir.join("gone.json").to_string_lossy()}),
        ),
        ("dep:nowhere", "nowhere", "ddd444", json!({})),
    ];
    for (id, graph_id, version, watch) in &pins {
        let mut args = json!({"id": id, "name": graph_id, "source": "https://example.org/x",
                              "version": version, "graph_id": graph_id,
                              "relation": ["part_of"]});
        for (k, v) in watch.as_object().unwrap() {
            args[k] = v.clone();
        }
        call!(s, external_dependency, args);
    }
    // The member moves after the hub reviewed it.
    export_of(&dir, "moving", Some("cmp:new-role")).await;

    let status = j!(s.upstream_status(Parameters(UpstreamStatusReq {})));
    let members = status["members"].as_array().expect("members listed");
    assert_eq!(members.len(), pins.len(), "{status}");
    let watch_kinds = [
        "unchanged",
        "moved",
        "never_seen",
        "missing",
        "not_watched",
        "not_observed",
        "unreadable",
        "unreachable",
        "refused",
        "graph_id_mismatch",
    ];
    for (id, _, version, _) in &pins {
        let m: &Value = members
            .iter()
            .find(|m| m["dependency"] == json!(id))
            .unwrap_or_else(|| panic!("{id} listed: {status}"));
        assert_eq!(
            m["reviewed_revision"],
            json!(version),
            "{id} shows the revision the hub last reviewed it at: {status}"
        );
        let state = m["record_state"]
            .as_str()
            .unwrap_or_else(|| panic!("{id} shows its record's state: {status}"));
        let finding = status["findings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| {
                f["dependency"] == json!(id)
                    && watch_kinds.contains(&f["kind"].as_str().unwrap_or(""))
            })
            .unwrap_or_else(|| panic!("{id} has a watch finding: {status}"));
        assert_eq!(
            state,
            finding["kind"].as_str().unwrap(),
            "{id}: the member's state is the finding's: {status}"
        );
    }
    let state_of = |id: &str| {
        members
            .iter()
            .find(|m| m["dependency"] == json!(id))
            .map(|m| m["record_state"].clone())
            .unwrap()
    };
    assert_eq!(state_of("dep:still"), json!("unchanged"));
    assert_eq!(state_of("dep:moving"), json!("moved"));
    assert_eq!(state_of("dep:gone"), json!("missing"));
    assert_eq!(state_of("dep:nowhere"), json!("not_watched"));
}

/// Catching up is a re-declare with the new revision: the pin reads it, and
/// the record's baseline is re-taken with it.
#[tokio::test]
async fn re_declaring_the_pin_at_a_new_revision_is_how_the_hub_catches_up() {
    let dir = scratch("catch-up");
    let path = export_of(&dir, "member", None).await;
    let s = ReflowService::in_memory_as("hub").expect("hub");
    let pin = |version: &str| {
        json!({"id": "dep:member", "name": "member", "source": "https://example.org/m",
               "version": version, "graph_id": "member", "design_export": path,
               "relation": ["part_of"]})
    };
    call!(s, external_dependency, pin("r1"));
    export_of(&dir, "member", Some("cmp:new")).await;
    let status = j!(s.upstream_status(Parameters(UpstreamStatusReq {})));
    assert_eq!(
        status["members"][0]["record_state"],
        json!("moved"),
        "{status}"
    );
    assert_eq!(
        status["members"][0]["reviewed_revision"],
        json!("r1"),
        "{status}"
    );

    call!(s, external_dependency, pin("r2"));
    let status = j!(s.upstream_status(Parameters(UpstreamStatusReq {})));
    assert_eq!(
        status["members"][0]["record_state"],
        json!("unchanged"),
        "{status}"
    );
    assert_eq!(
        status["members"][0]["reviewed_revision"],
        json!("r2"),
        "{status}"
    );
}
