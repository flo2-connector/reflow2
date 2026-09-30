//! "DOES THIS STORED VALUE SETTLE INTENT?" HAS ONE ANSWER.
//!
//! Two programs ask it: the core's [`reflow2_core::intent`] table, which every
//! writer consults before it writes, and CI's `tools/check_intent_authority.py`
//! (`settles_intent`), which reads the committed export afterwards. Two copies
//! of one rule drift silently — the generic writers went round a rule the
//! typed doors followed for exactly that reason
//! (fact:root-cause-the-settle-rule-guards-the-typed-doors-and-the-generic-writers-go-around-it-2026-09-29).
//! This runs the Python predicate over every case the core table decides and
//! fails on the first disagreement.

use std::collections::HashMap;
use std::process::Command;

use reflow2_core::Value;
use reflow2_core::intent::{SETTLING, SettleWhen, settles_intent};

fn cases() -> Vec<(&'static str, &'static str, Option<Value>)> {
    let mut out = Vec::new();
    for sp in SETTLING {
        out.push((sp.node_type, sp.property, None));
        let values: Vec<Value> = match sp.when {
            SettleWhen::Present => vec![Value::Bool(true), Value::Bool(false)],
            SettleWhen::NotIn(_) | SettleWhen::In(_) => [
                "proposed",
                "accepted",
                "deferred",
                "dropped",
                "met",
                "rejected",
                "superseded",
            ]
            .iter()
            .map(|s| Value::String((*s).into()))
            .collect(),
        };
        for v in values {
            out.push((sp.node_type, sp.property, Some(v)));
        }
    }
    // A type that asserts no intent, carrying a status word.
    out.push((
        "Capability",
        "status",
        Some(Value::String("accepted".into())),
    ));
    out
}

#[test]
fn the_ci_gate_and_the_core_agree_on_every_case() {
    let tools = concat!(env!("CARGO_MANIFEST_DIR"), "/../../tools");
    let cases = cases();
    let nodes: Vec<serde_json::Value> = cases
        .iter()
        .map(|(t, p, v)| {
            let mut props = serde_json::Map::new();
            if let Some(v) = v {
                props.insert((*p).into(), serde_json::to_value(v).expect("value"));
            }
            serde_json::json!({"node_type": t, "properties": props})
        })
        .collect();
    let script = format!(
        "import json, sys; sys.path.insert(0, {tools:?}); \
         from check_intent_authority import settles_intent; \
         print(json.dumps([bool(settles_intent(n)) for n in json.loads(sys.stdin.read())]))"
    );
    let mut child = Command::new("python3")
        .args(["-c", &script])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("python3 runs — CI's gates already need it");
    use std::io::Write;
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(serde_json::to_string(&nodes).expect("json").as_bytes())
        .expect("write");
    let out = child.wait_with_output().expect("python3 finished");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let python: Vec<bool> = serde_json::from_slice(&out.stdout).expect("a list of booleans");
    let mut problems = Vec::new();
    for ((t, p, v), py) in cases.iter().zip(python) {
        let mut props = HashMap::new();
        if let Some(v) = v {
            props.insert((*p).to_string(), v.clone());
        }
        let core = settles_intent(t, &props);
        if core != py {
            problems.push(format!(
                "{t}.{p}={v:?}: core says {core}, check_intent_authority.py says {py}"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
