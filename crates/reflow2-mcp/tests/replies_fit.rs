//! Replies fit (PR B): a reply asked at N arrives at N or less, keeps every
//! count, and names what it withheld — for the read-back of a proof, a skill
//! with the lessons a design holds for it, and a change's blast radius.
//!
//! - `fact:root-cause-a-reasoning-read-back-outgrows-the-client-on-repeated-premise-prose-and-text-added-after-the-budget-2026-10-22`
//!   (item 1: premise rows stop repeating prose);
//! - `fact:root-cause-get-skill-attaches-every-lesson-unbounded-so-six-skills-outgrow-a-chat-reply-on-reflow2s-design-2026-10-21`
//!   (item 3: get_skill takes a budget);
//! - `fact:the-rocket-equation-rung-ran-on-0-90-0-and-a-unit-check-caught-a-nasa-formula-but-reflow2-checks-no-units-2026-10-23`,
//!   finding (b) (item 4: propagate_from can be bounded).
//!
//! The proof is the heat-equation rung's committed fixture: 126 nodes, the
//! theorem `req:thm-uniqueness` resting on 50 claims through 126 premise rows.

use std::collections::BTreeMap;

use reflow2_mcp::reply_budget::chars_of;
use reflow2_mcp::service::*;
use rmcp::handler::server::wrapper::Parameters;
use serde_json::{Value, json};

const HEAT: &str = include_str!("../../reflow2-core/tests/fixtures/proof_trial_heat_equation.json");
const THEOREM: &str = "req:thm-uniqueness";
const CITED: &str = reflow2_core::inference::CITED_UNVERIFIED_WHY;

async fn heat() -> (ReflowService, BTreeMap<String, Value>) {
    let doc: Value = serde_json::from_str(HEAT).expect("the fixture parses");
    let s = ReflowService::in_memory().expect("in-memory service");
    s.import_graph(Parameters(ImportGraphReq {
        document: doc.as_object().cloned(),
        path: None,
        accept_newer: None,
        merge: None,
    }))
    .await
    .expect("the heat rung imports");
    let props = doc["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| {
            (
                n["node_id"].as_str().unwrap().to_string(),
                n["properties"].clone(),
            )
        })
        .collect();
    (s, props)
}

async fn read(s: &ReflowService, args: Value) -> Value {
    let req: ReasoningReportReq = serde_json::from_value(args).expect("a request");
    s.reasoning_report(Parameters(req))
        .await
        .expect("the read-back answers")
        .structured_content
        .expect("structured")
}

/// How many times `needle` is said anywhere in `v`'s strings.
fn said(v: &Value, needle: &str) -> usize {
    match v {
        Value::String(s) => s.matches(needle).count(),
        Value::Array(a) => a.iter().map(|x| said(x, needle)).sum(),
        Value::Object(o) => o.values().map(|x| said(x, needle)).sum(),
        _ => 0,
    }
}

/// ⭐ Item 1. At every budget a page fits (one whole claim is its floor: the
/// theorem's own row is about 11,000 characters, so 8,000 cannot be met, as on
/// 0.90.0), every claim it returns is whole
/// (its words, each step's rule and warrant, each premise's name), every
/// premise's full reason is on the page ONCE — in `premise_whys`, or in its
/// own claim row when it is a claim on the page — and the citation advisory
/// is said once, naming every id it applies to.
#[tokio::test]
async fn a_premise_reason_is_said_once_and_every_claim_stays_whole() {
    let (s, props) = heat().await;
    for budget in [20_000usize, 30_000, 60_000, 400_000] {
        let r = read(
            &s,
            json!({"claim": THEOREM, "budget_chars": budget, "limit": 200}),
        )
        .await;
        let size = chars_of(&r);
        assert!(size <= budget, "asked at {budget}, answered {size}");
        assert_eq!(r["total_claims"], 50, "the count is kept at {budget}");

        let claims = r["claims"].as_array().expect("claims");
        let on_page: BTreeMap<&str, &str> = claims
            .iter()
            .map(|c| (c["id"].as_str().unwrap(), c["why"].as_str().unwrap()))
            .collect();
        let whys = r["premise_whys"].as_object().cloned().unwrap_or_default();
        for c in claims {
            let id = c["id"].as_str().unwrap();
            if let Some(words) = props[id]["statement"].as_str() {
                assert_eq!(c["says"].as_str(), Some(words), "{id} keeps its words");
            }
            for step in c["support"].as_array().into_iter().flatten() {
                let inf = step["inference"].as_str().unwrap();
                if let Some(w) = props[inf]["warrant"].as_str() {
                    assert_eq!(step["warrant"].as_str(), Some(w), "{inf} keeps its warrant");
                }
                for p in step["premises"].as_array().unwrap() {
                    let pid = p["id"].as_str().unwrap();
                    assert_eq!(p["name"], props[pid]["name"], "{pid} keeps its name");
                    // The row keeps its `why`: its reason's leading clause.
                    let reads = p["why"].as_str().expect("a row says its reason in short");
                    assert!(!reads.is_empty() && !reads.ends_with('…'), "{p}");
                    // Its full reason is on the page: in premise_whys, or in
                    // its own row, or it is its own reading.
                    let cited = whys.contains_key(pid);
                    let in_row = on_page.contains_key(pid);
                    assert!(
                        cited || in_row || reads.len() <= 100,
                        "{pid}'s full reason must be on the page"
                    );
                    assert!(
                        !(cited
                            && in_row
                            && whys[pid].as_str().is_some_and(|w| on_page[pid].contains(w))),
                        "{pid}'s reason is said in its row and again in premise_whys"
                    );
                }
            }
        }
        let advisory = said(&r, "Register the source");
        let cites = said(&r, "advisories`: cited_unverified");
        if cites > 0 {
            assert_eq!(advisory, 1, "the advisory is said once at {budget}");
            let applies = &r["advisories"][0]["applies_to"];
            assert!(
                applies.as_array().is_some_and(|a| !a.is_empty()),
                "it names the ids it applies to: {applies}"
            );
        }
        assert_eq!(said(&r, CITED), advisory.min(1), "the whole sentence, once");
    }
}

/// The measured gain on the fixture: the whole walk in fewer characters, and
/// a 60,000 page holding more whole claims than the 18 it held before.
#[tokio::test]
async fn the_whole_walk_costs_less_and_a_page_holds_more_claims() {
    let (s, _) = heat().await;
    let all = read(
        &s,
        json!({"claim": THEOREM, "budget_chars": 400_000, "limit": 200}),
    )
    .await;
    let size = chars_of(&all);
    // 133,198 characters before (0.90.0); 23,332 of them premise-row reasons.
    assert!(size < 120_000, "the whole walk answered {size}");
    let page = read(&s, json!({"claim": THEOREM, "budget_chars": 60_000})).await;
    let n = page["claims"].as_array().unwrap().len();
    assert!(n > 18, "a 60,000 page held 18 whole claims before; now {n}");
    eprintln!("heat rung: whole walk {size} characters; a 60,000 page holds {n} claims");
}

/// ⭐ Item 3. get_skill holds the lessons a design keeps for the step to its
/// budget: the body WHOLE, the lessons ranked newest first and withheld whole
/// from the end, how many withheld and which (by id) named, the total kept.
/// Before, it took only `name` and attached every lesson: root-cause was
/// 54,054 characters on reflow2's own design.
#[tokio::test]
async fn get_skill_keeps_its_body_whole_and_names_the_lessons_it_withheld() {
    use reflow2_mcp::tools::skills_tools::GetSkillReq;
    let s = ReflowService::in_memory().expect("service");
    s.add_project(Parameters(ProjectReq {
        id: "proj:p".into(),
        name: Some("P".into()),
        description: None,
        decomposition_levels: None,
        status: None,
    }))
    .await
    .expect("project");
    for i in 0..40 {
        s.record_finding(Parameters(RecordFindingReq {
            id: format!("fact:lesson-{i:02}"),
            subject_id: "proj:p".into(),
            node_type: None,
            name: Some(format!("lesson {i}")),
            statement: Some("what bit us, and what to do instead. ".repeat(14)),
            fact_type: Some("finding".into()),
            basis: None,
            confidence: None,
            source: None,
            value: None,
            valid_from: Some(format!("2026-09-{:02}", 1 + i % 28)),
            valid_to: None,
            caused_by: None,
            caused_by_type: None,
            cause_evidence: None,
            steps: Some(vec!["root-cause".into()]),
        }))
        .await
        .expect("a lesson");
    }
    let ask = |budget: Option<usize>| {
        s.get_skill(Parameters(GetSkillReq {
            name: "root-cause".into(),
            budget_chars: budget,
        }))
    };
    let whole = ask(Some(1_000_000))
        .await
        .unwrap()
        .structured_content
        .unwrap();
    assert_eq!(whole["lessons"]["items"].as_array().unwrap().len(), 40);
    let body = whole["body"].as_str().unwrap().to_string();
    for budget in [None, Some(25_000usize), Some(1_000)] {
        let r = ask(budget).await.unwrap().structured_content.unwrap();
        let n = budget.unwrap_or(30_000);
        assert_eq!(r["body"].as_str(), Some(body.as_str()), "the body is whole");
        let lessons = &r["lessons"];
        let shown = lessons["items"].as_array().unwrap().len();
        let withheld = lessons["withheld_ids"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0);
        assert_eq!(
            shown + withheld,
            40,
            "every lesson shown or named: {lessons}"
        );
        if chars_of(&whole) > n {
            assert_eq!(lessons["total"], 40, "the count is kept");
            assert!(withheld > 0);
            assert!(
                lessons["withheld_note"]
                    .as_str()
                    .unwrap()
                    .contains("get_node"),
                "it says how to get the rest"
            );
        }
        // Fits, unless the body alone does not: then every lesson is withheld
        // and the body still comes whole.
        if shown > 0 {
            assert!(chars_of(&r) <= n, "asked at {n}, answered {}", chars_of(&r));
        }
        // Newest first: what is shown is the head of the ranking.
        let ids: Vec<&str> = lessons["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| l["id"].as_str().unwrap())
            .collect();
        let head: Vec<&str> = whole["lessons"]["items"].as_array().unwrap()[..shown]
            .iter()
            .map(|l| l["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, head, "the ranking's head is shown");
    }
}

/// ⭐ Item 4. propagate_from takes budget_chars: whole rows withheld from the
/// end of the inference walk, each named by id, every count kept. Reproduced
/// first on this fixture: at max_depth 2 the radius from one cited lemma was
/// 41,073 characters (38,564 of them the inference walk), with no bound.
#[tokio::test]
async fn propagate_from_is_held_to_its_budget_naming_every_step_it_withheld() {
    let (s, _) = heat().await;
    let ask = |budget: Option<usize>, full: Option<bool>| {
        s.propagate_from(Parameters(PropagateFromReq {
            seed_ids: vec!["req:cite-continuity-and-derivative-algebra".into()],
            max_depth: Some(2),
            full,
            arriving_from: None,
            interfaces: None,
            budget_chars: budget,
        }))
    };
    let whole = ask(Some(1_000_000), None)
        .await
        .unwrap()
        .structured_content
        .unwrap();
    let steps = whole["inferences"].as_array().unwrap().len();
    assert!(
        chars_of(&whole) > 30_000,
        "the radius outgrows the default: {}",
        chars_of(&whole)
    );
    for (budget, full) in [
        (None, None),
        (Some(10_000usize), None),
        (Some(30_000), Some(true)),
    ] {
        let r = ask(budget, full).await.unwrap().structured_content.unwrap();
        let n = budget.unwrap_or(30_000);
        assert!(chars_of(&r) <= n, "asked at {n}, answered {}", chars_of(&r));
        if full.is_none() {
            assert_eq!(
                r["total_impacted"], whole["total_impacted"],
                "counts are kept"
            );
            assert_eq!(r["counts_by_distance"], whole["counts_by_distance"]);
        }
        let shortened = r["budget"]["lists_shortened"].as_array().expect("named");
        let inf = shortened
            .iter()
            .find(|l| l["path"] == "inferences")
            .expect("the inference walk gave");
        assert_eq!(inf["total"], steps);
        let shown = r["inferences"].as_array().unwrap();
        assert_eq!(inf["shown"], shown.len());
        assert_eq!(
            shown.len() + inf["withheld"].as_array().unwrap().len(),
            steps,
            "every step is shown or named"
        );
        // A row shown is whole: the same as in the unbounded walk.
        assert_eq!(shown[0], whole["inferences"][0]);
    }
}
