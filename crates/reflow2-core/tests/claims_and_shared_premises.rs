//! CLAIMS, SUPERSEDED MIRRORS AND SHARED PREMISES READ RIGHT — PR A2, the
//! second half of "proofs and arguments read right", queued on Anthony's
//! word of 2026-10-22 ("Go with your recommendations (for all the open
//! decisions)").
//!
//! 1. **A superseded mirrored judgment is kept**
//!    (`req:a-superseded-mirrored-judgment-is-kept-and-what-rested-on-it-
//!    reads-reason-lost`, parts (1)–(5)). Measured on the split-capstone
//!    trial's two released surfaces, KESTREL-ASSESS v1 and v2 (FICTIONAL),
//!    carried by the courier on 2026-10-22: the refresh DELETED v1's four
//!    judgments and every local edge into them, and nothing read a lost
//!    reason.
//! 2. **A claim reads "claimed"**
//!    (`req:a-claim-made-by-a-named-party-and-not-yet-verified-reads-
//!    claimed`): on the Navier–Stokes map (ad65e4983833f39f) OpenAI's (C)
//!    could only read "unproven"; on the four colour rung (7d851f5be7cca068)
//!    "Kempe's proof, accepted 1879–1890" lived only in dated notes nobody
//!    read.
//! 3. **The independence check compares premises**: three trials (the
//!    Navier–Stokes map's finding (b), the four colour rung's (e), the
//!    Gettier rung's (a)) read two lines "independent" though both rest on a
//!    premise neither derives; and `independent_of: []` did not clear the
//!    statement.

use reflow2_core::DesignGraph;
use reflow2_core::export::GraphExport;
use reflow2_core::foundation::core::Value;
use reflow2_core::inference::InferenceSpec;
use reflow2_core::nodes::Props;
use reflow2_core::propagate::PropagateOptions;
use serde_json::Value as Json;

const NAVIER_STOKES: &str = include_str!("fixtures/proof_map_navier_stokes.json");
const FOUR_COLOUR: &str = include_str!("fixtures/proof_trial_four_colour.json");
const GETTIER: &str = include_str!("fixtures/argument_trial_gettier.json");
const HEAT: &str = include_str!("fixtures/proof_trial_heat_equation.json");
const ARROW: &str = include_str!("fixtures/proof_trial_arrow.json");
const LOGIC: &str = include_str!("fixtures/proof_trial_logic_rung.json");
const KESTREL_V1: &str = include_str!("fixtures/split_capstone_kestrel_assess_v1.surface.json");
const KESTREL_V2: &str = include_str!("fixtures/split_capstone_kestrel_assess_v2.surface.json");
const PROOF_SKILL: &str = include_str!("../../../getting-started/skills/design-a-proof/SKILL.md");
const ARGUMENT_SKILL: &str =
    include_str!("../../../getting-started/skills/design-an-argument/SKILL.md");

fn load(doc: &str) -> DesignGraph {
    let doc: GraphExport = serde_json::from_str(doc).expect("fixture parses");
    let mut g = DesignGraph::open_in_memory().expect("in-memory graph");
    g.import_graph(&doc).expect("fixture imports");
    g
}

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("in-memory graph")
}

fn json<T: serde::Serialize>(v: T) -> Json {
    serde_json::to_value(v).expect("serializes")
}

fn type_of(g: &DesignGraph, id: &str) -> String {
    let held = g.node_types_holding(id).expect("types holding");
    assert_eq!(held.len(), 1, "{id}: {held:?}");
    held[0].clone()
}

fn set(g: &mut DesignGraph, id: &str, key: &str, value: Value) {
    let ty = type_of(g, id);
    g.upsert_node(&ty, id, Props::new().set(key, value))
        .unwrap_or_else(|e| panic!("{id}.{key}: {e}"));
}

fn list(items: &[&str]) -> Value {
    Value::List(items.iter().map(|s| Value::String(s.to_string())).collect())
}

fn strs(v: &Json) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn independence(g: &DesignGraph, step: &str) -> Vec<Json> {
    g.independence_of(step)
        .unwrap()
        .into_iter()
        .map(json)
        .collect()
}

fn claim(g: &DesignGraph, id: &str) -> Json {
    let r = json(g.reasoning_report(Some(id), Some(500)).unwrap());
    r["claims"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .cloned()
        .unwrap_or_else(|| panic!("{id} not in its own read-back"))
}

fn proof_status(g: &DesignGraph, id: &str) -> Json {
    json(g.proof_reading(id).unwrap())["status"].clone()
}

fn statement_reading(g: &DesignGraph, id: &str) -> Json {
    json(g.statement_reading(id).unwrap())
}

// ─── 3. The independence check compares premises ──────────────────────────

/// FAILS ON v0.90.0 (the Gettier rung's finding (a), confirmed by the
/// observer): `inf:not-sufficient-via-case-2`, stated independent of case 1,
/// read {"holds": true, "shared_steps": [], "why": "the two lines share no
/// step …"}, though both cases rest on Gettier's two points, `req:g-
/// fallibility` and `req:g-closure` — propositions taken as given, neither a
/// cited result nor evidence, so the check never looked at them.
#[test]
fn two_gettier_cases_resting_on_both_of_gettiers_points_are_not_independent() {
    let g = load(GETTIER);
    let ind = independence(&g, "inf:not-sufficient-via-case-2");
    assert_eq!(ind.len(), 1);
    let i = &ind[0];
    assert_eq!(i["of"], "inf:not-sufficient-via-case-1");
    assert_eq!(i["holds"], false, "{i:#}");
    assert_eq!(
        strs(&i["shared_premises"]),
        vec!["req:g-closure", "req:g-fallibility"],
        "{i:#}"
    );
    let why = i["why"].as_str().unwrap();
    assert!(why.contains("req:g-closure") && why.contains("req:g-fallibility"));
    // Each case's own scenario is its own: never named as shared.
    assert!(!why.contains("con:case-1-coins"), "{why}");
    // It is put to the owner, naming the premises.
    let gaps: Vec<Json> = g
        .detect_gaps()
        .unwrap()
        .into_iter()
        .map(json)
        .filter(|x| {
            x["gap_source"] == "reasoning_not_as_stated"
                && x["title"]
                    .as_str()
                    .is_some_and(|t| t.contains("stated independent"))
        })
        .collect();
    assert_eq!(gaps.len(), 1, "{gaps:#?}");
    assert!(strs(&gaps[0]["affected_ids"]).contains(&"req:g-closure".to_string()));
    // Naming them in `independent_except` is the owner's way to say "apart
    // from Gettier's two points": then it holds.
    let mut g = g;
    set(
        &mut g,
        "inf:not-sufficient-via-case-2",
        "independent_except",
        list(&["req:g-closure", "req:g-fallibility"]),
    );
    let i = &independence(&g, "inf:not-sufficient-via-case-2")[0];
    assert_eq!(i["holds"], true, "{i:#}");
}

/// FAILS ON v0.90.0 (the four colour rung's finding (e)): the Coq route,
/// stated independent of RSST's, read "share no step" and named nothing,
/// though both rest on the 633 configurations
/// (`con:rsst-633-configurations`). The configurations are recorded as a
/// DEFINITION, and a definition is vocabulary, which leaves two lines
/// independent: the reading names it in `shared_setting`. Recorded as what it
/// is — data, a proposition taken as given — it is a shared premise, and the
/// statement reads false.
#[test]
fn the_coq_and_rsst_routes_name_the_633_configurations() {
    let mut g = load(FOUR_COLOUR);
    let ind = independence(&g, "inf:coq-2005-route");
    let rsst = ind
        .iter()
        .find(|i| i["of"] == "inf:rsst-1997-route")
        .expect("coq is stated independent of rsst");
    assert_eq!(
        strs(&rsst["shared_setting"]),
        vec!["con:rsst-633-configurations"],
        "{rsst:#}"
    );
    assert!(
        rsst["why"]
            .as_str()
            .unwrap()
            .contains("con:rsst-633-configurations")
    );
    // Recorded as data, it counts.
    set(
        &mut g,
        "con:rsst-633-configurations",
        "statement_kind",
        Value::String("proposition".into()),
    );
    let ind = independence(&g, "inf:coq-2005-route");
    let rsst = ind
        .iter()
        .find(|i| i["of"] == "inf:rsst-1997-route")
        .unwrap();
    assert_eq!(rsst["holds"], false, "{rsst:#}");
    assert_eq!(
        strs(&rsst["shared_premises"]),
        vec!["con:rsst-633-configurations"]
    );
    // Coq against Appel–Haken shares nothing, and still reads independent.
    let ah = ind
        .iter()
        .find(|i| i["of"] == "inf:ah-1976-route")
        .expect("coq is stated independent of ah");
    assert_eq!(ah["holds"], true, "{ah:#}");
    // RSST against Appel–Haken: Euler, a citation under a named lemma, stays
    // the reason, as it was (the rung's pass (6)).
    let r = &independence(&g, "inf:rsst-1997-route")[0];
    assert_eq!(r["holds"], false, "{r:#}");
    assert_eq!(strs(&r["shared_citations"]), vec!["req:thm-euler-formula"]);
}

/// THE THEOREM'S OWN SETTING IS SHARED BY EVERY PROOF OF IT: on the heat
/// rung, the Fourier and maximum-principle proofs both take up L, T > 0 at
/// their final step, and stay independent (the rung's reference), naming it.
#[test]
fn the_heat_proofs_share_only_the_theorems_own_setting() {
    let g = load(HEAT);
    let i = &independence(&g, "inf:uniqueness-fourier")[0];
    assert_eq!(i["holds"], true, "{i:#}");
    assert!(strs(&i["shared_setting"]).contains(&"req:given-L-T-positive".to_string()));
    assert!(strs(&i["shared_premises"]).is_empty());
}

/// FAILS ON v0.90.0 (the Navier–Stokes map's finding (b), reproduced): the
/// designer stated the Lean route independent of the natural-language route
/// and read "share no step … independent", though both rest on OpenAI's
/// claim that the Lean check passes (`req:claim-lean-check-passes`), the
/// second through `inf:nl-correct-via-lean`. The export carries no
/// statement any more (the designer cleared it with create_node null), so
/// the test states it again.
#[test]
fn the_lean_and_natural_language_routes_share_the_lean_check_claim() {
    let mut g = load(NAVIER_STOKES);
    set(
        &mut g,
        "inf:c-via-lean",
        "independent_of",
        list(&["inf:c-via-nl-proof"]),
    );
    let i = &independence(&g, "inf:c-via-lean")[0];
    assert_eq!(i["holds"], false, "{i:#}");
    assert_eq!(
        strs(&i["shared_premises"]),
        vec!["req:claim-lean-check-passes"],
        "{i:#}"
    );
}

/// FAILS ON v0.90.0 (the Navier–Stokes map's finding (b), second half):
/// `independent_of: []` did not clear the statement, though the field says it
/// overwrites: `add_inference` removed the key from the map it MERGED into
/// the stored node, so the stored key survived. The same held for
/// `independent_except: []`, `discharges: []`, `cases: []` and
/// `cases_exhaustive: ""`.
#[test]
fn an_empty_list_clears_a_stated_independence() {
    let mut g = load(NAVIER_STOKES);
    let spec = |of: Option<Vec<String>>, except: Option<Vec<String>>| InferenceSpec {
        id: "inf:c-via-lean".to_string(),
        rule: "all".to_string(),
        premises: vec![
            "req:claim-lean-check-passes".to_string(),
            "req:conn-lean-statement-is-c".to_string(),
        ],
        conclusion: "req:stmt-c".to_string(),
        independent_of: of,
        independent_except: except,
        ..Default::default()
    };
    g.add_inference(&spec(
        Some(vec!["inf:c-via-nl-proof".to_string()]),
        Some(vec!["req:claim-lean-check-passes".to_string()]),
    ))
    .expect("stated");
    assert_eq!(independence(&g, "inf:c-via-lean").len(), 1);
    assert_eq!(independence(&g, "inf:c-via-lean")[0]["holds"], true);
    g.add_inference(&spec(Some(Vec::new()), Some(Vec::new())))
        .expect("cleared");
    let stored = g.get_node("Inference", "inf:c-via-lean").unwrap().unwrap();
    assert!(
        !stored.properties.contains_key("independent_of"),
        "`independent_of: []` clears the statement: {:?}",
        stored.properties
    );
    assert!(!stored.properties.contains_key("independent_except"));
    assert!(independence(&g, "inf:c-via-lean").is_empty());
    // Omitting the field keeps what is stored.
    g.add_inference(&spec(Some(vec!["inf:c-via-nl-proof".to_string()]), None))
        .unwrap();
    g.add_inference(&spec(None, None)).unwrap();
    assert_eq!(independence(&g, "inf:c-via-lean").len(), 1);
}

/// THE CLASS, on every trial design that states an independence: a reading
/// that holds names no shared premise, and no premise it names is one the
/// statement names. Measured: only Gettier's reading moves (heat, Arrow,
/// four colour and the logic rung read as before).
#[test]
fn every_stated_independence_names_what_the_lines_share() {
    let mut moved: Vec<String> = Vec::new();
    for (name, doc) in [
        ("gettier", GETTIER),
        ("four-colour", FOUR_COLOUR),
        ("heat", HEAT),
        ("arrow", ARROW),
        ("logic", LOGIC),
    ] {
        let g = load(doc);
        for i in g.stated_independence().unwrap() {
            let i = json(i);
            let shared = strs(&i["shared_premises"]);
            if i["holds"] == true {
                assert!(shared.is_empty(), "{name}: {i:#}");
            }
            for p in &shared {
                assert!(
                    !strs(&i["except"]).contains(p),
                    "{name}: a named premise is never reported shared: {i:#}"
                );
            }
            if !shared.is_empty() {
                moved.push(format!("{name} {} vs {}", i["step"], i["of"]));
            }
        }
    }
    assert_eq!(
        moved,
        vec!["gettier \"inf:not-sufficient-via-case-2\" vs \"inf:not-sufficient-via-case-1\""],
        "{moved:#?}"
    );
}

// ─── 2. A claim reads "claimed", and a status over a period is read back ──

/// OpenAI's claims on the Navier–Stokes map, marked as the trial designer
/// would mark them: the natural-language proof is correct, and the Lean check
/// passes — each claimed by OpenAI, nobody's verification.
fn navier_stokes_with_openais_claims() -> DesignGraph {
    let mut g = load(NAVIER_STOKES);
    for id in ["req:claim-nl-proof-correct", "req:claim-lean-check-passes"] {
        set(&mut g, id, "claimed_by", Value::String("OpenAI".into()));
    }
    g
}

/// FAILS ON v0.90.0 (the Navier–Stokes map's finding (a)): (C), which
/// OpenAI claims to have shown, could only read "unproven" — the word for a
/// refuted claim and for one nobody tried. It reads "claimed by OpenAI,
/// unverified" wherever its truth is printed, and (A), which rests on no
/// claim, still reads open.
#[test]
fn a_claim_a_named_party_made_reads_claimed_never_unproven_or_open() {
    let bare = load(NAVIER_STOKES);
    assert_eq!(
        proof_status(&bare, "req:stmt-c"),
        "unproven",
        "measured before"
    );

    let g = navier_stokes_with_openais_claims();
    assert_eq!(proof_status(&g, "req:stmt-c"), "claimed");
    let r = statement_reading(&g, "req:stmt-c");
    assert_eq!(r["reads"], "claimed", "{r:#}");
    assert!(
        r["why"]
            .as_str()
            .unwrap()
            .contains("claimed by OpenAI, unverified"),
        "{r:#}"
    );
    let c = claim(&g, "req:stmt-c");
    assert_eq!(strs(&c["claimed_by"]), vec!["OpenAI"], "{c:#}");
    assert!(c["why"].as_str().unwrap().starts_with("claimed by OpenAI"));
    assert_eq!(c["proof_status"], "claimed");
    // Never shown: its truth stays unknown (the rung's pass (2)).
    assert_eq!(c["truth"], "unknown");
    // The claim itself, with no steps of its own, reads claimed too.
    assert_eq!(
        statement_reading(&g, "req:claim-nl-proof-correct")["reads"],
        "claimed"
    );
    // (A) rests on no claim and reads as before.
    assert_eq!(
        statement_reading(&g, "req:stmt-a")["reads"],
        statement_reading(&bare, "req:stmt-a")["reads"]
    );
    assert!(strs(&claim(&g, "req:stmt-a")["claimed_by"]).is_empty());
}

/// A WHAT-IF THAT DROPS THE CLAIMANT'S ARGUMENT returns (C) to open and
/// leaves what does not rest on it standing.
#[test]
fn dropping_the_claimants_argument_returns_the_claim_to_open() {
    let g = navier_stokes_with_openais_claims();
    let r = json(
        g.propagate_from(
            &["req:claim-nl-proof-correct", "req:claim-lean-check-passes"],
            PropagateOptions::default(),
        )
        .unwrap(),
    );
    let rows: Vec<&Json> = r["inferences"]
        .as_array()
        .expect("inference rows")
        .iter()
        .filter(|x| x["conclusion"] == "req:stmt-c")
        .collect();
    assert!(!rows.is_empty(), "{r:#}");
    for row in &rows {
        assert_eq!(row["conclusion_was"], "claimed", "{row:#}");
        assert_eq!(row["conclusion_reads"], "open", "{row:#}");
        assert!(strs(&row["claimed_by"]).is_empty(), "{row:#}");
    }
    // Dropping only the Lean check leaves the natural-language claim: (C)
    // is still claimed.
    let r = json(
        g.propagate_from(
            &["req:claim-lean-check-passes"],
            PropagateOptions::default(),
        )
        .unwrap(),
    );
    let row = r["inferences"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["inference"] == "inf:c-via-lean")
        .cloned()
        .expect("the Lean route is reached");
    assert_eq!(row["conclusion_reads"], "claimed", "{row:#}");
    assert_eq!(strs(&row["claimed_by"]), vec!["OpenAI"]);
    // Nothing that does not rest on the claims is reached.
    let reached: Vec<String> = r["inferences"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|x| x["conclusion"].as_str().map(str::to_string))
        .collect();
    assert!(!reached.contains(&"req:stmt-a".to_string()), "{reached:?}");
}

/// FAILS ON v0.90.0 (the four colour rung's finding (a)): "Kempe's proof,
/// accepted 1879–1890" lived only in a dated record no report read. The step
/// now reads withdrawn — the period, the record, and Heawood's objection that
/// withdrew it — and the theorem's three dated statuses are read back.
#[test]
fn kempes_proof_reads_withdrawn_with_its_period_and_what_withdrew_it() {
    let g = load(FOUR_COLOUR);
    let step = json(g.inference_reading("inf:kempe-1879-reductio").unwrap());
    let w = &step["withdrawn"];
    assert_eq!(w["held"], "1879–1890", "{step:#}");
    assert_eq!(w["fact"], "fact:kempe-proof-accepted-1879-1890");
    assert_eq!(
        strs(&w["withdrawn_by"]),
        vec!["req:obj-heawood-1890-kempe-degree-5"],
        "{w:#}"
    );
    assert!(w["why"].as_str().unwrap().starts_with("withdrawn"));
    // Tait's argument, the same shape (1880–1891, until Petersen).
    let tait = json(
        g.inference_reading("inf:tait-1880-3-edge-colouring")
            .unwrap(),
    );
    assert_eq!(tait["withdrawn"]["held"], "1880–1891", "{tait:#}");
    // The theorem holds now: its history is read back, and it is not
    // withdrawn.
    let thm = claim(&g, "req:thm-four-colour");
    let periods: Vec<String> = thm["status_over_time"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["fact"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(
        periods,
        vec![
            "fact:4ct-believed-proven-1879-1890",
            "fact:4ct-open-1890-1976",
            "fact:4ct-proven-since-1976",
        ]
    );
    assert!(thm.get("withdrawn").is_none(), "{thm:#}");
}

/// THE SKILLS SAY HOW TO RECORD BOTH, in the words the readings use.
#[test]
fn the_proof_and_argument_skills_say_how_to_record_a_claim_and_a_period() {
    for (name, skill) in [
        ("design-a-proof", PROOF_SKILL),
        ("design-an-argument", ARGUMENT_SKILL),
    ] {
        assert!(skill.contains("claimed_by"), "{name}");
        assert!(skill.contains("fact_type: status"), "{name}");
        assert!(skill.contains("shared premise"), "{name}");
        assert!(skill.contains("not_read"), "{name}");
        assert!(
            skill.contains("nobody has sourced") || skill.contains("only contests"),
            "{name}"
        );
    }
}

// ─── 1. A superseded mirrored judgment is kept ────────────────────────────

const KJ1: &str = "req:kj1-kestrel-operates-in-x-band";
const KJ2: &str = "req:kj2-frequency-agility-unresolved";
const KJ3: &str = "req:kj3-erp-about-75-dbw";
const KJ4: &str = "req:kj4-pulse-doppler-prf-about-20khz";
const IFC: &str = "ifc:kestrel-threat-assessment-release";

fn surface(doc: &str) -> GraphExport {
    serde_json::from_str(doc).expect("surface parses")
}

fn edge(g: &mut DesignGraph, from: &str, rel: &str, to: &str, props: Props) {
    let (ft, tt) = (type_of(g, from), type_of(g, to));
    g.create_edge(rel, &ft, from, &tt, to, props)
        .unwrap_or_else(|e| panic!("{from} {rel} {to}: {e}"));
}

fn step(g: &mut DesignGraph, id: &str, premises: &[&str], conclusion: &str) {
    g.upsert_node(
        "Inference",
        id,
        Props::new().set("name", id).set("rule", "all"),
    )
    .unwrap();
    for p in premises {
        edge(g, p, "PREMISE_OF", id, Props::new());
    }
    edge(g, id, "CONCLUDES", conclusion, Props::new());
}

/// The acquisition design of the split-capstone trial, round 1, in small:
/// KESTREL-ASSESS v1 mirrored, and three decisions each concluded by a step
/// resting on the mirrored judgments — (a) cover the X band (KJ1, KJ2), (b)
/// design for agility (KJ2), (c) the detection range (KJ3) — plus a consumer
/// of the release, an emulator test calibrated against KJ4 and the importer's
/// DOCUMENTS from the mirrored product to KJ1.
fn acquisition() -> DesignGraph {
    let mut g = graph();
    g.add_project(
        "proj:harrier-rwr-acquisition",
        "HARRIER RWR acquisition (FICTIONAL)",
    )
    .unwrap();
    g.mirror_surface(&surface(KESTREL_V1), Some("2026-10-22"))
        .expect("v1 mirrors");
    g.add_requirement("req:mission-need", "Mission need", "Warn of KESTREL.")
        .unwrap();
    for (id, premises) in [
        ("dec:a-cover-x-band", vec![KJ1, KJ2, "req:mission-need"]),
        ("dec:b-design-for-agility", vec![KJ2]),
        ("dec:c-detection-range", vec![KJ3]),
    ] {
        g.add_decision(id, id, "chosen", None).unwrap();
        let s = format!("inf:{}", id.trim_start_matches("dec:"));
        step(&mut g, &s, &premises, id);
    }
    g.add_component("cmp:rwr", "RWR", "warns", Some("subsystem"))
        .unwrap();
    g.consumes("cmp:rwr", IFC).unwrap();
    edge(
        &mut g,
        "art:kestrel-assess-v1",
        "DOCUMENTS",
        KJ1,
        Props::new(),
    );
    g
}

/// FAILS ON v0.90.0 (part 4): the re-mirror guard counted edges INTO a
/// withdrawn node only, so the PREMISE_OF edges the decisions rest on — which
/// leave the judgment — were missed and the refresh went ahead.
#[test]
fn the_re_mirror_guard_counts_the_reasoning_edges_that_rest_on_a_withdrawn_judgment() {
    let mut g = acquisition();
    let err = g
        .mirror_surface(&surface(KESTREL_V2), Some("2026-10-22"))
        .expect_err("withdrawing what three decisions rest on is refused")
        .to_string();
    for needle in [
        &format!("PREMISE_OF {KJ1} -> inf:a-cover-x-band") as &str,
        &format!("PREMISE_OF {KJ2} -> inf:b-design-for-agility"),
        &format!("PREMISE_OF {KJ3} -> inf:c-detection-range"),
        "accept_withdrawal",
    ] {
        assert!(err.contains(needle), "{needle} not in: {err}");
    }
    // Refused means nothing changed.
    assert_eq!(
        g.mirrors().unwrap()[0].mirror_content_hash,
        surface(KESTREL_V1).content_hash
    );
}

/// FAILS ON v0.90.0 (parts 1 and 2): the refresh DELETED v1's withdrawn
/// judgments with no snapshot and no change event, and dropped every local
/// edge touching a mirrored node — the decisions' PREMISE_OF edges, the
/// importer's DOCUMENTS, and the CONSUMES into the release, which v2 still
/// publishes.
#[test]
fn a_refresh_keeps_each_withdrawn_judgment_marked_superseded_with_its_edges() {
    let mut g = acquisition();
    let v2 = surface(KESTREL_V2);
    let r = json(
        g.mirror_surface_with(&v2, Some("2026-10-22"), true)
            .expect("refreshed, the withdrawal accepted"),
    );
    assert_eq!(r["refreshed"], true);
    let mut gone = vec![KJ1, KJ2, KJ3, KJ4];
    gone.sort();
    assert_eq!(strs(&r["withdrawn"]), gone, "{r:#}");
    assert_eq!(strs(&r["superseded"]), gone, "{r:#}");
    assert_eq!(strs(&r["snapshots"]).len(), 4, "{r:#}");
    let chg = r["change_event"]
        .as_str()
        .expect("a change event")
        .to_string();
    for kj in [KJ1, KJ2, KJ3, KJ4] {
        let n = g
            .get_node("Requirement", kj)
            .unwrap()
            .unwrap_or_else(|| panic!("{kj} was deleted"));
        assert_eq!(
            n.properties
                .get("superseded_by_surface")
                .and_then(Value::as_str),
            v2.content_hash.as_deref(),
            "{kj}"
        );
        assert!(
            g.incoming(kj, Some("CHANGED"))
                .unwrap()
                .iter()
                .any(|e| e.from_id == chg),
            "{kj}: the change event names it"
        );
        assert!(!g.outgoing(kj, Some("HAS_SNAPSHOT")).unwrap().is_empty());
    }
    let kept = strs(&r["kept_edges"]);
    for e in [
        format!("PREMISE_OF {KJ1} -> inf:a-cover-x-band"),
        format!("PREMISE_OF {KJ3} -> inf:c-detection-range"),
    ] {
        assert!(kept.contains(&e), "{e} not reported kept: {kept:#?}");
    }
    // Every local edge is still there — into withdrawn and kept nodes alike.
    assert!(!g.outgoing(KJ1, Some("PREMISE_OF")).unwrap().is_empty());
    assert!(
        g.incoming(KJ1, Some("DOCUMENTS"))
            .unwrap()
            .iter()
            .any(|e| e.from_id == "art:kestrel-assess-v1")
    );
    assert!(
        g.incoming(IFC, Some("CONSUMES"))
            .unwrap()
            .iter()
            .any(|e| e.from_id == "cmp:rwr"),
        "a refresh drops no local edge into a node the new surface still publishes"
    );
    // v2's judgments arrived, and the pin moved.
    assert!(
        g.get_node("Requirement", "req:kj1-v2-kestrel-operates-in-ku-band")
            .unwrap()
            .is_some()
    );
    assert_eq!(g.mirrors().unwrap()[0].mirror_content_hash, v2.content_hash);
    // A withdrawn judgment reads superseded.
    assert_eq!(statement_reading(&g, KJ1)["reads"], "superseded");
    // Refreshing again with the same surface changes nothing more.
    let again = json(
        g.mirror_surface_with(&v2, Some("2026-10-23"), true)
            .unwrap(),
    );
    assert!(strs(&again["superseded"]).is_empty(), "{again:#}");
}

/// FAILS ON v0.90.0 (part 3): after the refresh `reasons_lost` read 0,
/// `reasoning_report` read the decisions as before minus the vanished
/// premises, and `propagate_from` on a withdrawn id answered unknown_seeds.
/// Every decision that rested on a withdrawn judgment now reads reason-lost
/// until its owner re-grounds it or lets it go.
#[test]
fn what_rested_on_a_withdrawn_judgment_reads_reason_lost_until_regrounded() {
    let mut g = acquisition();
    let before = json(g.reasoning_debt().unwrap().unwrap());
    assert_eq!(before["reasons_lost"], 0, "{before:#}");
    g.mirror_surface_with(&surface(KESTREL_V2), Some("2026-10-22"), true)
        .unwrap();
    let debt = json(g.reasoning_debt().unwrap().unwrap());
    assert_eq!(
        strs(&debt["reason_lost_ids"]),
        vec![
            "dec:a-cover-x-band",
            "dec:b-design-for-agility",
            "dec:c-detection-range"
        ],
        "{debt:#}"
    );
    assert_eq!(
        strs(&debt["premises_withdrawn_ids"]),
        vec![KJ1, KJ2, KJ3],
        "{debt:#}"
    );
    let a = claim(&g, "dec:a-cover-x-band");
    assert_eq!(strs(&a["rests_on_withdrawn"]), vec![KJ1, KJ2], "{a:#}");
    // The what-if seeds from a withdrawn id.
    let w = json(
        g.propagate_from(&[KJ2], PropagateOptions::default())
            .unwrap(),
    );
    assert!(strs(&w["unknown_seeds"]).is_empty(), "{w:#}");
    let reached: Vec<String> = w["inferences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["inference"].as_str().unwrap().to_string())
        .collect();
    assert!(
        reached.contains(&"inf:b-design-for-agility".to_string()),
        "{w:#}"
    );

    // RE-GROUNDED: (a) moves onto v2's judgment.
    g.delete_edge("PREMISE_OF", KJ1, "inf:a-cover-x-band")
        .unwrap();
    g.delete_edge("PREMISE_OF", KJ2, "inf:a-cover-x-band")
        .unwrap();
    edge(
        &mut g,
        "req:kj2-v2-9-4ghz-emissions-unexplained",
        "PREMISE_OF",
        "inf:a-cover-x-band",
        Props::new(),
    );
    // LET GO: (b) is superseded by its owner.
    set(
        &mut g,
        "dec:b-design-for-agility",
        "status",
        Value::String("superseded".into()),
    );
    let debt = json(g.reasoning_debt().unwrap().unwrap());
    assert_eq!(
        strs(&debt["reason_lost_ids"]),
        vec!["dec:c-detection-range"],
        "{debt:#}"
    );
}

/// FAILS ON v0.90.0 (part 5): in the producer's design, a judgment replaced
/// by `replaces` read "contested" — its own step still held, and the
/// replacement left its truth unknown — and stayed published beside its
/// successor until it was set internal by hand.
#[test]
fn a_judgment_replaced_by_replaces_reads_superseded_and_is_not_published() {
    let mut g = graph();
    g.add_project("proj:intel", "Intel (FICTIONAL)").unwrap();
    g.add_requirement("req:r2", "R2 SIGINT", "Emissions at 9.4 GHz.")
        .unwrap();
    set(
        &mut g,
        "req:r2",
        "statement_kind",
        Value::String("evidence".into()),
    );
    set(&mut g, "req:r2", "taken_as_given", Value::Bool(true));
    for (id, name) in [
        ("req:kj1", "KJ1: X band"),
        ("req:kj1-v2", "v2 KJ1: Ku band"),
    ] {
        g.add_requirement(id, name, name).unwrap();
        set(
            &mut g,
            id,
            "statement_kind",
            Value::String("judgement".into()),
        );
        g.set_requirement_designation(id, "published").unwrap();
    }
    step(&mut g, "inf:kj1", &["req:r2"], "req:kj1");
    assert!(g.published_promises().unwrap().contains("req:kj1"));
    g.supersede("Requirement", "req:kj1-v2", "req:kj1", "add_requirement")
        .unwrap();
    assert_eq!(proof_status(&g, "req:kj1"), "superseded");
    assert_eq!(statement_reading(&g, "req:kj1")["reads"], "superseded");
    let c = claim(&g, "req:kj1");
    assert!(
        c["superseded"].as_str().unwrap().contains("req:kj1-v2"),
        "{c:#}"
    );
    let published = g.published_promises().unwrap();
    assert!(!published.contains("req:kj1"), "{published:?}");
    assert!(published.contains("req:kj1-v2"));
}

// ─── 4. A source named but not read never counts as citing ────────────────

fn uncited(g: &DesignGraph) -> Vec<String> {
    strs(&json(g.reasoning_debt().unwrap().unwrap())["uncited_premise_ids"])
}

/// FAILS ON v0.90.0 (the Navier–Stokes map's finding (c)): an Artifact the
/// designer marked NOT READ — OpenAI's 166-page paper, which nobody read —
/// took the claim it DOCUMENTS off `uncited_premises` (here the Lean
/// formalization, against OpenAI's claim that the Lean check passes), because a DOCUMENTS
/// edge from any Artifact counted as citing and nothing could say "named,
/// not read" (dec:idea-a-source-says-what-kind-of-support-it-gives, settled
/// 2026-10-23: option (c) plus a "named, not read" marking).
#[test]
fn a_source_named_but_not_read_never_takes_a_claim_off_uncited() {
    let mut g = load(NAVIER_STOKES);
    let claim_id = "req:claim-lean-check-passes";
    assert!(
        uncited(&g).contains(&claim_id.to_string()),
        "measured: uncited, {:?}",
        uncited(&g)
    );
    edge(
        &mut g,
        "art:openai-lean-not-read",
        "DOCUMENTS",
        claim_id,
        Props::new(),
    );
    assert!(
        !uncited(&g).contains(&claim_id.to_string()),
        "an unmarked source still cites, as before"
    );
    set(
        &mut g,
        "art:openai-lean-not-read",
        "not_read",
        Value::Bool(true),
    );
    assert!(
        uncited(&g).contains(&claim_id.to_string()),
        "a source named but not read never counts as citing: {:?}",
        uncited(&g)
    );
    let c = claim(&g, claim_id);
    assert_eq!(c["sources"][0]["not_read"], true, "{c:#}");
    // And what rests on it is read with it uncited.
    let r = json(g.reasoning_report(Some("req:stmt-c"), Some(500)).unwrap());
    assert!(
        strs(&r["uncited_premises"]).contains(&claim_id.to_string()),
        "{r:#}"
    );
    // Even with a checksum, it pins nothing.
    set(
        &mut g,
        "art:openai-lean-not-read",
        "checksum",
        Value::String("sha256:00".into()),
    );
    let c = claim(&g, claim_id);
    assert_eq!(c["sources"][0]["pinned"], false, "{c:#}");
}

/// A SOURCE CARRIES EVIDENCE'S GRADE, and a citation it documents reads no
/// stronger than that grade allows: a pinned abstract graded B/2 checks the
/// citation; graded D or 4 and worse, it reads unverified.
#[test]
fn a_citation_reads_no_stronger_than_its_sources_grade() {
    let mut g = load(NAVIER_STOKES);
    let cite = "req:cite-civ-regularity";
    let src = "art:s5-civ-abstract";
    set(&mut g, src, "checksum", Value::String("sha256:01".into()));
    assert_eq!(statement_reading(&g, cite)["reads"], "cited");
    set(&mut g, src, "source_reliability", Value::String("B".into()));
    set(
        &mut g,
        src,
        "information_credibility",
        Value::String("2".into()),
    );
    assert_eq!(statement_reading(&g, cite)["reads"], "cited");
    let c = claim(&g, cite);
    assert_eq!(c["sources"][0]["grade"]["source_reliability"], "B", "{c:#}");
    set(
        &mut g,
        src,
        "information_credibility",
        Value::String("4".into()),
    );
    assert_eq!(statement_reading(&g, cite)["reads"], "cited_unverified");
    let c = claim(&g, cite);
    assert!(
        c["sources"][0]["pins_nothing"]
            .as_str()
            .unwrap()
            .contains("information credibility 4"),
        "{c:#}"
    );
    set(
        &mut g,
        src,
        "information_credibility",
        Value::String("2".into()),
    );
    set(&mut g, src, "source_reliability", Value::String("E".into()));
    assert_eq!(statement_reading(&g, cite)["reads"], "cited_unverified");
}

// ─── 5. An objection nobody has sourced contests and never defeats ────────

/// FAILS ON v0.90.0 (the four colour rung's finding (c),
/// req:an-objection-taken-as-given-with-no-pinned-source-contests-and-never-
/// defeats): Heawood's objection to Kempe's degree-5 step is taken as given
/// (Heawood 1890 not read, the example map recalled). AS EXPORTED, S1 — a
/// source pinned by checksum — DOCUMENTS it, so it defeats the step under the
/// old rule and the new alike. With no pinned source documenting it, the
/// state the finding describes, it DEFEATED the step outright; it now
/// contests it, and the step's conclusion reads contested through it. Once a
/// pinned source documents it again, it defeats the step as before.
#[test]
fn an_objection_nobody_has_sourced_contests_kempes_step_and_never_defeats_it() {
    const OBJ: &str = "req:obj-heawood-1890-kempe-degree-5";
    let mut g = load(FOUR_COLOUR);
    let step = "inf:kempe-degree-5";
    assert_eq!(
        json(g.inference_reading(step).unwrap())["objected"],
        "defeated",
        "as exported, a pinned source documents the objection"
    );
    let sources: Vec<String> = g
        .incoming(OBJ, Some("DOCUMENTS"))
        .unwrap()
        .into_iter()
        .map(|e| e.from_id)
        .collect();
    assert!(sources.contains(&"art:s1-gonthier-2008-notices".to_string()));
    for src in &sources {
        g.delete_edge("DOCUMENTS", src, OBJ).unwrap();
    }
    let r = json(g.inference_reading(step).unwrap());
    assert_eq!(r["objected"], "contested", "{r:#}");
    let o = &r["objections"][0];
    assert_eq!(o["standing"], "stands", "{r:#}");
    assert!(
        o["why"].as_str().unwrap().contains("nobody has sourced"),
        "{r:#}"
    );
    let lemma = "req:lem-kempe-mc4-no-degree-5";
    assert_eq!(proof_status(&g, lemma), "contested");
    // Kempe's proof still reads withdrawn, the objection still named.
    let k = json(g.inference_reading("inf:kempe-1879-reductio").unwrap());
    assert_eq!(strs(&k["withdrawn"]["withdrawn_by"]), vec![OBJ], "{k:#}");
    // A pinned source documents it: the defeat rule applies again.
    edge(
        &mut g,
        "art:s1-gonthier-2008-notices",
        "DOCUMENTS",
        OBJ,
        Props::new(),
    );
    let r = json(g.inference_reading(step).unwrap());
    assert_eq!(r["objected"], "defeated", "{r:#}");
}
