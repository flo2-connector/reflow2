//! reflow2's derived relations are DECLARED, the declarations are held to the
//! code, and a served read evaluates them.
//!
//! `req:reflow2-declares-its-derived-relations-and-serves-a-read-that-runs-them`,
//! promoted by Anthony on 2026-09-29 ("go with the 7 ideas you'd promote").
//! Root cause it answers:
//! `fact:root-cause-no-tool-can-evaluate-reflow2s-derived-relations-because-reflow2-declares-none-of-them-2026-09-29`
//! — 23 derived relations, hand-written across ~15 files, declared nowhere.
//!
//! THE COUNTER-ARGUMENT THIS FILE EXISTS TO ANSWER
//! (`dec:idea-a-served-read-evaluates-every-derived-relation-reflow2-computes`):
//! declarations kept beside hand-written code are a second copy, honest only if
//! a test checks them against the code.
//!
//! ⭐ EACH CHECK STARTS FROM A POPULATION THE DECLARATIONS DO NOT CHOOSE. The
//! first version (#631) held every declaration only to what the declaration
//! itself named: the functions it listed (followed one `self.` call deep, blind
//! to const tables), the edges it listed, and its own `components`. So it could
//! only find disagreement INSIDE what was declared. Measured 2026-09-29 on
//! 87f17ec (graph-primitives' monitoring pass 3, re-measured here in Rust):
//! `readiness_gate` read HAS_READINESS two calls deep and did not declare it;
//! `impact` read five risk edges through the `RISK_EDGES` const and did not
//! declare them; `budget_rollup` and `arrival_delta` were computed over
//! relations their `over` did not name; 135 functions naming an edge type were
//! answered for by nothing; and 13 declarations had padded `components` to
//! pass the check that each edge share a primitive with them. Root cause:
//! `fact:root-cause-the-derived-declaration-check-draws-every-population-from-the-declarations-it-checks-2026-09-29`.
//! So now:
//!
//! - THE CODE'S REACH. Everything a declared function reaches through calls —
//!   `self.f()`, `Self::f`, `Type::f()`, free and module-path calls, any depth —
//!   and every edge type named directly, through a `const`/`static` table, or
//!   through the schema's `inference_edge_types()` selector, is held to the
//!   declaration. A `dedicated` function may reach only declared edges, and may
//!   reach another relation's own code only if `over` names that relation.
//!   A walk from a reader never enters a writer (`&mut self`: the borrow rules
//!   forbid it on the same graph), and a declared site that is a writer fails.
//! - THE CRATE'S POPULATION. Every function in `src/` that names an edge type
//!   is either answered for by a declaration (named by one, or reached by a
//!   dedicated one) or listed in `schema/derived/edge_readers.yaml`, with the
//!   edges it reads. A new one, or a listed one reading a new edge type,
//!   fails until somebody judges it. Whether a function DERIVES a relation is a
//!   judgement about meaning that no scanner can make — a writer, a guard, a
//!   one-hop projection and a derivation name edge types the same way — so the
//!   list's baseline says `unjudged`, and it may only shrink.
//! - THE RULE'S OWN TEXT. `components` are exactly the primitives the
//!   composition writes (`Reading::composition_primitives`), and a declared
//!   edge whose reading shares no primitive with the composition — its own or
//!   that of a relation it is `over` — is named in `not_yet_stated`.
//!
//! WHAT THIS CANNOT CHECK: that a relation's RULE, as prose, is what the code
//! COMPUTES. A function that reads the declared edges and returns the wrong
//! answer passes. Checking results needs the rule stated as something a
//! machine evaluates — the rule engine this design does not yet have
//! (`req:derived-relations-and-reports-are-maintained-incrementally`). Calls
//! are resolved by NAME, not type: a name defined twice resolves to both, so
//! the walk over-approximates (a `.name()` on anything but `self` resolves only
//! to `DesignGraph` methods). An edge type chosen at runtime by any selector
//! other than `inference_edge_types()` — by reading, by property value — is
//! invisible to it.
//!
//! OBSERVED FAILING, 2026-09-29, on origin/main 87f17ec with only this file
//! and `schema/derived/edge_readers.yaml` applied: see the PR. The transitive
//! check named readiness_gate's HAS_READINESS and impact's five risk edges;
//! the population check named the 135 unanswered functions; the components
//! check named the 14 declarations whose components are not their
//! composition's.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use reflow2_core::DesignGraph;
use reflow2_core::derived::{
    DERIVED_COSTS, DERIVED_EVALUATORS, DerivedRelation, INFERENCE_KINDS, STATED_OVER_READINGS,
    declared_derived_relations,
};
use reflow2_core::foundation::core::{READING_FORMS, RELATION_PRIMITIVES, Reading, Value};
use reflow2_core::nodes::{edge, node};

fn graph() -> DesignGraph {
    DesignGraph::open_in_memory().expect("in-memory graph")
}

fn declarations() -> Vec<DerivedRelation> {
    declared_derived_relations().expect("schema/derived/relations.yaml parses")
}

// ─── The source scanner ─────────────────────────────────────────────────────

/// A source text blanked where it cannot name an edge: comments and string
/// literals become spaces, except a literal that is all capitals and
/// underscores, which may be an edge type written as a string.
fn blank(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let spaces = |out: &mut String, s: &str| {
        for ch in s.chars() {
            out.push(if ch == '\n' { '\n' } else { ' ' });
        }
    };
    while i < b.len() {
        let rest = &src[i..];
        if rest.starts_with("//") {
            let end = rest.find('\n').unwrap_or(rest.len());
            spaces(&mut out, &rest[..end]);
            i += end;
        } else if let Some(after) = rest.strip_prefix("/*") {
            let end = after.find("*/").map_or(rest.len(), |e| e + 4);
            spaces(&mut out, &rest[..end]);
            i += end;
        } else if (rest.starts_with("r\"") || rest.starts_with("r#"))
            && (i == 0 || !(b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_'))
        {
            let hashes = rest[1..].chars().take_while(|c| *c == '#').count();
            if rest[1 + hashes..].starts_with('"') {
                let close = format!("\"{}", "#".repeat(hashes));
                let start = 2 + hashes;
                let end = rest[start..]
                    .find(&close)
                    .map_or(rest.len(), |e| start + e + close.len());
                spaces(&mut out, &rest[..end]);
                i += end;
            } else {
                out.push('r');
                i += 1;
            }
        } else if rest.starts_with('"') {
            let mut j = 1;
            let rb = rest.as_bytes();
            while j < rb.len() && rb[j] != b'"' {
                j += if rb[j] == b'\\' { 2 } else { 1 };
            }
            let lit = &rest[1..j.min(rest.len())];
            let keep = !lit.is_empty() && lit.bytes().all(|c| c.is_ascii_uppercase() || c == b'_');
            out.push('"');
            if keep {
                out.push_str(lit);
            } else {
                spaces(&mut out, lit);
            }
            out.push('"');
            i += (j + 1).min(rest.len());
        } else if rest.starts_with('\'') {
            // A char literal ('x', '\n', '{') — not a lifetime ('a).
            let rb = rest.as_bytes();
            let len = if rb.len() > 3 && rb[1] == b'\\' && rb[3] == b'\'' {
                4
            } else if rb.len() > 2 && rb[2] == b'\'' {
                3
            } else {
                0
            };
            if len > 0 {
                spaces(&mut out, &rest[..len]);
                i += len;
            } else {
                out.push('\'');
                i += 1;
            }
        } else {
            let ch = rest.chars().next().expect("non-empty");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The byte offset of the `}` closing the `{` at `open`, in blanked text.
fn matching_brace(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (k, ch) in s[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + k);
                }
            }
            _ => {}
        }
    }
    None
}

/// Every offset where `word` starts as a whole token (not inside a longer identifier).
fn token_starts(s: &str, word: &str) -> Vec<usize> {
    let mut out = Vec::new();
    for (k, _) in s.match_indices(word) {
        let before_ok = k == 0 || !is_ident(s[..k].chars().next_back().unwrap());
        let after_ok = s[k + word.len()..]
            .chars()
            .next()
            .is_none_or(|c| !is_ident(c));
        if before_ok && after_ok {
            out.push(k);
        }
    }
    out
}

/// Blank every `#[cfg(test)] mod … { … }` block: test code computes nothing
/// the design serves.
fn strip_test_modules(s: &str) -> String {
    let mut out = s.to_string();
    let mut from = 0;
    while let Some(p) = out[from..].find("#[cfg(test)]") {
        let at = from + p;
        from = at + 1;
        let rest = out[at + "#[cfg(test)]".len()..].trim_start();
        let rest = rest.strip_prefix("pub ").unwrap_or(rest);
        if !rest.starts_with("mod ") {
            continue;
        }
        let Some(open) = out[at..].find('{').map(|o| at + o) else {
            continue;
        };
        if out[at..open].contains(';') {
            continue; // `mod tests;` — a file of its own, not scanned as src.
        }
        let Some(close) = matching_brace(&out, open) else {
            continue;
        };
        let blanked: String = out[at..=close]
            .chars()
            .map(|c| if c == '\n' { '\n' } else { ' ' })
            .collect();
        out.replace_range(at..=close, &blanked);
    }
    out
}

/// One function item in the core crate's source.
struct FnItem {
    /// Path under `crates/reflow2-core/src/`, e.g. `readiness.rs`.
    file: String,
    name: String,
    /// The type of the `impl` block it sits in, when it is a method.
    self_ty: Option<String>,
    /// True when it takes `&mut self`: a WRITER. A function reading the
    /// design through `&self` cannot call one on the same graph, so a walk
    /// from a reader never enters a writer, and a declaration naming a writer
    /// as the code that computes a relation is wrong on its face.
    mut_self: bool,
    /// Its body, blanked.
    body: String,
}

/// How a call names the function it calls.
enum CallForm {
    /// `self.f(…)` or `Self::f(…)`.
    OnSelf,
    /// `x.f(…)` on anything but `self`.
    OnOther,
    /// `Type::f(…)`.
    Assoc(String),
    /// `f(…)` or `module::f(…)`.
    Free,
}

/// The core crate's source, read as functions: which edge types each names,
/// and which functions each calls — so a declaration can be held to
/// everything its code REACHES, not only the body it names.
struct Source {
    fns: Vec<FnItem>,
    by_name: HashMap<String, Vec<usize>>,
    /// `edge::CONST` → the edge type it names.
    edge_consts: HashMap<String, String>,
    edge_types: BTreeSet<String>,
    /// A `const` or `static` item outside every function → the edge types it
    /// names, directly or through another such item.
    const_edges: HashMap<String, BTreeSet<String>>,
    /// A function that returns edge types chosen at RUNTIME from the schema →
    /// the types it returns. A call to one reads them all.
    selectors: HashMap<String, BTreeSet<String>>,
}

impl Source {
    fn load(g: &DesignGraph) -> Self {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = BTreeMap::new();
        let mut stack = vec![dir.clone()];
        while let Some(d) = stack.pop() {
            for entry in std::fs::read_dir(&d).expect("read a src/ directory") {
                let p = entry.expect("dir entry").path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                    let rel = p
                        .strip_prefix(&dir)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    files.insert(rel, std::fs::read_to_string(&p).expect("read source"));
                }
            }
        }
        // `pub mod edge { pub const NAME: &str = "VALUE"; … }` in nodes.rs.
        let nodes_raw = &files["nodes.rs"];
        let edge_mod = &nodes_raw[nodes_raw.find("pub mod edge").expect("pub mod edge")..];
        let mut edge_consts = HashMap::new();
        for line in edge_mod.lines() {
            let l = line.trim();
            if let Some(rest) = l.strip_prefix("pub const ")
                && let (Some(colon), Some(q1)) = (rest.find(':'), rest.find('"'))
            {
                let name = rest[..colon].trim().to_string();
                let value: String = rest[q1 + 1..].chars().take_while(|c| *c != '"').collect();
                edge_consts.insert(name, value);
            }
            if l == "}" {
                break;
            }
        }
        let edge_types: BTreeSet<String> = g.schema().edge_types.keys().cloned().collect();
        let mut src = Source::from_files(files, edge_consts, edge_types);
        // `impact` propagates along every edge type of the schema's inference
        // module, chosen by module at runtime and named nowhere in its code.
        src.selectors.insert(
            "inference_edge_types".to_string(),
            g.schema()
                .inference_edge_types()
                .into_iter()
                .map(str::to_string)
                .collect(),
        );
        src
    }

    /// Build from raw source files (path → text). Split out so the scanner's
    /// own net can run on synthetic source.
    fn from_files(
        raw: BTreeMap<String, String>,
        edge_consts: HashMap<String, String>,
        edge_types: BTreeSet<String>,
    ) -> Self {
        let mut src = Source {
            fns: Vec::new(),
            by_name: HashMap::new(),
            edge_consts,
            edge_types,
            const_edges: HashMap::new(),
            selectors: HashMap::new(),
        };
        let mut const_text: Vec<(String, String)> = Vec::new();
        for (file, text) in raw {
            let s = strip_test_modules(&blank(&text));
            // impl blocks: (open, close, self type).
            let mut impls: Vec<(usize, usize, String)> = Vec::new();
            for k in token_starts(&s, "impl")
                .into_iter()
                .chain(token_starts(&s, "trait"))
            {
                let head_end = match s[k..].find(['{', ';']) {
                    Some(e) if s.as_bytes()[k + e] == b'{' => k + e,
                    _ => continue,
                };
                let word = if s[k..].starts_with("impl") { 4 } else { 5 };
                let mut head = s[k + word..head_end].trim();
                if head.starts_with('<') {
                    let mut depth = 0i32;
                    let mut cut = head.len();
                    for (i, ch) in head.char_indices() {
                        match ch {
                            '<' => depth += 1,
                            '>' => {
                                depth -= 1;
                                if depth == 0 {
                                    cut = i + 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    head = head[cut..].trim();
                }
                if let Some(f) = head.find(" for ") {
                    head = head[f + 5..].trim();
                }
                let path: String = head
                    .chars()
                    .take_while(|c| is_ident(*c) || *c == ':')
                    .collect();
                let ty = path.rsplit("::").next().unwrap_or_default().to_string();
                if let Some(close) = matching_brace(&s, head_end) {
                    impls.push((head_end, close, ty));
                }
            }
            // fn items.
            let mut bodies: Vec<(usize, usize)> = Vec::new();
            for k in token_starts(&s, "fn") {
                let after = &s[k + 2..];
                if !after.starts_with(' ') {
                    continue;
                }
                let name: String = after
                    .trim_start()
                    .chars()
                    .take_while(|c| is_ident(*c))
                    .collect();
                if name.is_empty() {
                    continue;
                }
                let Some(sig_end) = s[k..].find(['{', ';']).map(|e| k + e) else {
                    continue;
                };
                if s.as_bytes()[sig_end] == b';' {
                    continue;
                }
                let Some(close) = matching_brace(&s, sig_end) else {
                    continue;
                };
                let self_ty = impls
                    .iter()
                    .filter(|(o, c, _)| *o < k && k < *c)
                    .max_by_key(|(o, _, _)| *o)
                    .map(|(_, _, t)| t.clone());
                bodies.push((k, close));
                src.by_name
                    .entry(name.clone())
                    .or_default()
                    .push(src.fns.len());
                src.fns.push(FnItem {
                    file: file.clone(),
                    name,
                    self_ty,
                    mut_self: s[k..sig_end].contains("mut self"),
                    body: s[sig_end + 1..close].to_string(),
                });
            }
            // const and static items outside every fn body.
            for kw in ["const", "static"] {
                for k in token_starts(&s, kw) {
                    if bodies.iter().any(|(o, c)| *o < k && k < *c) {
                        continue;
                    }
                    let rest = s[k + kw.len()..].trim_start();
                    let name: String = rest.chars().take_while(|c| is_ident(*c)).collect();
                    if name.is_empty() || name.chars().any(|c| c.is_ascii_lowercase()) {
                        continue;
                    }
                    let Some(eq) = s[k..].find('=').map(|e| k + e) else {
                        continue;
                    };
                    let mut depth = 0i32;
                    let mut end = s.len();
                    for (i, ch) in s[eq..].char_indices() {
                        match ch {
                            '(' | '[' | '{' => depth += 1,
                            ')' | ']' | '}' => depth -= 1,
                            ';' if depth == 0 => {
                                end = eq + i;
                                break;
                            }
                            _ => {}
                        }
                    }
                    const_text.push((name, s[eq..end].to_string()));
                }
            }
        }
        // Edges named by each const, then through consts that name consts.
        for (name, text) in &const_text {
            let e = src.direct_edges(text);
            src.const_edges.entry(name.clone()).or_default().extend(e);
        }
        loop {
            let mut grew = false;
            for (name, text) in &const_text {
                let mut add = BTreeSet::new();
                for t in upper_tokens(text) {
                    if t != *name
                        && let Some(e) = src.const_edges.get(&t)
                    {
                        add.extend(e.iter().cloned());
                    }
                }
                let entry = src.const_edges.get_mut(name).unwrap();
                let before = entry.len();
                entry.extend(add);
                grew |= entry.len() > before;
            }
            if !grew {
                break;
            }
        }
        src
    }

    /// Edge types a text names itself: `edge::CONST` and quoted edge-type literals.
    fn direct_edges(&self, body: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut at = 0;
        while let Some(p) = body[at..].find("edge::") {
            let start = at + p + "edge::".len();
            at = start;
            let name: String = body[start..].chars().take_while(|c| is_ident(*c)).collect();
            if let Some(v) = self.edge_consts.get(&name) {
                out.insert(v.clone());
            }
        }
        for (k, _) in body.match_indices('"') {
            let lit: String = body[k + 1..].chars().take_while(|c| *c != '"').collect();
            if self.edge_types.contains(&lit) {
                out.insert(lit);
            }
        }
        out
    }

    /// The edge types a body names: directly, through a const or static item
    /// it uses (a table of edge types kept outside the function), or through a
    /// call to a schema selector.
    fn edges_in(&self, body: &str) -> BTreeSet<String> {
        let mut out = self.direct_edges(body);
        for t in upper_tokens(body) {
            if let Some(e) = self.const_edges.get(&t) {
                out.extend(e.iter().cloned());
            }
        }
        for (name, e) in &self.selectors {
            if token_starts(body, name)
                .iter()
                .any(|k| body[k + name.len()..].starts_with('('))
            {
                out.extend(e.iter().cloned());
            }
        }
        out
    }

    /// The functions a body calls, resolved by name and call form. A name
    /// defined more than once resolves to every definition that fits the form
    /// — the walk over-approximates rather than miss a read.
    fn calls(&self, f: usize) -> BTreeSet<usize> {
        let item = &self.fns[f];
        let body = &item.body;
        let mut out = BTreeSet::new();
        let bytes = body.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i] as char;
            if !(c.is_ascii_alphabetic() || c == '_') || (i > 0 && is_ident(bytes[i - 1] as char)) {
                i += 1;
                continue;
            }
            let start = i;
            while i < bytes.len() && is_ident(bytes[i] as char) {
                i += 1;
            }
            let name = &body[start..i];
            let mut j = i;
            if body[j..].starts_with("::<") {
                // turbofish: skip to the matching '>'
                let mut depth = 0i32;
                for (k, ch) in body[j + 2..].char_indices() {
                    match ch {
                        '<' => depth += 1,
                        '>' => {
                            depth -= 1;
                            if depth == 0 {
                                j = j + 2 + k + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
            }
            let before = &body[..start];
            // A method named by path without a call (`.map(Self::is_live)`)
            // is still a call the walk must follow.
            let by_path = before.ends_with("::")
                && before[..before.len() - 2]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_ascii_alphanumeric());
            let by_path = by_path
                && (before.ends_with("Self::")
                    || before[..before.len() - 2]
                        .rsplit(|c: char| !is_ident(c))
                        .next()
                        .is_some_and(|seg| seg.starts_with(|c: char| c.is_ascii_uppercase())));
            if bytes.get(j) != Some(&b'(') && !by_path {
                continue;
            }
            let Some(defs) = self.by_name.get(name) else {
                continue;
            };
            let form = if before.ends_with("self.") || before.ends_with("Self::") {
                CallForm::OnSelf
            } else if before.ends_with('.') {
                CallForm::OnOther
            } else if let Some(p) = before.strip_suffix("::") {
                let seg: String = p
                    .chars()
                    .rev()
                    .take_while(|c| is_ident(*c))
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect();
                if seg.starts_with(|c: char| c.is_ascii_uppercase()) {
                    CallForm::Assoc(seg)
                } else {
                    CallForm::Free
                }
            } else if before.trim_end().ends_with("fn") {
                continue; // a definition, not a call
            } else {
                CallForm::Free
            };
            let fits = |d: &usize| {
                if self.fns[*d].mut_self && !item.mut_self {
                    return false;
                }
                let t = self.fns[*d].self_ty.as_deref();
                match &form {
                    CallForm::OnSelf => t.is_some() && t == item.self_ty.as_deref(),
                    CallForm::OnOther => t == Some("DesignGraph"),
                    CallForm::Assoc(ty) => t == Some(ty.as_str()),
                    CallForm::Free => t.is_none(),
                }
            };
            let mut hit: Vec<usize> = defs.iter().copied().filter(fits).collect();
            if hit.is_empty() && matches!(form, CallForm::OnSelf) {
                // A trait's default method, or self of a type the index missed.
                hit = defs
                    .iter()
                    .copied()
                    .filter(|d| self.fns[*d].self_ty.is_some() && !self.fns[*d].mut_self)
                    .collect();
            }
            out.extend(hit.into_iter().filter(|d| *d != f));
        }
        out
    }

    /// The functions named `function` in `file`.
    fn named(&self, file: &str, function: &str) -> Vec<usize> {
        self.by_name
            .get(function)
            .into_iter()
            .flatten()
            .copied()
            .filter(|i| self.fns[*i].file == file)
            .collect()
    }

    /// Every function reachable from `starts` by calls, with the caller it was
    /// first reached from. A reached function for which `stop` holds is
    /// recorded but not entered.
    fn walk(
        &self,
        starts: &[usize],
        stop: &dyn Fn(usize) -> bool,
    ) -> BTreeMap<usize, Option<usize>> {
        let mut parent = BTreeMap::new();
        let mut queue = std::collections::VecDeque::new();
        for &s in starts {
            if parent.insert(s, None).is_none() {
                queue.push_back(s);
            }
        }
        while let Some(f) = queue.pop_front() {
            if !starts.contains(&f) && stop(f) {
                continue;
            }
            for g in self.calls(f) {
                if let std::collections::btree_map::Entry::Vacant(v) = parent.entry(g) {
                    v.insert(Some(f));
                    queue.push_back(g);
                }
            }
        }
        parent
    }

    fn label(&self, f: usize) -> String {
        format!("{}::{}", self.fns[f].file, self.fns[f].name)
    }

    /// `a → b → c`, from a walk's start to `f`.
    fn path(&self, parent: &BTreeMap<usize, Option<usize>>, f: usize) -> String {
        let mut chain = vec![f];
        let mut at = f;
        while let Some(Some(p)) = parent.get(&at) {
            chain.push(*p);
            at = *p;
        }
        chain.reverse();
        let mut s = self.label(chain[0]);
        for c in &chain[1..] {
            s.push_str(" → ");
            s.push_str(&self.fns[*c].name);
        }
        s
    }
}

/// Identifier tokens written all in capitals (const and static names).
fn upper_tokens(s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut cur = String::new();
    for ch in s.chars().chain(std::iter::once(' ')) {
        if is_ident(ch) {
            cur.push(ch);
        } else {
            if cur.len() > 1
                && cur.starts_with(|c: char| c.is_ascii_uppercase())
                && !cur.chars().any(|c| c.is_ascii_lowercase())
            {
                out.insert(cur.clone());
            }
            cur.clear();
        }
    }
    out
}

// ─── The declarations are well-formed ──────────────────────────────────────

fn primitives_of(r: &Reading) -> BTreeSet<String> {
    let mut s: BTreeSet<String> = r.components.iter().cloned().collect();
    if let Some(p) = &r.primitive {
        s.insert(p.clone());
    }
    s
}

#[test]
fn every_declaration_is_well_formed() {
    let g = graph();
    let decls = declarations();
    assert_eq!(decls.len(), 23, "the study's 23 relations are all declared");
    let ids: BTreeSet<&str> = decls.iter().map(|d| d.id.as_str()).collect();
    for d in &decls {
        let id = &d.id;
        assert!(
            READING_FORMS.contains(&d.reading.form.as_str()),
            "{id}: reading form {:?}",
            d.reading.form
        );
        assert_eq!(
            d.reading.form, "composite",
            "{id}: a derived relation is a composition"
        );
        assert!(
            d.reading.composition.is_some(),
            "{id}: composition is stated"
        );
        assert!(
            !d.reading.components.is_empty(),
            "{id}: components are listed"
        );
        for c in &d.reading.components {
            assert!(
                RELATION_PRIMITIVES.contains(&c.as_str()),
                "{id}: component {c:?} is not one of the sixteen primitives"
            );
        }
        assert!(
            STATED_OVER_READINGS.contains(&d.stated_over_readings.as_str()),
            "{id}: stated_over_readings {:?}",
            d.stated_over_readings
        );
        assert_eq!(
            d.not_yet_stated.is_some(),
            d.stated_over_readings != "full",
            "{id}: not_yet_stated says what is missing exactly when the rule is not fully stated"
        );
        assert!(
            INFERENCE_KINDS.contains(&d.inference.as_str()),
            "{id}: inference"
        );
        assert!(DERIVED_COSTS.contains(&d.cost.as_str()), "{id}: cost");
        assert!(!d.code.is_empty(), "{id}: names where the code computes it");
        assert_eq!(
            d.edges.is_empty(),
            d.stated_over_readings == "none",
            "{id}: a rule that reads no edge is stated over no reading, and only that one"
        );
        for o in &d.over {
            assert!(ids.contains(o.as_str()), "{id}: over {o:?} is not declared");
        }
        for e in &d.edges {
            assert!(
                g.schema().edge_types.contains_key(e),
                "{id}: edge {e} is not a schema edge type"
            );
        }
    }
}

#[test]
fn the_declarations_and_the_evaluators_name_the_same_relations() {
    let declared: BTreeSet<String> = declarations().into_iter().map(|d| d.id).collect();
    let evaluated: BTreeSet<String> = DERIVED_EVALUATORS.iter().map(|s| s.to_string()).collect();
    let undeclared: Vec<_> = evaluated.difference(&declared).collect();
    let unevaluated: Vec<_> = declared.difference(&evaluated).collect();
    assert!(
        undeclared.is_empty() && unevaluated.is_empty(),
        "evaluators with no declaration: {undeclared:?}; declarations with no evaluator: {unevaluated:?}"
    );
}

// ─── The declarations are held to the code ────────────────────────────────

/// Every relation `d` is computed over, transitively.
fn over_star<'a>(
    d: &'a DerivedRelation,
    by_id: &HashMap<&str, &'a DerivedRelation>,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut stack: Vec<&str> = d.over.iter().map(String::as_str).collect();
    while let Some(o) = stack.pop() {
        if out.insert(o.to_string()) {
            stack.extend(by_id[o].over.iter().map(String::as_str));
        }
    }
    out
}

/// What the declarations say about the code, resolved against the source.
struct Held<'a> {
    src: Source,
    decls: &'a [DerivedRelation],
    by_id: HashMap<&'a str, &'a DerivedRelation>,
    /// fn index → the relations it is a `dedicated` site of.
    dedicated_to: HashMap<usize, BTreeSet<String>>,
    /// fn index → every relation that names it at all.
    named_by: HashMap<usize, BTreeSet<String>>,
    /// Functions a declaration names that do not exist.
    missing: Vec<String>,
}

impl<'a> Held<'a> {
    fn new(src: Source, decls: &'a [DerivedRelation]) -> Self {
        let by_id: HashMap<&str, &DerivedRelation> =
            decls.iter().map(|d| (d.id.as_str(), d)).collect();
        let mut dedicated_to: HashMap<usize, BTreeSet<String>> = HashMap::new();
        let mut named_by: HashMap<usize, BTreeSet<String>> = HashMap::new();
        let mut missing = Vec::new();
        for d in decls {
            for c in &d.code {
                let hits = src.named(&c.file, &c.function);
                if hits.is_empty() {
                    missing.push(format!("{}: fn {} is not in {}", d.id, c.function, c.file));
                }
                for f in hits {
                    if src.fns[f].mut_self {
                        missing.push(format!(
                            "{}: {}::{} takes &mut self — a writer, which records facts and computes no relation",
                            d.id, c.file, c.function
                        ));
                    }
                    named_by.entry(f).or_default().insert(d.id.clone());
                    if c.dedicated {
                        dedicated_to.entry(f).or_default().insert(d.id.clone());
                    }
                }
            }
        }
        Held {
            src,
            decls,
            by_id,
            dedicated_to,
            named_by,
            missing,
        }
    }

    fn own_sites(&self, d: &DerivedRelation, dedicated_only: bool) -> Vec<usize> {
        d.code
            .iter()
            .filter(|c| c.dedicated || !dedicated_only)
            .flat_map(|c| self.src.named(&c.file, &c.function))
            .collect()
    }

    /// Where a walk for `d` stops: at code that is ANOTHER relation's own
    /// (a dedicated site of it that `d` does not name), which that relation's
    /// declaration answers for.
    fn boundary(&self, d: &DerivedRelation, f: usize) -> Option<&BTreeSet<String>> {
        let other = self.dedicated_to.get(&f)?;
        let own = self.named_by.get(&f).is_some_and(|r| r.contains(&d.id));
        (!own && !other.contains(&d.id)).then_some(other)
    }

    /// Everything `d`'s own code reaches, stopping at other relations' code.
    fn reach(&self, d: &DerivedRelation, starts: &[usize]) -> BTreeMap<usize, Option<usize>> {
        self.src.walk(starts, &|f| self.boundary(d, f).is_some())
    }

    /// The edges `d` may read: its own and those of every relation it is
    /// computed over.
    fn allowed(&self, d: &DerivedRelation) -> BTreeSet<String> {
        let mut allowed: BTreeSet<String> = d.edges.iter().cloned().collect();
        for o in over_star(d, &self.by_id) {
            allowed.extend(self.by_id[o.as_str()].edges.iter().cloned());
        }
        allowed
    }

    /// Every disagreement between the declarations and the code they name.
    fn problems(&self) -> Vec<String> {
        let mut problems = self.missing.clone();
        for d in self.decls {
            let over = over_star(d, &self.by_id);
            let allowed = self.allowed(d);
            // Code with no declaration: everything a DEDICATED function
            // reaches, however deep, reads only declared edges, and computes
            // over only relations the declaration names.
            for s in self.own_sites(d, true) {
                let parent = self.reach(d, &[s]);
                for &f in parent.keys() {
                    if let Some(others) = self.boundary(d, f) {
                        if others.is_disjoint(&over) {
                            problems.push(format!(
                                "{}: {} reaches {} code ({}), and `over` does not name it",
                                d.id,
                                self.src.path(&parent, f),
                                others.iter().cloned().collect::<Vec<_>>().join(", "),
                                self.src.label(f),
                            ));
                        }
                        continue;
                    }
                    for e in self
                        .src
                        .edges_in(&self.src.fns[f].body)
                        .difference(&allowed)
                    {
                        problems.push(format!(
                            "{}: {} reads {e}, which the declaration does not list",
                            d.id,
                            self.src.path(&parent, f)
                        ));
                    }
                }
            }
            // A declaration with no code: every edge it lists is read by
            // something its own functions reach.
            let parent = self.reach(d, &self.own_sites(d, false));
            let mut read = BTreeSet::new();
            for &f in parent.keys() {
                if self.boundary(d, f).is_none() {
                    read.extend(self.src.edges_in(&self.src.fns[f].body));
                }
            }
            for e in d.edges.iter().filter(|e| !read.contains(*e)) {
                problems.push(format!(
                    "{}: declares {e}, but nothing its functions reach reads it",
                    d.id
                ));
            }
        }
        problems.sort();
        problems.dedup();
        problems
    }

    /// Functions the declarations answer for: every function one names, and
    /// everything a dedicated function reaches short of another relation's code.
    fn covered(&self) -> BTreeSet<usize> {
        let mut covered: BTreeSet<usize> = self.named_by.keys().copied().collect();
        for d in self.decls {
            for s in self.own_sites(d, true) {
                for &f in self.reach(d, &[s]).keys() {
                    if self.boundary(d, f).is_none() {
                        covered.insert(f);
                    }
                }
            }
        }
        covered
    }

    /// Every function that names an edge type and that no declaration answers
    /// for, as `file::fn` (a name defined twice in one file is one entry).
    fn unanswered_readers(&self) -> BTreeMap<String, BTreeSet<String>> {
        let covered = self.covered();
        let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (i, f) in self.src.fns.iter().enumerate() {
            if covered.contains(&i) {
                continue;
            }
            let edges = self.src.edges_in(&f.body);
            if !edges.is_empty() {
                out.entry(format!("{}::{}", f.file, f.name))
                    .or_default()
                    .extend(edges);
            }
        }
        out
    }
}

#[test]
fn every_declaration_matches_everything_its_code_reaches() {
    let g = graph();
    let decls = declarations();
    let held = Held::new(Source::load(&g), &decls);
    let problems = held.problems();
    assert!(
        problems.is_empty(),
        "declarations out of step with the code:\n{}",
        problems.join("\n")
    );
}

// ─── Every function that names an edge is answered for ─────────────────────

/// `schema/derived/edge_readers.yaml`: every function that names an edge type
/// and that no declaration answers for, with the edges it names.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ReaderList {
    readers: BTreeMap<String, ListedReader>,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ListedReader {
    /// `unjudged` (on the list when it was first drawn, and nobody has said
    /// whether it derives a relation) or `not_derived` (judged; `why` says
    /// what it does instead).
    kind: String,
    edges: Vec<String>,
    #[serde(default)]
    why: Option<String>,
}

/// The unjudged entries the list may hold. It ONLY SHRINKS: judging an entry
/// lowers it, and a new function is judged, never added as unjudged.
const UNJUDGED_AT_MOST: usize = 130;

fn reader_list() -> ReaderList {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../schema/derived/edge_readers.yaml");
    let text = std::fs::read_to_string(&p).expect("read schema/derived/edge_readers.yaml");
    serde_yaml_ng::from_str(&text).expect("schema/derived/edge_readers.yaml parses")
}

/// What `held` says every unanswered function reads, and what the list says,
/// compared. Split out so the check's own net can run it on synthetic source.
fn reader_problems(
    found: &BTreeMap<String, BTreeSet<String>>,
    list: &ReaderList,
) -> (Vec<String>, usize) {
    let mut problems = Vec::new();
    for (f, edges) in found {
        match list.readers.get(f) {
            None => problems.push(format!(
                "{f} names {edges:?} and no declaration answers for it: declare the relation it \
                 computes in schema/derived/relations.yaml, or list it in \
                 schema/derived/edge_readers.yaml as `not_derived` with `why`"
            )),
            Some(l) => {
                let listed: BTreeSet<String> = l.edges.iter().cloned().collect();
                if &listed != edges {
                    problems.push(format!(
                        "{f} now names {edges:?}, and the list says {listed:?}: judge the change — \
                         a new edge read is where a new derived relation starts"
                    ));
                }
            }
        }
    }
    let mut unjudged = 0usize;
    for (f, l) in &list.readers {
        if !found.contains_key(f) {
            problems.push(format!(
                "{f} is listed, but it is gone, names no edge, or a declaration now answers for \
                 it: take it off the list"
            ));
        }
        match l.kind.as_str() {
            "unjudged" => unjudged += 1,
            "not_derived" => {
                if l.why.as_deref().is_none_or(|w| w.trim().is_empty()) {
                    problems.push(format!("{f}: `not_derived` says why"));
                }
            }
            other => problems.push(format!(
                "{f}: kind {other:?} is not unjudged or not_derived"
            )),
        }
    }
    (problems, unjudged)
}

#[test]
fn every_function_that_names_an_edge_is_declared_or_listed() {
    let g = graph();
    let decls = declarations();
    let held = Held::new(Source::load(&g), &decls);
    let found = held.unanswered_readers();
    let (mut problems, unjudged) = reader_problems(&found, &reader_list());
    if unjudged > UNJUDGED_AT_MOST {
        problems.push(format!(
            "{unjudged} entries are unjudged, and at most {UNJUDGED_AT_MOST} may be: a function \
             added to the list is judged, never listed as unjudged"
        ));
    } else if unjudged < UNJUDGED_AT_MOST {
        problems.push(format!(
            "{unjudged} entries are unjudged: lower UNJUDGED_AT_MOST to {unjudged}, so the list \
             only shrinks"
        ));
    }
    assert!(
        problems.is_empty(),
        "functions that name an edge type, against schema/derived/edge_readers.yaml:\n{}",
        problems.join("\n")
    );
}

// ─── The rule's own text ───────────────────────────────────────────────────

#[test]
fn every_components_list_is_the_primitives_its_composition_writes() {
    let mut problems = Vec::new();
    for d in declarations() {
        let listed: BTreeSet<&str> = d.reading.components.iter().map(String::as_str).collect();
        let written: BTreeSet<&str> = d.reading.composition_primitives().into_iter().collect();
        if listed != written {
            problems.push(format!(
                "{}: components {:?}, but the composition writes {:?} (extra {:?}, missing {:?})",
                d.id,
                listed,
                written,
                listed.difference(&written).collect::<Vec<_>>(),
                written.difference(&listed).collect::<Vec<_>>(),
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "a declaration's components are what its rule states, not the edges it reads:\n{}",
        problems.join("\n")
    );
}

fn edge_primitives(g: &DesignGraph, e: &str) -> BTreeSet<String> {
    let def = &g.schema().edge_types[e];
    let reading = def
        .reading
        .as_ref()
        .unwrap_or_else(|| panic!("edge {e} has no reading"));
    let mut prims = primitives_of(&reading.reading);
    for split in &reading.splits {
        for r in split.values.values() {
            prims.extend(primitives_of(r));
        }
    }
    prims
}

#[test]
fn every_declared_edge_is_stated_by_the_rule_or_named_as_not_yet_stated() {
    let g = graph();
    let decls = declarations();
    let by_id: HashMap<&str, &DerivedRelation> = decls.iter().map(|d| (d.id.as_str(), d)).collect();
    let mut problems = Vec::new();
    for d in &decls {
        let mut stated: BTreeSet<String> = d
            .reading
            .composition_primitives()
            .into_iter()
            .map(str::to_string)
            .collect();
        for o in over_star(d, &by_id) {
            stated.extend(
                by_id[o.as_str()]
                    .reading
                    .composition_primitives()
                    .into_iter()
                    .map(str::to_string),
            );
        }
        for e in &d.edges {
            let prims = edge_primitives(&g, e);
            let named = d
                .not_yet_stated
                .as_deref()
                .is_some_and(|n| n.contains(e.as_str()));
            if prims.is_disjoint(&stated) && !named {
                problems.push(format!(
                    "{}: reads {e} ({prims:?}), which no primitive of its rule {stated:?} states, \
                     and not_yet_stated does not name it",
                    d.id
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

// ─── The scanner's own net ─────────────────────────────────────────────────

/// The scanner itself: blanking keeps an edge literal, drops prose, and does
/// not mistake a lifetime for a char literal.
#[test]
fn the_scanner_reads_what_it_should() {
    let s = blank(
        "fn f<'a>(x: &'a str) { let y = \"REALIZES\"; let z = \"not { an edge\"; // edge::VERIFIES\n self.g(edge::SATISFIES) }",
    );
    assert!(s.contains("\"REALIZES\""));
    assert!(!s.contains("VERIFIES"));
    assert!(!s.contains("not"));
    assert!(s.contains("edge::SATISFIES"));
}

/// A synthetic crate with every shape of read the first version missed.
fn synthetic() -> Source {
    let consts: HashMap<String, String> = ["REALIZES", "VERIFIES", "HAS_READINESS", "BLOCKS"]
        .iter()
        .map(|e| (e.to_string(), e.to_string()))
        .collect();
    let types: BTreeSet<String> = consts.values().cloned().collect();
    let files: BTreeMap<String, String> = [
        (
            "gate.rs",
            r#"
            const RISK: &[&str] = &["BLOCKS"];
            impl DesignGraph {
                pub fn gate(&self) -> usize { self.resolve() + Self::risky(self) }
                fn resolve(&self) -> usize { self.best() }
                fn best(&self) -> usize { self.outgoing(edge::HAS_READINESS).len() }
                fn risky(&self) -> usize { RISK.len() }
                pub fn record(&mut self) { self.link(edge::VERIFIES) }
                fn peek(&self) -> usize { helpers::count(self) }
            }
            "#,
        ),
        (
            "helpers.rs",
            r#"
            pub fn count(g: &DesignGraph) -> usize { g.realized() }
            impl DesignGraph {
                fn realized(&self) -> usize { self.outgoing("REALIZES").len() }
                fn link(&mut self, e: &str) {}
            }
            #[cfg(test)]
            mod tests {
                fn realized_in_a_test() { let _ = "VERIFIES"; }
            }
            "#,
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    Source::from_files(files, consts, types)
}

fn reached(src: &Source, from: &str) -> BTreeSet<String> {
    let start = src.named("gate.rs", from);
    let parent = src.walk(&start, &|_| false);
    parent
        .keys()
        .flat_map(|f| src.edges_in(&src.fns[*f].body))
        .collect()
}

#[test]
fn the_walk_sees_a_read_two_calls_deep_through_a_const_and_across_files() {
    let src = synthetic();
    let gate = reached(&src, "gate");
    // gate → resolve → best: two calls deep, as readiness_gate read HAS_READINESS.
    assert!(gate.contains("HAS_READINESS"), "{gate:?}");
    // gate → Self::risky, which names BLOCKS only through a const table, as
    // impact read its risk edges through RISK_EDGES.
    assert!(gate.contains("BLOCKS"), "{gate:?}");
    // peek → helpers::count (a free fn in another file) → g.realized().
    let peek = reached(&src, "peek");
    assert!(peek.contains("REALIZES"), "{peek:?}");
    // A reader never enters a writer, and test code is not source.
    assert!(!gate.contains("VERIFIES") && !peek.contains("VERIFIES"));
    assert!(src.fns.iter().all(|f| f.name != "realized_in_a_test"));
    assert!(src.fns.iter().any(|f| f.name == "record" && f.mut_self));
}

#[test]
fn a_function_reading_an_edge_that_nothing_answers_for_is_named() {
    let src = synthetic();
    let decl: DerivedRelation = serde_yaml_ng::from_str(
        r#"
        name: gate
        reading: { form: composite, composition: "g := measures(r, t)", components: [measures] }
        rule: r
        stated_over_readings: partial
        not_yet_stated: n
        inference: deduced
        edges: [HAS_READINESS]
        code: [ { file: gate.rs, function: resolve, dedicated: true } ]
        counts: c
        cost: bounded
        "#,
    )
    .expect("a synthetic declaration");
    let mut decl = decl;
    decl.id = "gate".to_string();
    let decls = vec![decl];
    let held = Held::new(src, &decls);
    let found = held.unanswered_readers();
    // resolve → best is answered for; risky, realized and record are not.
    assert!(!found.contains_key("gate.rs::best"), "{found:?}");
    for f in ["gate.rs::risky", "helpers.rs::realized", "gate.rs::record"] {
        assert!(found.contains_key(f), "{f} is not named: {found:?}");
    }
    let empty = ReaderList {
        readers: BTreeMap::new(),
    };
    let (problems, _) = reader_problems(&found, &empty);
    assert!(
        problems.iter().any(|p| p.starts_with("gate.rs::risky")),
        "{problems:?}"
    );
    // The declaration held to its reach: nothing undeclared below `resolve`.
    assert!(held.problems().is_empty(), "{:?}", held.problems());
}

// ─── The served read counts what the code computes ────────────────────────

fn s(v: &str) -> Value {
    Value::String(v.to_string())
}

fn props(pairs: &[(&str, &str)]) -> HashMap<String, Value> {
    pairs.iter().map(|(k, v)| (k.to_string(), s(v))).collect()
}

fn fixture() -> DesignGraph {
    let mut g = graph();
    let n = |g: &mut DesignGraph, t: &str, id: &str, extra: &[(&str, &str)]| {
        let mut p = props(&[("name", id)]);
        p.extend(props(extra));
        g.create_node(t, id, p)
            .unwrap_or_else(|e| panic!("create {id}: {e}"));
    };
    n(&mut g, node::REQUIREMENT, "req:a", &[("statement", "a")]);
    n(&mut g, node::REQUIREMENT, "req:b", &[("statement", "b")]);
    for c in ["cap:a", "cap:b", "cap:c"] {
        n(&mut g, node::CAPABILITY, c, &[("description", c)]);
    }
    for k in ["cmp:x", "cmp:y"] {
        n(&mut g, node::COMPONENT, k, &[("purpose", k)]);
    }
    n(&mut g, node::ARTIFACT, "art:x", &[]);
    n(
        &mut g,
        node::VERIFICATION,
        "ver:a",
        &[("status", "passing")],
    );
    n(
        &mut g,
        node::DECISION,
        "dec:o",
        &[("decision", "retire cap:c"), ("status", "accepted")],
    );
    n(&mut g, node::INTERFACE, "ifc:i", &[]);
    let e = |g: &mut DesignGraph, t: &str, ft: &str, f: &str, tt: &str, to: &str| {
        g.create_edge(t, ft, f, tt, to, HashMap::new())
            .unwrap_or_else(|err| panic!("{t} {f}->{to}: {err}"));
    };
    e(
        &mut g,
        edge::REALIZES,
        node::ARTIFACT,
        "art:x",
        node::CAPABILITY,
        "cap:a",
    );
    e(
        &mut g,
        edge::REALIZES,
        node::ARTIFACT,
        "art:x",
        node::COMPONENT,
        "cmp:x",
    );
    e(
        &mut g,
        edge::VERIFIES,
        node::VERIFICATION,
        "ver:a",
        node::CAPABILITY,
        "cap:a",
    );
    e(
        &mut g,
        edge::SATISFIES,
        node::CAPABILITY,
        "cap:a",
        node::REQUIREMENT,
        "req:a",
    );
    e(
        &mut g,
        edge::SATISFIES,
        node::CAPABILITY,
        "cap:b",
        node::REQUIREMENT,
        "req:b",
    );
    e(
        &mut g,
        edge::ALLOCATED_TO,
        node::CAPABILITY,
        "cap:b",
        node::COMPONENT,
        "cmp:x",
    );
    e(
        &mut g,
        edge::OBSOLETES,
        node::DECISION,
        "dec:o",
        node::CAPABILITY,
        "cap:c",
    );
    e(
        &mut g,
        edge::PROVIDES,
        node::COMPONENT,
        "cmp:x",
        node::INTERFACE,
        "ifc:i",
    );
    e(
        &mut g,
        edge::CONSUMES,
        node::COMPONENT,
        "cmp:y",
        node::INTERFACE,
        "ifc:i",
    );
    e(
        &mut g,
        edge::DEPENDS_ON,
        node::COMPONENT,
        "cmp:y",
        node::COMPONENT,
        "cmp:x",
    );
    g
}

#[test]
fn the_served_read_counts_what_the_code_computes() {
    let g = fixture();
    let r = g.derived_report(&[], 3).expect("derived_report");
    let count = |id: &str| {
        r.relations
            .iter()
            .find(|t| t.id == id)
            .unwrap_or_else(|| panic!("{id} in the report"))
            .count
    };
    assert_eq!(r.declared, 23);
    assert_eq!(r.relations.len(), 23, "every declared relation is reported");

    // By hand: cap:a is realized directly, cap:b through cmp:x; cap:c is withdrawn.
    assert_eq!(count("realized"), Some(2));
    assert_eq!(count("checked"), Some(1), "only cap:a has a passing check");
    assert_eq!(
        count("delivered"),
        Some(1),
        "req:a; req:b's capability is realized but unchecked"
    );
    assert_eq!(count("discontinued"), Some(1));
    assert_eq!(count("contract_pair"), Some(1));
    assert_eq!(count("coupling"), Some(1));

    // And by the code path each declaration names.
    assert_eq!(
        count("delivered"),
        Some(g.delivery_coverage().unwrap().delivered)
    );
    assert_eq!(
        count("discontinued"),
        Some(g.discontinued_ids().unwrap().len())
    );
    let seams = g.seam_coverage(None).unwrap();
    assert_eq!(count("contract_pair"), Some(seams.declared));
    assert_eq!(count("coupling"), Some(seams.couplings));
    assert_eq!(
        count("rerun_owed"),
        Some(g.invalidated_findings().unwrap().len())
    );
    assert_eq!(
        count("level_mismatch"),
        Some(g.hierarchy_issues().unwrap().len())
    );

    // The closure is kept as a rule and not counted, and says why.
    let impact = r.relations.iter().find(|t| t.id == "impact").unwrap();
    assert_eq!(impact.count, None);
    assert!(impact.not_counted.is_some());
    assert_eq!(r.counted, 22);

    // Nothing is stored: reading twice changes nothing and asserted facts hold.
    let again = g.derived_report(&[], 3).expect("second read");
    assert_eq!(again.asserted_facts, r.asserted_facts);
    assert_eq!(again.derived_facts, r.derived_facts);
    assert!(r.asserted_facts > 0 && r.derived_per_asserted.is_some());
}

#[test]
fn a_relation_nobody_declared_is_refused_not_answered_empty() {
    let g = fixture();
    let err = g
        .derived_report(&["no_such_relation".to_string()], 3)
        .expect_err("an undeclared id is refused");
    assert!(err.to_string().contains("no_such_relation"));
    let one = g
        .derived_report(&["realized".to_string()], 3)
        .expect("a declared id narrows the read");
    assert_eq!(one.relations.len(), 1);
    assert_eq!(one.relations[0].count, Some(2));
}
