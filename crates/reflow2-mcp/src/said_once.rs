//! A reasoning read-back says each premise's reason ONCE, and each advisory
//! sentence once with the ids it applies to.
//!
//! ⭐ WHY, MEASURED 2026-10-22 on the heat-equation rung's theorem through the
//! hub (fact:root-cause-a-reasoning-read-back-outgrows-the-client-on-repeated-
//! premise-prose-and-text-added-after-the-budget-2026-10-22): a page asked at
//! 60,000 reached the client at 63,998 characters and was refused. Every
//! premise row re-rendered that premise's whole `why` — 9,953 characters on
//! the page — and the "cited, unverified … Register the source" sentence was
//! said 27 times (7,749 characters). On the committed fixture
//! (`proof_trial_heat_equation.json`), the whole walk's 126 premise rows carry
//! 23,332 characters of `why`, of which 7,956 are distinct.
//!
//! So a premise row carries a SHORT READING (its `why`, cut to the leading
//! clause of its reason), the page cites each premise's full reason once (`premise_whys`,
//! keyed by the premise's id), and an advisory sentence that a reason would
//! repeat for every unverified citation is said once in `advisories`, naming
//! every id it applies to.
//!
//! 🛑 WHAT IS NEVER TOUCHED: the words of a claim (`says`), a step's rule and
//! warrant, a premise's name. Those ARE the proof
//! (req:the-reasoning-read-back-keeps-the-words-of-the-claims-it-returns);
//! this module moves only computed reasons (`why`), and moves them, never
//! drops them — every reason the page had is on the page, once. No field is
//! removed: a premise row's `why` holds its reason's leading clause.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value, json};

/// A reason this short is its own reading: carried in the row whole, never
/// cited apart.
pub const SHORT_READING_MAX: usize = 100;

/// Where a reason's leading clause ends: the first of these, whichever comes
/// first. Each marks the start of the detail a reader can look up once — the
/// explanation after a colon, the assumptions or citations it rests on.
const CLAUSE_ENDS: [&str; 5] = [": ", "; ", ", under the", ", resting on", " — "];

/// The leading clause of a reason: "cited, unverified", "concluded by
/// inf:w-regular holds", "taken as given, a stated foundation". A reason of
/// at most [`SHORT_READING_MAX`] characters is its own reading. The clause is
/// never cut inside itself: a reading is words the reason says, whole, and
/// the rest of the reason is cited once beside the rows.
#[must_use]
pub fn short_reading(why: &str) -> String {
    if why.chars().count() <= SHORT_READING_MAX {
        return why.to_string();
    }
    let end = CLAUSE_ENDS
        .iter()
        .filter_map(|m| why.find(m))
        .min()
        .unwrap_or(why.len());
    why[..end].to_string()
}

/// The advisories a reason may carry, each said once per page: its key, the
/// sentence as the reason says it, and what the reason says in its place.
fn advisories() -> [(&'static str, &'static str, String); 2] {
    use reflow2_core::inference::{CITED_UNVERIFIED_WHY, EVIDENCE_UNVERIFIED_TAIL};
    [
        (
            "cited_unverified",
            CITED_UNVERIFIED_WHY,
            "cited, unverified (said once in `advisories`: cited_unverified)".to_string(),
        ),
        (
            "evidence_unverified",
            EVIDENCE_UNVERIFIED_TAIL,
            ", unverified (said once in `advisories`: evidence_unverified)".to_string(),
        ),
    ]
}

/// Every premise row in one claim row: its `why` cut to its short reading,
/// the full reason moved out. Returns what was moved, premise id → full
/// reason. A reason that is its own reading stays whole and is not moved.
///
/// The row KEEPS its `why` field, now the leading clause of the reason: a
/// reply field is never removed without a release of notice
/// (`crate::deprecation`), and the clause is the same reason, in short.
pub fn premise_rows(claim: &mut Value) -> BTreeMap<String, String> {
    let mut moved = BTreeMap::new();
    let Some(steps) = claim.get_mut("support").and_then(Value::as_array_mut) else {
        return moved;
    };
    for step in steps {
        let Some(premises) = step.get_mut("premises").and_then(Value::as_array_mut) else {
            continue;
        };
        for p in premises {
            let Some(o) = p.as_object_mut() else { continue };
            let Some(why) = o.get("why").and_then(Value::as_str).map(str::to_string) else {
                continue;
            };
            let reads = short_reading(&why);
            if reads == why {
                continue;
            }
            if let Some(id) = o.get("id").and_then(Value::as_str) {
                moved.insert(id.to_string(), why);
                o.insert("why".into(), Value::String(reads));
            }
        }
    }
    moved
}

/// What `premise_whys` means, said once on a page that carries it.
pub const PREMISE_WHYS_NOTE: &str = "Each premise row says its reason in short (its `why`, \
     the reason's leading clause); the full reason of every premise on this page whose reason is longer is here ONCE, keyed by the \
     premise's id. A premise reads the same in every step that takes it up. A premise that is \
     itself a claim on this page is not repeated here: its own row's `why` says it in full.";

/// Put the page's `premise_whys` on `page` (from what its kept rows moved),
/// then say each advisory once: every `why` on the page that carries one says
/// a short pointer instead, and `advisories` names the ids it applies to.
pub fn cite_once(page: &mut Value, moved: &BTreeMap<String, String>) {
    let Some(o) = page.as_object_mut() else {
        return;
    };
    // A premise that is itself a claim on this page, whose row's `why` already
    // says its reason in full, is not said again: its row is where it is said.
    let in_rows: BTreeMap<&str, &str> = o
        .get("claims")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(|r| Some((r.get("id")?.as_str()?, r.get("why")?.as_str()?)))
                .collect()
        })
        .unwrap_or_default();
    let cited: Map<String, Value> = moved
        .iter()
        .filter(|(id, why)| {
            !in_rows
                .get(id.as_str())
                .is_some_and(|w| w.contains(why.as_str()))
        })
        .map(|(k, v)| (k.clone(), Value::String(v.clone())))
        .collect();
    if !cited.is_empty() {
        o.insert("premise_whys".into(), Value::Object(cited));
        o.insert("premise_whys_note".into(), json!(PREMISE_WHYS_NOTE));
    }
    let mut applied: BTreeMap<&'static str, BTreeSet<String>> = BTreeMap::new();
    let table = advisories();
    for (key, value) in o.iter_mut() {
        if key == "premise_whys" {
            if let Some(m) = value.as_object_mut() {
                for (id, why) in m.iter_mut() {
                    replace_in(why, &table, Some(id.as_str()), &mut applied);
                }
            }
            continue;
        }
        walk(value, key == "why", None, &table, &mut applied);
    }
    if applied.is_empty() {
        return;
    }
    let said: Vec<Value> = table
        .iter()
        .filter_map(|(key, sentence, _)| {
            applied.get(key).map(|ids| {
                json!({
                    "advisory": key,
                    "says": sentence.trim_start_matches([',', ' ']),
                    "applies_to": ids,
                })
            })
        })
        .collect();
    o.insert("advisories".into(), Value::Array(said));
}

/// Replace the advisories in every string under a `why` key, recording the
/// nearest enclosing `id` each applied to.
fn walk(
    v: &mut Value,
    under_why: bool,
    id: Option<&str>,
    table: &[(&'static str, &'static str, String)],
    applied: &mut BTreeMap<&'static str, BTreeSet<String>>,
) {
    match v {
        Value::String(_) if under_why => replace_in(v, table, id, applied),
        Value::Array(items) => items
            .iter_mut()
            .for_each(|x| walk(x, false, id, table, applied)),
        Value::Object(o) => {
            let own = o.get("id").and_then(Value::as_str).map(str::to_string);
            let here = own.as_deref().or(id);
            let keys: Vec<String> = o.keys().cloned().collect();
            for k in keys {
                if let Some(x) = o.get_mut(&k) {
                    walk(x, k == "why", here, table, applied);
                }
            }
        }
        _ => {}
    }
}

fn replace_in(
    v: &mut Value,
    table: &[(&'static str, &'static str, String)],
    id: Option<&str>,
    applied: &mut BTreeMap<&'static str, BTreeSet<String>>,
) {
    let Value::String(s) = v else { return };
    for (key, sentence, instead) in table {
        if s.contains(sentence) {
            *s = s.replace(sentence, instead);
            if let Some(id) = id {
                applied.entry(key).or_default().insert(id.to_string());
            }
        }
    }
}

/// Add what one row moved to what the page has moved: a premise reads the
/// same in every row, so the first is kept.
pub fn merge_moved(into: &mut BTreeMap<String, String>, from: &BTreeMap<String, String>) {
    for (k, v) in from {
        into.entry(k.clone()).or_insert_with(|| v.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reading_is_the_leading_clause_of_its_reason() {
        let cited = reflow2_core::inference::CITED_UNVERIFIED_WHY;
        assert_eq!(short_reading(cited), "cited, unverified");
        assert_eq!(
            short_reading(
                "concluded by inf:w-regular holds, under the assumption(s) req:hyp-two-solutions, \
                 which no step on the way discharges, resting on the unverified citation(s) \
                 req:cite-continuity-and-derivative-algebra"
            ),
            "concluded by inf:w-regular holds"
        );
        // Short enough to be its own reading: whole.
        let short = "taken as given, a stated foundation; nothing records it violated";
        assert_eq!(short_reading(short), short);
        // A clause is never cut inside itself.
        let long = "word ".repeat(40);
        assert_eq!(short_reading(&long), long);
    }

    #[test]
    fn a_reason_moves_once_and_an_advisory_is_said_once_with_its_ids() {
        let cited = reflow2_core::inference::CITED_UNVERIFIED_WHY;
        let mut page = json!({
            "claims": [
                {"id": "req:a", "why": "concluded", "support": [{"premises": [
                    {"id": "req:cite", "name": "the cited lemma", "why": cited},
                    {"id": "req:b", "why": "taken as given"},
                    {"id": "req:far", "why": cited},
                ]}]},
                {"id": "req:cite", "why": cited, "statement": {"why": cited}},
            ]
        });
        let mut moved = BTreeMap::new();
        for c in page["claims"].as_array_mut().unwrap() {
            merge_moved(&mut moved, &premise_rows(c));
        }
        cite_once(&mut page, &moved);
        let text = page.to_string();
        assert_eq!(text.matches("Register the source").count(), 1, "{page:#}");
        let row = &page["claims"][0]["support"][0]["premises"][0];
        assert_eq!(row["why"], "cited, unverified");
        assert_eq!(row["name"], "the cited lemma", "a premise keeps its name");
        assert_eq!(
            page["claims"][0]["support"][0]["premises"][1]["why"],
            "taken as given"
        );
        assert_eq!(
            page["advisories"][0]["applies_to"],
            json!(["req:cite", "req:far"])
        );
        // Cited once: a premise off the page in `premise_whys`; one that is a
        // claim on the page, in its own row.
        assert!(
            page["premise_whys"]["req:far"]
                .as_str()
                .unwrap()
                .contains("advisories")
        );
        assert!(page["premise_whys"].get("req:cite").is_none(), "{page:#}");
    }
}
