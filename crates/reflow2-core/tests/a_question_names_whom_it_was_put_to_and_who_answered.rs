//! The core half of `cap:a-question-names-whom-it-was-put-to-and-who-answered`:
//! a Question records its addressee, its batch number and its evidence; an
//! answer records its answerer and draws ANSWERS in the same call; and one
//! person's open questions read back as their batch.
//!
//! Root causes pinned:
//! `fact:root-cause-answering-takes-two-calls-and-records-no-answerer-because-the-answers-edge-landed-as-its-own-tool-2026-09-29`
//! and `fact:root-cause-an-owner-outside-the-chat-cannot-read-what-it-is-asked-to-approve-2026-09-29`.
//! The served surface is pinned in reflow2-mcp's
//! `a_question_is_put_to_a_person_and_records_who_answered.rs`.

use reflow2_core::nodes::{Props, edge, node};
use reflow2_core::{Answering, AskedQuestion, DesignGraph};

/// A design with anchored gaps, an owner, a finding to cite and a Decision an
/// answer can become. Returns the graph and the first three anchored gap ids.
fn design() -> (DesignGraph, Vec<(String, Vec<String>)>) {
    let mut g = DesignGraph::open_in_memory().unwrap();
    g.add_project("proj:p", "P").unwrap();
    g.add_requirement("req:r", "R", "Must do x.").unwrap();
    g.create_node(
        node::CONTRIBUTOR,
        "who:owner",
        Props::new().set("name", "The owner").set("kind", "person"),
    )
    .unwrap();
    g.create_node(
        node::CONTRIBUTOR,
        "who:delegate",
        Props::new()
            .set("name", "The delegate")
            .set("kind", "person"),
    )
    .unwrap();
    g.add_decision("dec:became", "What it became", "Thing x is done so.", None)
        .unwrap();
    let mut gaps: Vec<(String, Vec<String>)> = g
        .detect_gaps()
        .unwrap()
        .into_iter()
        .filter(|gap| !gap.affected_ids.is_empty())
        .map(|gap| (gap.id, gap.affected_ids))
        .collect();
    gaps.sort();
    assert!(gaps.len() >= 3, "three anchored gaps, got {gaps:?}");
    (g, gaps)
}

fn prop(g: &DesignGraph, id: &str, key: &str) -> Option<String> {
    g.get_node(node::QUESTION, id)
        .unwrap()
        .and_then(|n| n.properties.get(key).cloned())
        .and_then(|v| {
            v.as_str()
                .map(str::to_string)
                .or(v.as_i64().map(|i| i.to_string()))
        })
}

fn ask(
    g: &mut DesignGraph,
    gap: &(String, Vec<String>),
    asked_of: Option<&str>,
    batch: Option<&str>,
    evidence: &[String],
) -> String {
    g.record_asked_question(
        &gap.0,
        &gap.1,
        "Is this what you meant?",
        AskedQuestion {
            asked_of,
            batch,
            evidence,
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn a_question_records_whom_it_was_put_to_and_its_number_in_the_batch() {
    let (mut g, gaps) = design();
    let q1 = ask(&mut g, &gaps[0], Some("who:owner"), Some("round-1"), &[]);
    let q2 = ask(&mut g, &gaps[1], Some("who:owner"), Some("round-1"), &[]);
    assert_eq!(prop(&g, &q1, "asked_of").as_deref(), Some("who:owner"));
    assert_eq!(prop(&g, &q1, "batch_position").as_deref(), Some("1"));
    assert_eq!(prop(&g, &q2, "batch_position").as_deref(), Some("2"));

    // Asked again, it keeps its number and its addressee: a relay may already
    // have quoted "Q1".
    ask(&mut g, &gaps[0], None, None, &[]);
    assert_eq!(prop(&g, &q1, "batch_position").as_deref(), Some("1"));
    assert_eq!(prop(&g, &q1, "asked_of").as_deref(), Some("who:owner"));
}

#[test]
fn an_addressee_or_evidence_that_resolves_to_nothing_is_refused_before_anything_is_written() {
    let (mut g, gaps) = design();
    let r = g.record_asked_question(
        &gaps[0].0,
        &gaps[0].1,
        "Q?",
        AskedQuestion {
            asked_of: Some("who:ghost"),
            ..Default::default()
        },
    );
    assert!(r.is_err(), "an unknown addressee is refused");
    let ev = vec!["fact:ghost".to_string()];
    let r = g.record_asked_question(
        &gaps[0].0,
        &gaps[0].1,
        "Q?",
        AskedQuestion {
            evidence: &ev,
            ..Default::default()
        },
    );
    assert!(r.unwrap_err().to_string().contains("fact:ghost"));
    assert!(
        g.scan_nodes(node::QUESTION).unwrap().is_empty(),
        "nothing was written"
    );
}

#[test]
fn answering_records_the_answerer_and_draws_answers_in_the_same_call() {
    let (mut g, gaps) = design();
    let q = ask(&mut g, &gaps[0], Some("who:owner"), None, &[]);
    assert!(
        g.answer_question_by(
            &q,
            "Yes, done so.",
            Answering {
                answered_by: Some("who:delegate"),
                answered_at: Some("2026-09-29"),
                record: Some((node::DECISION, "dec:became")),
                note: Some("as ruled"),
            },
        )
        .unwrap()
    );
    assert_eq!(prop(&g, &q, "answered_by").as_deref(), Some("who:delegate"));
    assert_eq!(
        prop(&g, &q, "asked_of").as_deref(),
        Some("who:owner"),
        "whom it was put to and who answered are two facts, both kept"
    );
    let inbound = g.incoming(&q, Some(edge::ANSWERS)).unwrap();
    assert_eq!(inbound.len(), 1);
    assert_eq!(inbound[0].from_id, "dec:became");

    // A later answer naming nobody does not inherit the delegate's name.
    g.answer_question(&q, "Actually, no.").unwrap();
    assert_eq!(prop(&g, &q, "answered_by"), None);
    assert_eq!(prop(&g, &q, "answered_at"), None);
}

#[test]
fn an_unknown_answerer_or_record_leaves_the_question_as_it_was() {
    let (mut g, gaps) = design();
    let q = ask(&mut g, &gaps[0], None, None, &[]);
    for by in [
        Answering {
            answered_by: Some("who:ghost"),
            ..Default::default()
        },
        Answering {
            record: Some((node::DECISION, "dec:ghost")),
            ..Default::default()
        },
    ] {
        assert!(g.answer_question_by(&q, "Yes.", by).is_err());
        assert_eq!(prop(&g, &q, "status").as_deref(), Some("asked"));
    }
}

#[test]
fn one_persons_open_questions_read_as_their_batch_with_evidence_links() {
    let (mut g, gaps) = design();
    g.create_node(
        node::TEMPORAL_FACT,
        "fact:measured",
        Props::new()
            .set("name", "Half what was assumed")
            .set("statement", "Measured: half what was assumed.")
            .set("subject_id", "req:r")
            .set("fact_type", "finding"),
    )
    .unwrap();
    let ev = vec!["fact:measured".to_string()];
    let q2 = ask(&mut g, &gaps[1], Some("who:owner"), Some("round-1"), &[]);
    let q1 = ask(&mut g, &gaps[0], Some("who:owner"), Some("round-1"), &ev);
    ask(&mut g, &gaps[2], None, None, &[]);

    let mine = g.open_questions_for(Some("who:owner")).unwrap();
    let ids: Vec<&str> = mine.iter().map(|q| q.question_id.as_str()).collect();
    assert_eq!(
        ids,
        vec![q2.as_str(), q1.as_str()],
        "batch order, not id order"
    );
    let links: Vec<&str> = mine[1].evidence.iter().map(|e| e.id.as_str()).collect();
    assert!(links.contains(&"fact:measured"), "{links:?}");
    let fact = mine[1]
        .evidence
        .iter()
        .find(|e| e.id == "fact:measured")
        .unwrap();
    assert_eq!(fact.node_type, "TemporalFact");
    assert_eq!(fact.name, "Half what was assumed");

    let all = g.open_questions().unwrap();
    assert_eq!(all.len(), 3);
    let unaddressed = all.iter().find(|q| q.asked_of.is_none()).unwrap();
    let row = serde_json::to_value(unaddressed).unwrap();
    assert!(
        row.as_object().unwrap().contains_key("asked_of") && row["asked_of"].is_null(),
        "an unaddressed question says so: {row}"
    );

    assert!(
        g.open_questions_for(Some("who:ghost")).is_err(),
        "an unknown addressee is refused, never answered empty"
    );
}

#[test]
fn loop_status_for_a_person_lists_the_questions_put_to_them_and_is_not_clean() {
    let (mut g, gaps) = design();
    ask(&mut g, &gaps[0], Some("who:owner"), Some("round-1"), &[]);
    let ls = g.loop_status_for(Some("who:owner")).unwrap();
    assert_eq!(ls.questions_put_to_them.len(), 1);
    assert!(!ls.clean);
    assert!(
        ls.next
            .iter()
            .any(|l| l.contains("put to who:owner by name"))
    );

    let other = g.loop_status_for(Some("who:delegate")).unwrap();
    assert!(other.questions_put_to_them.is_empty());
    assert!(
        g.loop_status().unwrap().questions_put_to_them.is_empty(),
        "unscoped, nothing is attributed to anyone"
    );
}

#[test]
fn an_answer_recorded_through_an_agent_names_the_agent_beside_the_answerer() {
    let (mut g, gaps) = design();
    g.create_node(
        node::CONTRIBUTOR,
        "who:designer",
        Props::new()
            .set("name", "The designer agent")
            .set("kind", "automated_agent"),
    )
    .unwrap();
    let q = ask(&mut g, &gaps[0], Some("who:owner"), None, &[]);
    g.begin_acting(reflow2_core::acting::Acting {
        agent: "who:designer".into(),
        route: "session".into(),
    })
    .unwrap();
    g.answer_question_by(
        &q,
        "Relayed: yes.",
        Answering {
            answered_by: Some("who:owner"),
            ..Default::default()
        },
    )
    .unwrap();
    g.end_acting();
    assert_eq!(prop(&g, &q, "answered_by").as_deref(), Some("who:owner"));
    assert_eq!(
        prop(&g, &q, "answered_via").as_deref(),
        Some("who:designer")
    );
    assert!(
        g.outgoing("who:designer", Some(edge::ACTS_FOR))
            .unwrap()
            .iter()
            .any(|e| e.to_id == "who:owner"),
        "the agent is drawn acting for the answerer"
    );
    let row = g.open_questions().unwrap().remove(0);
    assert_eq!(row.answered_via.as_deref(), Some("who:designer"));
}
