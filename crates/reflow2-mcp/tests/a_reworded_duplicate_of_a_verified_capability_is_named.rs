//! A requirement that restates a verified capability in other words is told
//! the capability exists.
//!
//! MEASURED 2026-11-01 (`fact:root-cause-the-near-match-guard-stayed-silent-on-
//! a-requirement-that-restates-a-verified-capability-2026-11-01`): a
//! requirement was written that every reflow2 change writes its own
//! release-note file, folded into the CHANGELOG at the release. The design had
//! held exactly that since 2026-10-03, as an accepted decision and a verified
//! capability, and `add_requirement` said nothing. Replaying the guard's own
//! query on the design: the new node scored 281.1 against its own text, so
//! the floor (half of it) was 140.5; the decision scored 149.0 and the
//! capability 134.1, under the floor.
//!
//! The cause is the floor, not the index. A node scored against its own text
//! sits structurally far above every other hit, so half of it lands inside the
//! band of real neighbours, and a node that says the same thing IN OTHER WORDS
//! falls under it. That is the pair the guard exists for: "a Requirement and a
//! Capability can say the same thing, and that pair is the one worth
//! catching" (capture.rs).
//!
//! WHAT THIS PINS: the real pair, in their own words, inside a design that
//! also holds a CHANGELOG neighbourhood and unrelated work. The capability and
//! the decision are named; nothing unrelated is; the report stays bounded.
//! `req:the-near-match-guard-names-a-reworded-duplicate-of-a-verified-capability`.
//!
//! ⚠️ THE PIN IS A KNOWN GAP, SWITCHED OFF, AND THAT IS ON PURPOSE. It was
//! observed failing on today's cut ("It named []": self 164.6, floor 82.3, the
//! capability 68.5, the decision 62.3). Every cut on today's score that was
//! measured to catch it raised false alarms at some design size, so none was
//! shipped (`fact:no-cut-tried-for-the-near-match-guard-catches-a-reworded-
//! duplicate-without-new-false-alarms-2026-11-01`). Anthony ruled on
//! 2026-11-01 to keep today's cut, push this pin as a known gap, and measure a
//! different signal next
//! (`dec:how-the-near-match-guard-tells-a-reworded-duplicate-from-a-neighbour`).
//! Remove the `ignore` in the change that ships a signal that clears the bar.
//! `cargo test ... -- --ignored a_reworded_duplicate` runs it meanwhile.
//!
//! THE COUNTERWEIGHT BELOW RUNS: an unrelated requirement written beside the
//! pair is told nothing. Whatever closes the gap must keep it green.

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

/// The capability, as the design holds it (verified since 2026-10-04).
const CAP_ID: &str = "cap:changelog-entries-are-fragments-assembled-at-the-cut";
const CAP_NAME: &str =
    "Each pull request writes its CHANGELOG entry as its own fragment, and the cut assembles them";
const CAP_DESCRIPTION: &str = "A PR writes changelog.d/<slug>.md under Keep a Changelog headings \
     instead of editing CHANGELOG.md; tools/changelog_fragments.py --check (CI) refuses a \
     malformed fragment or an entry written straight into [Unreleased]; --cut <version> \
     assembles the fragments into CHANGELOG.md under the new version, grouped by section, and \
     removes them. Two PRs never edit one spot. Option (b) of \
     dec:idea-how-changelog-md-gets-written-once-the-design-is-the-record (Anthony, 2026-10-03).";

/// The decision that settled it, as the design holds it (its name and the
/// part of its body that states the options and the settlement).
const DEC_ID: &str = "dec:idea-how-changelog-md-gets-written-once-the-design-is-the-record";
const DEC_NAME: &str = "SETTLED 2026-10-03 — CHANGELOG.md is written as one fragment file per \
     PR, assembled at the release cut (option b)";
const DEC_BODY: &str = "THE PAIN, measured: every PR hand-edits CHANGELOG.md's Unreleased \
     section, so parallel PRs conflict there. OPTIONS: (a) As today: each PR hand-edits \
     Unreleased. It costs a conflict per parallel PR. (b) ONE FRAGMENT FILE PER PR \
     (changelog.d/<pr>.md, the towncrier pattern). The release cut assembles the fragments. No \
     conflicts, independent of where the design lives, and the author writes the upgrade step \
     while they still know it. (c) DRAFTED FROM THE DESIGN AT THE RELEASE CUT. changelog_view \
     renders the ChangeEvents since the last deployed release. Each PR records its \
     consumer-facing note, including any upgrade step, on its own ChangeEvent, so the design is \
     the single source. (d) Drafted from the design at EACH PR, from a fresh export, just before \
     merge. SIZE AND COUNTERS: (b) is small and standard. Its counter: a second record beside \
     the design, which can disagree with it. (d) inherits (c)'s obstacles, and a per-PR draft \
     still conflicts in Unreleased unless each draft lands in its own file. 2026-10-03 — \
     SETTLED ON (b), ONE FRAGMENT FILE PER PR. The proposal accepted: CHANGELOG fragments \
     assembled at release, and design records committed as per-PR fragments rather than a full \
     export per PR. WHY (c) and (d) lost: they need the upgrade-step text captured at PR time \
     anyway. (b) is small and standard, and removes the conflict now.";
const DEC_RATIONALE: &str = "The changelog conflict is the half of the move question that does \
     not depend on where the design lives. Options (b) and (c) remove it whether reflow2's \
     design stays local or moves to flo2.io.";

/// The requirement written on 2026-11-01, in its original words (before the
/// note appended to it later that day).
const PROBE_ID: &str = "req:each-reflow2-change-writes-its-own-release-note-file";
const PROBE_NAME: &str = "Each reflow2 change writes its own release-note file, and the release \
     folds them into the CHANGELOG, so two open PRs never conflict over it";
const PROBE_STATEMENT: &str = "A reflow2 PR adds its release note as a new file of its own \
     instead of editing CHANGELOG.md. At release, the notes are folded into CHANGELOG.md under \
     the version and the files are removed. Two open PRs then never conflict over the \
     CHANGELOG, and a conflicting PR no longer silently gets no CI. A check fails a PR that \
     changes behaviour and adds no note, and a release whose folded CHANGELOG disagrees with its \
     notes.";

/// The CHANGELOG neighbourhood: records that share the vocabulary and are NOT
/// this need. In the real design the window around the probe was full of
/// them (a deprecation announced in the notes, a release cut, the upgrade
/// notes retiring into the CHANGELOG).
const NEIGHBOURS: [(&str, &str, &str); 4] = [
    (
        "req:the-changelog-is-the-one-record-of-what-changed",
        "CHANGELOG.md is the one record of what changed between versions, and no upgrade step a \
         consumer must take is lost when the per-version upgrade notes retire",
        "The per-version upgrade notes retire. CHANGELOG.md is the one place a person reads what \
         changed in each version. Where an upgrade asks a consumer to DO something, a migration \
         or a required command, those steps are kept in the CHANGELOG under the version.",
    ),
    (
        "cap:a-served-item-is-removed-only-after-a-release-that-announced-it",
        "A served item is removed only after a release that announced it",
        "One registry declares each deprecation, the surface marks it, a call that uses it is \
         told, and the surface check lets the next release remove it. The announcement is a \
         Deprecated entry in the release notes with what to do first.",
    ),
    (
        "cap:the-release-cut-tags-builds-and-publishes-one-version",
        "The release cut tags, builds and publishes one version",
        "The cut bumps the workspace version, writes the release's section of the notes, tags \
         it, and the release workflow builds every binary and image from that tag alone.",
    ),
    (
        "dec:a-deprecation-says-where-it-was-announced",
        "A deprecation says which release announced it, and the check reads the last release",
        "Each registry entry names the release whose notes announced it, and the announcement \
         check reads the last released version from the top of the notes rather than a constant.",
    ),
];

/// Unrelated work, so the shared terms above are distinctive rather than
/// universal, as they are in a real design of thousands of nodes.
const UNRELATED: [(&str, &str, &str); 12] = [
    (
        "req:a-claim-is-advisory-and-never-a-lock",
        "A claim on a region of the design is advisory and never blocks a write",
        "A contributor claims everything within a few hops of a seed so colleagues can see it is \
         held. Nothing consults the claim before a write, and a second person who ignores it \
         still gets a correct merge.",
    ),
    (
        "cap:search-finds-a-node-by-what-it-says",
        "Search finds a node by what it says",
        "BM25 keyword search over every field a node's prose is written in, ranked, optionally \
         scoped to one node type, so a person can map their own words to the node they mean.",
    ),
    (
        "req:a-session-keeps-one-seat",
        "One client session keeps one seat for its whole life",
        "A seat is a durable name for a session, minted once and reused, so the claims a session \
         makes are attributed to it and read gone once the session ends or crashes.",
    ),
    (
        "cap:the-hosted-server-checks-every-callers-token",
        "The hosted server checks every caller's token before it answers",
        "Every request carries a bearer token whose audience must name this server; a token for \
         another resource is refused, and the refusal names the audience that was expected.",
    ),
    (
        "dec:the-saved-design-is-one-file-per-item",
        "The saved design is one file per node and one per edge",
        "So that git's ordinary merge merges two people's design changes: two people editing \
         different parts of the design touch different files and merge with no conflict.",
    ),
    (
        "req:a-check-that-never-ran-is-not-evidence",
        "A check that never ran is not evidence that anything works",
        "A verification left planned counts as no confirmation, and delivery counts a capability \
         only when its own check is passing, never when somebody typed that it passed.",
    ),
    (
        "cap:a-reply-says-what-it-withheld",
        "A reply that outgrows the budget says what it withheld",
        "Over the budget, prose goes first, then rows from the tail; every count and id stays, \
         and the reply names what was withheld and how to read the rest.",
    ),
    (
        "req:the-gate-fails-the-build-on-drift",
        "The design gate fails the build when the code drifts from the design",
        "Each registered file's checksum is compared with the one the design accepted; a file \
         that changed with no change event accepting it fails the gate and is named.",
    ),
    (
        "cap:an-empty-answer-says-which-empty-it-is",
        "An empty answer says which empty it is",
        "Nothing matched in six thousand and nothing matched in zero were the same reply; every \
         list now says how many it searched, so an absence is never mistaken for a coverage.",
    ),
    (
        "dec:skills-are-served-not-installed",
        "Skills are served by the server rather than installed in the project",
        "A skill read from the server always matches the running version, and nothing in the \
         consumer's repository goes stale when the server is upgraded.",
    ),
    (
        "req:the-store-refuses-a-second-writer",
        "The store on disk refuses a second writer while a server holds it",
        "One process holds the store's lock; a second that tries to open it for writing is \
         refused with the holder named, and read-only tools work from a snapshot meanwhile.",
    ),
    (
        "cap:the-installer-writes-the-mcp-configuration",
        "The installer writes the project's MCP configuration and its skills",
        "Run once in a consumer project, it installs the design environment only, never a source \
         layout or a build file, and says what it wrote.",
    ),
];

fn kind_of(id: &str) -> &'static str {
    match id.split(':').next() {
        Some("req") => "Requirement",
        Some("cap") => "Capability",
        Some("dec") => "Decision",
        other => panic!("no seed kind for {other:?}"),
    }
}

/// Write one seed through its capture tool, as the field does, naming every
/// seed of its type written so far as distinct so that the SEEDING is never
/// the thing under test.
async fn seed(svc: &ReflowService, written: &mut Vec<String>, id: &str, name: &str, body: &str) {
    let same: Vec<String> = written
        .iter()
        .filter(|w| kind_of(w) == kind_of(id))
        .cloned()
        .collect();
    match kind_of(id) {
        "Requirement" => {
            let req: RequirementReq = serde_json::from_value(json!({
                "id": id, "name": name, "statement": body, "distinct_from": same,
            }))
            .expect("requirement request");
            j!(svc.add_requirement(Parameters(req.into())));
        }
        "Capability" => {
            let req: CapabilityReq = serde_json::from_value(json!({
                "id": id, "name": name, "description": body, "distinct_from": same,
            }))
            .expect("capability request");
            j!(svc.add_capability(Parameters(req.into())));
        }
        _ => {
            let req: DecisionReq = serde_json::from_value(json!({
                "id": id, "name": name, "decision": body, "distinct_from": same,
            }))
            .expect("decision request");
            j!(svc.add_decision(Parameters(req.into())));
        }
    }
    written.push(id.to_string());
}

/// The design as it stood at 08:55 on 2026-11-01, in miniature.
async fn the_design_before_the_write() -> (ReflowService, Vec<String>) {
    let svc = ReflowService::in_memory().expect("in-memory service");
    let mut written = Vec::new();
    for (id, name, body) in UNRELATED.iter().chain(NEIGHBOURS.iter()) {
        seed(&svc, &mut written, id, name, body).await;
    }
    let dec: DecisionReq = serde_json::from_value(json!({
        "id": DEC_ID, "name": DEC_NAME, "decision": DEC_BODY, "rationale": DEC_RATIONALE,
        "kind": "exploratory",
        "no_relation_note": "Seeded as the design held it; its relations are not under test.",
        "distinct_from": written.iter().filter(|w| kind_of(w) == "Decision").collect::<Vec<_>>(),
    }))
    .expect("decision request");
    j!(svc.add_decision(Parameters(dec.into())));
    written.push(DEC_ID.into());
    seed(&svc, &mut written, CAP_ID, CAP_NAME, CAP_DESCRIPTION).await;
    (svc, written)
}

/// The window the guard reads, as `id score` lines, so a failure shows WHERE
/// the cut fell rather than only that it fell wrong.
async fn the_window(svc: &ReflowService) -> String {
    let req: SearchDesignReq = serde_json::from_value(json!({
        "query": format!("{PROBE_NAME} {PROBE_STATEMENT}"), "limit": 12,
    }))
    .expect("search request");
    let out = j!(svc.search_design(Parameters(req)));
    out["hits"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|h| format!("{:.1} {}", h["score"].as_f64().unwrap_or(0.0), h["node_id"]))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

fn named(out: &Value) -> Vec<String> {
    out["search_first"]["near_matches"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| m["node_id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
#[ignore = "known gap: no cut on today's score clears the bar \
            (dec:how-the-near-match-guard-tells-a-reworded-duplicate-from-a-neighbour); \
            switched on when a measured signal does"]
async fn a_requirement_restating_a_verified_capability_is_told_it_exists() {
    let (svc, written) = the_design_before_the_write().await;
    // The same-type seeds are named distinct so the write cannot be REFUSED
    // on them: the refusal is covered elsewhere, and what is under test here
    // is the report the write carries.
    let same: Vec<&String> = written
        .iter()
        .filter(|w| kind_of(w) == "Requirement")
        .collect();
    let req: RequirementReq = serde_json::from_value(json!({
        "id": PROBE_ID, "name": PROBE_NAME, "statement": PROBE_STATEMENT, "distinct_from": same,
    }))
    .expect("requirement request");
    let out = j!(svc.add_requirement(Parameters(req.into())));
    let shown = named(&out);
    let window = the_window(&svc).await;

    assert!(
        shown.iter().any(|id| id == CAP_ID),
        "the requirement restates {CAP_ID} in other words, and the guard must name it. It \
         named {shown:?}. See fact:root-cause-the-near-match-guard-stayed-silent-on-a-\
         requirement-that-restates-a-verified-capability-2026-11-01. The window:\n{window}"
    );
    assert!(
        shown.iter().any(|id| id == DEC_ID),
        "and the decision that settled the same thing must be named too: {shown:?}. The \
         window:\n{window}"
    );
    let unrelated: Vec<&String> = shown
        .iter()
        .filter(|id| UNRELATED.iter().any(|(u, _, _)| u == id))
        .collect();
    assert!(
        unrelated.is_empty(),
        "catching the reworded pair must not cost a false alarm: unrelated work was named as \
         near: {unrelated:?}"
    );
    assert!(
        shown.len() <= 6,
        "the report stays bounded: {} named",
        shown.len()
    );
}

/// THE COUNTERWEIGHT, in the same design: an unrelated requirement written
/// beside the CHANGELOG neighbourhood carries no near-match at all.
#[tokio::test]
async fn an_unrelated_requirement_beside_the_pair_is_told_nothing() {
    let (svc, _) = the_design_before_the_write().await;
    let req: RequirementReq = serde_json::from_value(json!({
        "id": "req:a-hosted-design-is-backed-up-nightly",
        "name": "A hosted design is backed up every night and the restore is exercised",
        "statement": "Each night the hosted store is copied to a second machine, and once a \
                      quarter the copy is restored onto a scratch server and read back, so a \
                      lost disk costs at most one day of work.",
    }))
    .expect("requirement request");
    let out = j!(svc.add_requirement(Parameters(req.into())));
    assert!(
        out.get("search_first").is_none(),
        "an unrelated requirement must carry no near-match block, got {:?}",
        named(&out)
    );
}
