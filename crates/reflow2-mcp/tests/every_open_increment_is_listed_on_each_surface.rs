//! EVERY OPEN INCREMENT IS LISTED ON EACH SURFACE, AND THE SURFACES AGREE.
//!
//! The requirement accepted 2026-11-14: "`loop_status`, `graph_report` and the
//! where-am-i skill list every open increment (planned epoch) oldest first.
//! Each comes with how much of what was scheduled for it is delivered, and
//! which unfinished signals it shows. The list is shown at a boundary: a
//! session's start or end, or when asked, never mid-thought. It keeps showing
//! an increment until that increment is closed."
//!
//! The signals themselves are pinned in reflow2-core's
//! `an_increment_started_and_never_finished_is_found.rs`. This file pins what a
//! CLIENT receives: the same list from `loop_status` and from `graph_report`
//! (as data and as the Markdown document), the skill that says when to read it
//! out, the option that lists every one when the oldest ten are not enough,
//! and that a reply cut to a small budget still carries the counts.
//!
//! OBSERVED FAILING first: written before the section existed, on origin/main
//! 84eca9422 (v0.99.0), where neither reply carries `open_increments` and
//! `graph_report` refuses `include_open_increments` as an unknown argument.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use reflow2_core::foundation::store::{StampClock, format_stamp_time};
use reflow2_mcp::service::ReflowService;
use rmcp::ServiceExt;
use serde_json::{Value, json};

#[derive(Clone)]
struct Probe;

impl rmcp::ClientHandler for Probe {
    fn get_info(&self) -> rmcp::model::ClientConfig {
        let mut cfg = rmcp::model::ClientConfig::default();
        cfg.client_info.name = "open-increments-probe".to_string();
        cfg
    }
}

struct Session {
    client: rmcp::service::RunningService<rmcp::service::RoleClient, Probe>,
}

/// A clock that moves one second per reading, so the order the store recorded
/// the increments in is the order this file planned them in.
fn ticking() -> StampClock {
    let at = Arc::new(AtomicI64::new(1_793_145_600));
    Arc::new(move || format_stamp_time(at.fetch_add(1, Ordering::SeqCst), 0))
}

async fn session() -> Session {
    let svc = ReflowService::in_memory_on_clock(ticking()).expect("in-memory service");
    let (server_rx, client_tx) = tokio::io::duplex(1 << 22);
    let (client_rx, server_tx) = tokio::io::duplex(1 << 22);
    tokio::spawn(async move {
        if let Ok(running) = svc.serve((server_rx, server_tx)).await {
            let _ = running.waiting().await;
        }
    });
    let client = Probe
        .serve((client_rx, client_tx))
        .await
        .expect("in-process handshake");
    let s = Session { client };
    s.ok("add_project", json!({"id": "proj:1", "name": "Thing"}))
        .await;
    s
}

impl Session {
    /// `(answered, structured half, text half)`.
    async fn call(&self, tool: &str, args: Value) -> (bool, Value, String) {
        let arguments = args.as_object().cloned().unwrap_or_default();
        match self
            .client
            .call_tool(
                rmcp::model::CallToolRequestParams::new(tool.to_string()).with_arguments(arguments),
            )
            .await
        {
            Ok(r) => {
                let text = r
                    .content
                    .iter()
                    .filter_map(|c| c.as_text().map(|t| t.text.clone()))
                    .collect::<Vec<_>>()
                    .join("\n");
                (
                    !r.is_error.unwrap_or(false),
                    r.structured_content.unwrap_or(Value::Null),
                    text,
                )
            }
            Err(rmcp::service::ServiceError::McpError(e)) => {
                (false, Value::Null, e.message.to_string())
            }
            Err(e) => panic!("transport failure on {tool}: {e}"),
        }
    }

    async fn ok(&self, tool: &str, args: Value) -> Value {
        let shown = args.to_string();
        let (answered, structured, text) = self.call(tool, args).await;
        assert!(answered, "`{tool}` {shown} was refused: {text}");
        structured
    }

    async fn plan(&self, id: &str, name: &str) {
        self.ok(
            "plan_epoch",
            json!({"id": id, "name": name, "epoch_type": "milestone", "sequence": 1}),
        )
        .await;
    }

    /// A requirement scheduled for `epoch`, not delivered.
    async fn need(&self, req: &str, epoch: &str) {
        self.ok(
            "add_requirement",
            json!({"id": req, "name": req, "statement": "it must hold"}),
        )
        .await;
        self.ok("schedule_for", json!({"item_id": req, "target_id": epoch}))
            .await;
    }

    /// Deliver `req` the way the delivery reading asks: a realized capability
    /// satisfies it, a file realizes that capability, and a passing check
    /// verifies it.
    async fn deliver(&self, req: &str, suffix: &str) {
        let (cap, art, ver) = (
            format!("cap:{suffix}"),
            format!("art:{suffix}"),
            format!("ver:{suffix}"),
        );
        self.ok(
            "add_capability",
            json!({"id": cap, "name": suffix, "description": "does it", "status": "realized"}),
        )
        .await;
        self.ok("satisfies", json!({"from_id": cap, "to_id": req}))
            .await;
        self.ok(
            "add_artifact",
            json!({"id": art, "name": suffix, "artifact_type": "code",
                   "location": format!("src/{suffix}.rs")}),
        )
        .await;
        self.ok("realizes", json!({"artifact_id": art, "target_id": cap}))
            .await;
        self.ok(
            "add_verification",
            json!({"id": ver, "name": suffix, "method": "test",
                   "verifies": [{"target_id": cap}]}),
        )
        .await;
        self.ok(
            "set_verification_status",
            json!({"verification_id": ver, "status": "passing", "last_run_at": "2026-11-14"}),
        )
        .await;
    }

    /// Four open increments, one for each reading, planned in this order:
    /// `epoch:done` (everything scheduled is delivered), `epoch:passed` (an
    /// increment drawn after it has arrived), `epoch:bare` (nothing scheduled)
    /// and `epoch:going` (something outstanding, none of the three signs). And
    /// one that arrived, which is not open.
    async fn four_open(self) -> Self {
        self.plan("epoch:done", "Done and never closed").await;
        self.need("req:done", "epoch:done").await;
        self.deliver("req:done", "done").await;

        self.plan("epoch:passed", "Passed by a later one").await;
        self.need("req:passed", "epoch:passed").await;
        self.plan("epoch:later", "The later one").await;
        self.ok(
            "precedes",
            json!({"earlier_epoch": "epoch:passed", "later_epoch": "epoch:later"}),
        )
        .await;
        self.ok(
            "set_epoch_status",
            json!({"epoch_id": "epoch:later", "status": "arrived"}),
        )
        .await;

        self.plan("epoch:bare", "Nothing scheduled").await;

        self.plan("epoch:going", "Under way").await;
        self.need("req:going", "epoch:going").await;
        self
    }
}

/// Every count `open_increments` carries, on every surface.
const COUNTS: [&str; 9] = [
    "open",
    "done_but_not_closed",
    "overtaken",
    "empty",
    "no_signal",
    "never_moved",
    "changes_recorded",
    "changes_with_no_recorded_time",
    "increments_with_no_recorded_time",
];

fn ids(list: &Value) -> Vec<String> {
    list["increments"]
        .as_array()
        .expect("`increments` is a list")
        .iter()
        .map(|i| i["epoch_id"].as_str().expect("an epoch id").to_string())
        .collect()
}

fn row<'a>(list: &'a Value, epoch: &str) -> &'a Value {
    list["increments"]
        .as_array()
        .expect("`increments` is a list")
        .iter()
        .find(|i| i["epoch_id"] == epoch)
        .unwrap_or_else(|| panic!("{epoch} is listed: {list}"))
}

#[tokio::test(flavor = "multi_thread")]
async fn loop_status_and_graph_report_serve_the_same_list_oldest_first() {
    let s = session().await.four_open().await;

    let loop_status = s.ok("loop_status", json!({})).await;
    let report = s.ok("graph_report", json!({})).await;
    let from_loop = &loop_status["open_increments"];
    let from_report = &report["open_increments"];
    // One answer, read by both: the overview names the same increments in the
    // same order with every count the same, and leaves each one's row to
    // loop_status (dec:graph-reports-default-names-open-increments-by-id-and-count-2026-11-18).
    assert_eq!(
        ids(from_loop),
        ids(from_report),
        "the same increments, in order"
    );
    for count in COUNTS {
        assert_eq!(
            from_loop[count], from_report[count],
            "`{count}`: {from_report}"
        );
    }

    assert_eq!(from_loop["open"], 4, "{from_loop}");
    assert_eq!(from_loop["done_but_not_closed"], 1);
    assert_eq!(from_loop["overtaken"], 1);
    assert_eq!(from_loop["empty"], 1);
    assert_eq!(from_loop["no_signal"], 1);
    assert_eq!(
        ids(from_loop),
        ["epoch:done", "epoch:passed", "epoch:bare", "epoch:going"],
        "oldest first, and the arrived one is not listed"
    );

    // Each row: how much of what was scheduled is delivered, and its signs.
    let done = row(from_loop, "epoch:done");
    assert_eq!(
        (&done["scheduled"], &done["delivered"]),
        (&json!(1), &json!(1))
    );
    assert_eq!(done["signals"], json!(["done_but_not_closed"]));
    let passed = row(from_loop, "epoch:passed");
    assert_eq!(
        (&passed["scheduled"], &passed["delivered"]),
        (&json!(1), &json!(0))
    );
    assert_eq!(passed["signals"], json!(["overtaken"]));
    assert_eq!(passed["overtaken_by"], "epoch:later");
    let bare = row(from_loop, "epoch:bare");
    assert_eq!(bare["scheduled"], 0);
    assert_eq!(bare["signals"], json!(["empty"]));
    assert_eq!(bare["changes_since_it_moved"], "never");
    assert_eq!(row(from_loop, "epoch:going")["signals"], json!([]));

    // `next` carries the counts as one line, and says when to mention them.
    let next = loop_status["next"].as_array().expect("`next` is a list");
    let line = next
        .iter()
        .filter_map(Value::as_str)
        .find(|l| l.contains("open increment"))
        .unwrap_or_else(|| panic!("`next` names the open increments: {next:?}"));
    assert!(line.starts_with("4 open increment"), "{line}");
    assert!(
        line.contains("1 done but not closed") && line.contains("1 overtaken"),
        "{line}"
    );
    assert!(line.contains("start or end of a session"), "{line}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_markdown_report_lists_them_in_the_same_order_with_the_same_counts() {
    let s = session().await.four_open().await;
    let (answered, _, md) = s.call("graph_report", json!({"format": "markdown"})).await;
    assert!(answered, "{md}");

    let section = md
        .split("## Open increments")
        .nth(1)
        .unwrap_or_else(|| panic!("the document has an `Open increments` section:\n{md}"));
    let section = section.split("\n## ").next().unwrap_or(section);
    assert!(section.contains("**4** open"), "{section}");
    for count in ["1 done but not closed", "1 overtaken", "1 empty"] {
        assert!(section.contains(count), "missing `{count}`:\n{section}");
    }
    let at = |id: &str| {
        section
            .find(&format!("`{id}`"))
            .unwrap_or_else(|| panic!("`{id}` is listed:\n{section}"))
    };
    assert!(
        at("epoch:done") < at("epoch:passed")
            && at("epoch:passed") < at("epoch:bare")
            && at("epoch:bare") < at("epoch:going"),
        "oldest first:\n{section}"
    );
    assert!(
        !section.contains("`epoch:later` —") && !section.contains("- `epoch:later`"),
        "an arrived epoch is not a row:\n{section}"
    );
    assert!(section.contains("Nothing is called stalled"), "{section}");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_increment_leaves_every_surface_when_it_arrives() {
    let s = session().await.four_open().await;
    s.ok(
        "set_epoch_status",
        json!({"epoch_id": "epoch:bare", "status": "arrived"}),
    )
    .await;

    let loop_status = s.ok("loop_status", json!({})).await;
    let report = s.ok("graph_report", json!({})).await;
    for list in [&loop_status["open_increments"], &report["open_increments"]] {
        assert_eq!(list["open"], 3, "{list}");
        assert_eq!(list["empty"], 0, "{list}");
        assert_eq!(ids(list), ["epoch:done", "epoch:passed", "epoch:going"]);
    }
    let (_, _, md) = s.call("graph_report", json!({"format": "markdown"})).await;
    assert!(md.contains("**3** open"), "{md}");
    assert!(!md.contains("- `epoch:bare`"), "{md}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_design_with_nothing_open_says_so_with_a_zero_and_no_to_do_line() {
    let s = session().await;
    let loop_status = s.ok("loop_status", json!({})).await;
    assert_eq!(loop_status["open_increments"]["open"], 0, "{loop_status}");
    let next = loop_status["next"].as_array().expect("`next` is a list");
    assert!(
        !next
            .iter()
            .filter_map(Value::as_str)
            .any(|l| l.contains("open increment")),
        "{next:?}"
    );
    let (_, _, md) = s.call("graph_report", json!({"format": "markdown"})).await;
    assert!(!md.contains("## Open increments"), "{md}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_where_am_i_skill_says_what_to_read_and_when_to_say_it() {
    let s = session().await;
    let skill = s.ok("get_skill", json!({"name": "where-am-i"})).await;
    let body = skill["body"].as_str().expect("the skill's body");
    // The skill is prose wrapped at a line width: read it as one line.
    let body = body.split_whitespace().collect::<Vec<_>>().join(" ");
    for said in [
        // Where the list is, on both reads, and how to get all of it.
        "`loop_status` → `open_increments`",
        "`include_open_increments`",
        "**Open increments**",
        // What it holds.
        "oldest first",
        "how much of what was scheduled for it is delivered",
        // When it is said, and until when.
        "at the start or the end of a session, or when they ask",
        "never in the middle of other work",
        "until the increment is closed",
        // And what is not claimed.
        "the design calls nothing stalled",
    ] {
        assert!(body.contains(said), "where-am-i does not say `{said}`");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn asked_for_whole_graph_report_lists_every_open_increment() {
    let s = session().await;
    let long = "An increment whose name runs well past what one row of a bounded list can \
                carry without being cut short";
    s.plan("epoch:00", long).await;
    for i in 1..12 {
        s.plan(&format!("epoch:{i:02}"), &format!("Increment {i}"))
            .await;
    }

    // The everyday read: the oldest ten, the rest counted, and where they are.
    let report = s.ok("graph_report", json!({})).await;
    let bounded = &report["open_increments"];
    assert_eq!(bounded["open"], 12, "{bounded}");
    assert_eq!(ids(bounded).len(), 10);
    let full_list = bounded["full_list"].as_str().expect("`full_list`");
    assert!(
        full_list.contains("graph_report")
            && full_list.contains("include_open_increments")
            && full_list.contains("all 12"),
        "{full_list}"
    );
    // loop_status's rows carry the names, cut to a line when long.
    let at_the_boundary = s.ok("loop_status", json!({})).await;
    let first = row(&at_the_boundary["open_increments"], "epoch:00");
    assert_eq!(first["name_cut"], true, "{first}");
    assert!(first["name"].as_str().unwrap().chars().count() < long.chars().count());

    // Asked for whole: all twelve, oldest first, each name as it was written.
    let report = s
        .ok("graph_report", json!({"include_open_increments": true}))
        .await;
    let whole = &report["open_increments"];
    assert_eq!(whole["open"], 12, "{whole}");
    assert!(whole.get("full_list").is_none(), "{whole}");
    let listed = ids(whole);
    assert_eq!(listed.len(), 12);
    assert_eq!(listed.first().map(String::as_str), Some("epoch:00"));
    assert_eq!(listed.last().map(String::as_str), Some("epoch:11"));
    let first = row(whole, "epoch:00");
    assert_eq!(first["name"], long);
    assert!(first.get("name_cut").is_none(), "{first}");
}

#[tokio::test(flavor = "multi_thread")]
async fn the_markdown_document_refuses_the_whole_list_option() {
    let s = session().await;
    s.plan("epoch:a", "Increment a").await;
    let (answered, _, text) = s
        .call(
            "graph_report",
            json!({"format": "markdown", "include_open_increments": true}),
        )
        .await;
    assert!(!answered, "{text}");
    assert!(text.contains("include_open_increments"), "{text}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_reply_cut_to_a_small_budget_keeps_the_counts() {
    let s = session().await.four_open().await;
    for i in 0..25 {
        s.plan(&format!("epoch:x{i:02}"), &format!("Increment {i}"))
            .await;
    }
    // 4 from `four_open` and 25 more, all still planned: 29 open, 26 of them
    // with nothing scheduled.
    let whole = s.ok("loop_status", json!({})).await;
    let whole_size = whole.to_string().chars().count();

    for (tool, args) in [
        ("loop_status", json!({"budget_chars": 2500})),
        ("graph_report", json!({"budget_chars": 2500})),
        (
            "graph_report",
            json!({"budget_chars": 2500, "include_open_increments": true}),
        ),
    ] {
        let shown = args.to_string();
        let reply = s.ok(tool, args).await;
        let list = &reply["open_increments"];
        assert_eq!(list["open"], 29, "{tool} {shown}: {list}");
        assert_eq!(list["done_but_not_closed"], 1, "{tool} {shown}: {list}");
        assert_eq!(list["overtaken"], 1, "{tool} {shown}: {list}");
        assert_eq!(list["empty"], 26, "{tool} {shown}: {list}");
        assert!(
            !ids(list).is_empty(),
            "{tool} {shown}: no list is emptied: {list}"
        );
        // The cut withholds rows from this list like any other. Nothing in
        // the section counts its own rows, so nothing in it is wrong after:
        // the whole count stands, and the call that lists them all is named.
        assert!(
            list.get("not_listed").is_none() && list.get("listed").is_none(),
            "{tool} {shown}: {list}"
        );
        if ids(list).len() < 29 {
            let full_list = list["full_list"].as_str().unwrap_or_default();
            assert!(full_list.contains("all 29"), "{tool} {shown}: {list}");
        }
        assert!(
            reply.get("budget").is_some(),
            "{tool} {shown}: a reply that was cut says so"
        );
    }
    assert!(
        whole_size > 2500,
        "the uncut loop_status ({whole_size} chars) is larger than the budget asked for, \
         so the three replies above were really cut"
    );
}

/// THE OVERVIEW NAMES THEM BY ID, and says where each is in full.
///
/// Anthony, 2026-11-18, option A (dec:graph-reports-default-names-open-increments-by-id-and-count-2026-11-18):
/// flo2's own design's graph_report no longer read whole through flo2's chat
/// door under 0.100.0 (28,728 characters against its 28,000; 2,647 of them
/// the seven open-increment rows). The rows say what loop_status already says
/// at a session's boundary, so the overview carries every count and the
/// oldest ten by id; `include_open_increments` gives each in full.
///
/// OBSERVED FAILING first, on v0.100.0 (8918c7c5e): every row carried its
/// name, delivery, signals and changes since it moved, and `reads` was there.
#[tokio::test(flavor = "multi_thread")]
async fn the_overview_names_them_by_id_and_every_count_and_says_where_each_is_in_full() {
    let s = session().await.four_open().await;
    let report = s.ok("graph_report", json!({})).await;
    let overview = &report["open_increments"];

    for row in overview["increments"].as_array().expect("`increments`") {
        let keys: Vec<&String> = row
            .as_object()
            .expect("a row is an object")
            .keys()
            .collect();
        assert_eq!(keys, ["epoch_id"], "an id and nothing else: {row}");
    }
    assert_eq!(
        ids(overview),
        ["epoch:done", "epoch:passed", "epoch:bare", "epoch:going"],
        "oldest first"
    );
    for count in COUNTS {
        assert!(
            overview[count].is_u64(),
            "`{count}` is still carried: {overview}"
        );
    }
    assert!(
        overview.get("reads").is_none(),
        "no note on how to read rows it does not carry: {overview}"
    );
    // Every one is listed by id here, and still the call that gives each one
    // in full is named, with loop_status.
    let full_list = overview["full_list"].as_str().expect("`full_list`");
    assert!(
        full_list.contains("include_open_increments")
            && full_list.contains("all 4")
            && full_list.contains("loop_status"),
        "{full_list}"
    );

    // Asked for whole, the rows are back as loop_status gives them.
    let whole = s
        .ok("graph_report", json!({"include_open_increments": true}))
        .await;
    let loop_status = s.ok("loop_status", json!({})).await;
    assert_eq!(
        whole["open_increments"], loop_status["open_increments"],
        "four open: the whole list and loop_status's oldest ten are the same"
    );

    // Nothing open: the counts, no rows and nothing to point at.
    let empty = session().await;
    let report = empty.ok("graph_report", json!({})).await;
    let overview = &report["open_increments"];
    assert_eq!(overview["open"], 0, "{overview}");
    assert!(ids(overview).is_empty(), "{overview}");
    assert!(overview.get("full_list").is_none(), "{overview}");
}
