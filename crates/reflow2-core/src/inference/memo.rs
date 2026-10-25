//! ONE EVALUATION PER REPORT, AND A CIRCLE WORKED OUT ONCE
//! (`dec:reasoning-evaluation-is-memoized-across-cycles-and-shared-across-a-report`,
//! Anthony 2026-10-25, after the Shop stall on flo2.io).
//!
//! Until this module, every reader made its own evaluation ([`super::Eval`])
//! with its own memo, so a report reading 300 claims read the reasoning
//! beneath them 300 times; and a truth computed while a circle was cut beneath
//! it was never kept, so on a design with circles (objections that answer each
//! other, a claim that comes back to itself) everything above the circle was
//! computed again each time it was asked, which grows exponentially with the
//! chains above it. A design of about 600 nodes and 56 steps took
//! reasoning_report 13.3 s and loop_status past the gateway's 30 s.
//!
//! TWO THINGS ARE KEPT HERE, AND NEITHER CHANGES A READING.
//!
//! 1. A SCOPE ([`DesignGraph::reasoning_scope`]): while a report, loop_status,
//!    detect_gaps or a what-if runs, every evaluation of that graph on this
//!    thread shares ONE memo, per set of premises counted as lost.
//!
//! 2. A READING KEPT WITH WHAT IT DEPENDS ON ([`Kept`]). A circle is read as it
//!    always was: a claim met again on the evaluation's own stack "comes back
//!    to itself ... a circle has no verdict", so what a node reads can depend
//!    on which nodes are on the stack when it is asked. A reading is therefore
//!    kept with exactly that: the nodes and steps of the stack it was cut on
//!    (`on`), and every node and step it consulted and found off the stack
//!    (`off`). Asked again, it is handed back only when the stack agrees on
//!    every one of them, which is when computing it again would take the same
//!    path and give the same reading. So each circle is worked out once per
//!    way into it, never once per path through everything above it.
//!
//! A reading that met the depth limit ([`super::INFERENCE_DEPTH`]) depends on
//! how deep it was asked, so it is never shared: it stays in its own
//! evaluation's memo, as before.
//!
//! While a scope is open it also keeps two answers that are facts about the
//! graph alone: which single type holds an id (`type_holding` reads every
//! schema type to say, and a report asks tens of thousands of times), and the
//! design's reasoning debt (loop_status reads it three times). Measured on
//! the Shop's export, release build: reasoning_report 2.6 s before, 0.52 s
//! with the shared evaluation, 0.25 s with these two as well.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::rc::Rc;

use super::Read;
use crate::graph::DesignGraph;

/// A node or a step as the evaluation's stacks hold it, interned:
/// `index << 1 | kind`.
pub(super) type Key = u32;

/// Which stack: the claims being read (`nodes`) or the steps (`path`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Stack {
    Node = 0,
    Step = 1,
}

/// How many readings of one node, each for a different state of the stack,
/// are kept. Past it a reading is computed again, as it always was.
const VARIANTS_CAP: usize = 16;

thread_local! {
    /// How many times any evaluation on this thread met the depth limit.
    /// Only ever compared before and after a computation.
    static TRUNCATIONS: Cell<u64> = const { Cell::new(0) };
    /// The open scope: which graph, how many guards hold it, and its memo.
    static SCOPE: RefCell<Option<Open>> = const { RefCell::new(None) };
}

/// The open scope: which graph (its address, stable while it is borrowed),
/// how many guards hold it, and its memo.
struct Open {
    graph: usize,
    holders: usize,
    memo: Rc<RefCell<Shared>>,
}

/// The open scope's memo, when one is open for `graph` on this thread.
fn open_memo(graph: &DesignGraph) -> Option<Rc<RefCell<Shared>>> {
    let at = graph as *const DesignGraph as usize;
    SCOPE.with(|s| match &*s.borrow() {
        Some(open) if open.graph == at => Some(open.memo.clone()),
        _ => None,
    })
}

/// Say that an evaluation met the depth limit: what it read depends on how
/// deep it was asked, so it, and everything resting on it, is not shared.
pub(crate) fn note_truncation() {
    TRUNCATIONS.with(|t| t.set(t.get() + 1));
}

pub(super) fn truncations() -> u64 {
    TRUNCATIONS.with(Cell::get)
}

/// One reading, kept with what it depends on (module documentation, 2).
pub(super) struct Kept {
    pub(super) read: Read,
    /// Stack entries it was cut on that were on the stack before it was asked:
    /// each must be on the stack again for it to be handed back.
    on: Box<[Key]>,
    /// Every stack entry it consulted and found OFF the stack, sorted: none
    /// may be on the stack for it to be handed back.
    off: Box<[Key]>,
    /// A circle was cut somewhere beneath it.
    pub(super) had_cuts: bool,
    /// It met the depth limit somewhere beneath it.
    pub(super) truncated: bool,
}

/// What one computation consulted, gathered while it runs.
#[derive(Default)]
pub(super) struct Frame {
    on: HashSet<Key>,
    off: HashSet<Key>,
}

/// The memo shared across evaluations: an interner for stack entries, and one
/// table of kept readings per set of premises counted as lost.
#[derive(Default)]
pub(super) struct Shared {
    /// Which single type holds each id asked about (`type_holding`): asking
    /// costs a read per schema type, and a report asks tens of thousands of
    /// times. The graph cannot change while the scope is open.
    types: HashMap<String, Option<String>>,
    /// The design's reasoning debt (`reasoning_debt`), which loop_status
    /// reads three times over: once itself and once each through two
    /// detectors.
    debt: Option<Option<crate::reasoning::ReasoningDebt>>,
    ids: HashMap<String, u32>,
    names: Vec<String>,
    tables: Vec<HashMap<String, Vec<Rc<Kept>>>>,
    table_of: HashMap<Vec<String>, usize>,
}

impl Shared {
    fn key(&mut self, id: &str, stack: Stack) -> Key {
        let i = match self.ids.get(id) {
            Some(i) => *i,
            None => {
                let i = self.names.len() as u32;
                self.names.push(id.to_string());
                self.ids.insert(id.to_string(), i);
                i
            }
        };
        (i << 1) | stack as u32
    }

    fn known(&self, id: &str, stack: Stack) -> Option<Key> {
        self.ids.get(id).map(|i| (i << 1) | stack as u32)
    }

    pub(super) fn table(&mut self, lost: &BTreeSet<String>) -> usize {
        let lost: Vec<String> = lost.iter().cloned().collect();
        if let Some(t) = self.table_of.get(&lost) {
            return *t;
        }
        self.tables.push(HashMap::new());
        let t = self.tables.len() - 1;
        self.table_of.insert(lost, t);
        t
    }
}

/// The memo an evaluation of `graph` uses: the open scope's when one is open
/// for this graph on this thread, otherwise one of its own.
pub(super) fn memo_for(graph: &DesignGraph) -> Rc<RefCell<Shared>> {
    open_memo(graph).unwrap_or_else(|| Rc::new(RefCell::new(Shared::default())))
}

/// `type_holding` for `id`, from the open scope when one is open for this
/// graph, else `compute`d.
pub(super) fn type_held<E>(
    graph: &DesignGraph,
    id: &str,
    compute: impl FnOnce() -> Result<Option<String>, E>,
) -> Result<Option<String>, E> {
    let Some(memo) = open_memo(graph) else {
        return compute();
    };
    if let Some(t) = memo.borrow().types.get(id) {
        return Ok(t.clone());
    }
    let t = compute()?;
    memo.borrow_mut().types.insert(id.to_string(), t.clone());
    Ok(t)
}

/// `reasoning_debt`, from the open scope when one is open for this graph,
/// else `compute`d.
pub(crate) fn debt_held<E>(
    graph: &DesignGraph,
    compute: impl FnOnce() -> Result<Option<crate::reasoning::ReasoningDebt>, E>,
) -> Result<Option<crate::reasoning::ReasoningDebt>, E> {
    let Some(memo) = open_memo(graph) else {
        return compute();
    };
    if let Some(d) = &memo.borrow().debt {
        return Ok(d.clone());
    }
    let d = compute()?;
    memo.borrow_mut().debt = Some(d.clone());
    Ok(d)
}

/// While held, every evaluation of one graph on this thread shares one memo.
/// Opened by [`DesignGraph::reasoning_scope`]; nested guards share the
/// outermost one's memo, and the memo goes when the last guard does.
pub(crate) struct ReasoningScope {
    held: bool,
}

impl Drop for ReasoningScope {
    fn drop(&mut self) {
        if !self.held {
            return;
        }
        SCOPE.with(|s| {
            let mut s = s.borrow_mut();
            let last = match s.as_mut() {
                Some(open) => {
                    open.holders -= 1;
                    open.holders == 0
                }
                None => false,
            };
            if last {
                *s = None;
            }
        });
    }
}

impl DesignGraph {
    /// Share one evaluation of this design's reasoning across everything read
    /// until the guard drops: a report, loop_status, detect_gaps or a what-if
    /// reads many nodes, and each node's reasoning is worked out once for all
    /// of them. The graph cannot change while it is borrowed, so nothing kept
    /// goes stale. A scope already open for another graph (a design read from
    /// inside another) is left alone, and this graph's evaluations keep their
    /// own memos.
    pub(crate) fn reasoning_scope(&self) -> ReasoningScope {
        let at = self as *const DesignGraph as usize;
        SCOPE.with(|s| {
            let mut s = s.borrow_mut();
            match s.as_mut() {
                None => {
                    *s = Some(Open {
                        graph: at,
                        holders: 1,
                        memo: Rc::new(RefCell::new(Shared::default())),
                    });
                    ReasoningScope { held: true }
                }
                Some(open) if open.graph == at => {
                    open.holders += 1;
                    ReasoningScope { held: true }
                }
                Some(_) => ReasoningScope { held: false },
            }
        })
    }
}

/// The two stacks of an evaluation, as [`Kept`] is checked against them.
pub(super) struct Stacks<'a> {
    pub(super) nodes: &'a BTreeSet<String>,
    pub(super) path: &'a BTreeSet<String>,
}

impl Stacks<'_> {
    fn holds(&self, shared: &Shared, key: Key) -> bool {
        let id = &shared.names[(key >> 1) as usize];
        if key & 1 == Stack::Node as u32 {
            self.nodes.contains(id)
        } else {
            self.path.contains(id)
        }
    }

    /// Whether `kept` reads now as it did when it was computed: every entry
    /// it was cut on is on the stack, and nothing it found off the stack is
    /// on it.
    fn agree(&self, shared: &Shared, kept: &Kept) -> bool {
        if !kept.on.iter().all(|k| self.holds(shared, *k)) {
            return false;
        }
        if kept.off.is_empty() {
            return true;
        }
        let off = |id: &String, stack: Stack| {
            shared
                .known(id, stack)
                .is_some_and(|k| kept.off.binary_search(&k).is_ok())
        };
        !(self.nodes.iter().any(|id| off(id, Stack::Node))
            || self.path.iter().any(|id| off(id, Stack::Step)))
    }
}

/// The bookkeeping one evaluation does against its memo.
pub(super) struct Tracker {
    pub(super) shared: Rc<RefCell<Shared>>,
    pub(super) table: usize,
    pub(super) frames: Vec<Frame>,
}

impl Tracker {
    pub(super) fn new(graph: &DesignGraph, lost: &BTreeSet<String>) -> Self {
        let shared = memo_for(graph);
        let table = shared.borrow_mut().table(lost);
        Tracker {
            shared,
            table,
            frames: Vec::new(),
        }
    }

    /// The computation under way consulted `id` and found it ON the stack.
    pub(super) fn found_on(&mut self, id: &str, stack: Stack) {
        if let Some(top) = self.frames.last_mut() {
            let k = self.shared.borrow_mut().key(id, stack);
            top.on.insert(k);
        }
    }

    /// The computation under way consulted `id` and found it OFF the stack.
    pub(super) fn found_off(&mut self, id: &str, stack: Stack) {
        if let Some(top) = self.frames.last_mut() {
            let k = self.shared.borrow_mut().key(id, stack);
            top.off.insert(k);
        }
    }

    /// The computation under way took `kept`, the reading of node `id`: it
    /// depends on everything that did, and on `id` being off the stack.
    pub(super) fn took(&mut self, id: &str, kept: &Kept) {
        if kept.truncated {
            note_truncation();
        }
        if let Some(top) = self.frames.last_mut() {
            top.on.extend(kept.on.iter().copied());
            top.off.extend(kept.off.iter().copied());
            let k = self.shared.borrow_mut().key(id, Stack::Node);
            top.off.insert(k);
        }
    }

    /// A shared reading of `id` that holds on the stack as it stands.
    pub(super) fn recall(&self, id: &str, stacks: &Stacks<'_>) -> Option<Rc<Kept>> {
        let shared = self.shared.borrow();
        shared.tables[self.table]
            .get(id)?
            .iter()
            .find(|k| stacks.agree(&shared, k))
            .cloned()
    }

    /// Keep the reading just computed for `id` from what `frame` gathered,
    /// once `id` is off the stack again; share it unless it met the depth
    /// limit; and count it toward the computation that asked.
    pub(super) fn keep(
        &mut self,
        id: &str,
        read: Read,
        frame: Frame,
        had_cuts: bool,
        truncated: bool,
        stacks: &Stacks<'_>,
    ) -> Rc<Kept> {
        let kept = {
            let shared = self.shared.borrow();
            let mut on: Vec<Key> = frame
                .on
                .into_iter()
                .filter(|k| stacks.holds(&shared, *k))
                .collect();
            on.sort_unstable();
            let mut off: Vec<Key> = frame.off.into_iter().collect();
            off.sort_unstable();
            Rc::new(Kept {
                read,
                on: on.into(),
                off: off.into(),
                had_cuts,
                truncated,
            })
        };
        if !truncated {
            let mut shared = self.shared.borrow_mut();
            let table = self.table;
            let variants = shared.tables[table].entry(id.to_string()).or_default();
            if variants.len() < VARIANTS_CAP {
                variants.push(kept.clone());
            }
        }
        self.took(id, &kept);
        kept
    }
}
