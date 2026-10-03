//! ONE WRITE UNIT PER TOOL CALL — the served half of
//! `dec:idea-a-refused-typed-write-stores-nothing` (accepted 2026-10-02).
//!
//! # The failure
//!
//! A typed constructor was a sequence of individually durable writes: the
//! node, then its optional fields, then its links and edges, each checked as
//! it was written. A refusal at a LATER check left the earlier writes stored —
//! measured on 0.77.0 in 13 constructors
//! (`fact:root-cause-a-typed-constructor-writes-the-node-before-its-later-checks-and-a-refusal-leaves-it-2026-10-02`).
//! On a revise the refusal said "nothing was written" after the body had been
//! overwritten; the corrected retry counted as a revise, so the duplicate guard
//! never ran; and the leftover became a near-match blocking the next create.
//! Per-handler rollbacks existed in two places and covered a create, never a
//! revise. The class was "atomicity is a property of the bulk door only".
//!
//! # The cure: the call is the unit, settled at the choke point
//!
//! `call_tool` opens a [`CallUnit`] for every tool the catalogue marks as a
//! write (`read_only_hint` false — the predicate the receipt and `--call`
//! use), and settles it once when the handler answers: COMMITTED when it
//! succeeded, DISCARDED when it refused or errored. The unit is the store's
//! own atomic batch (`DesignGraph::begin_unit`), so reads inside the call see
//! its writes and a bulk form or import inside it nests.
//!
//! The unit takes the graph's write lock at the call's FIRST write and holds
//! it to the end of the call, so no other session can read a staged write or
//! write between two of this call's. A handler's later `write_lock` and every
//! `read` inside the call are served from that one hold — which is why both
//! doors live here.
//!
//! # Why a handler cannot go round it
//!
//! [`SharedGraph`] is the only way a handler reaches the design, and it has
//! two doors: `read`, which yields `&DesignGraph`, and `hold_for_write`, which
//! is visible to `service.rs` alone, so a handler reaches `&mut DesignGraph`
//! only through `ReflowService::write_lock`. A handler that took the raw lock
//! (`review_relations` did, and so wrote outside the crediting and signing
//! that `write_lock` carries) no longer compiles. And a tool marked read-only
//! that asks for the write lock is refused inside a served call, so the
//! annotation that decides whether a call gets a unit cannot lie.
//!
//! What the unit does NOT cover: files beside the store (the identity label,
//! the usage ledger, the handshake record, an export written to a path). It
//! is the store's unit, and those are not the store.

use std::sync::{Arc, Weak};

use reflow2_core::DesignGraph;
use tokio::sync::{
    OwnedRwLockMappedWriteGuard, OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock,
    RwLockReadGuard, RwLockWriteGuard,
};

tokio::task_local! {
    /// The served call now running, set by `call_tool` around the handler.
    /// Absent outside a served call — a handler a test calls directly, the
    /// CLI's one-shot modes, startup — where every write lands as it is made,
    /// exactly as before units existed.
    static CALL: CallScope;
}

/// What `call_tool` says about the call it is serving.
#[derive(Clone)]
pub(crate) struct CallScope {
    /// The tool's name, for a refusal that has to say which.
    pub(crate) tool: Arc<str>,
    /// The call's write unit — `None` for a tool the catalogue marks read-only.
    pub(crate) unit: Option<Arc<CallUnit>>,
}

impl CallScope {
    /// Run `call` as the served call `self` describes.
    pub(crate) async fn run<T>(self, call: impl std::future::Future<Output = T>) -> T {
        CALL.scope(self, call).await
    }

    /// The served call now running, if any.
    pub(crate) fn current() -> Option<CallScope> {
        CALL.try_with(Clone::clone).ok()
    }
}

/// One call's write unit: the graph held for writing from the call's first
/// write to its end, with the unit open on it.
pub(crate) struct CallUnit {
    /// WHICH graph this unit is for. A call is served by one design; a handler
    /// that opened ANOTHER design must not have that design's writes staged in
    /// this unit, so the doors below match on it.
    graph: usize,
    slot: Arc<RwLock<Option<UnitHold>>>,
}

/// The graph's write guard with the unit open on it. Dropping it unsettled —
/// a handler that panicked, a call the client cancelled — DISCARDS the unit:
/// a call that never finished stores nothing, and the next call must not find
/// a batch left open under it.
pub(crate) struct UnitHold {
    guard: OwnedRwLockWriteGuard<DesignGraph>,
}

impl Drop for UnitHold {
    fn drop(&mut self) {
        if self.guard.in_unit() {
            self.guard.discard_unit();
        }
    }
}

/// How a unit was settled.
#[derive(Debug)]
pub(crate) enum Settled {
    /// The call never wrote (or asked to write) the store.
    Untouched,
    /// Committed: this many store operations landed, in one atomic write.
    Committed(usize),
    /// The call refused or errored, and everything it staged was dropped.
    Discarded,
    /// The call succeeded but its writes could not be committed. Nothing
    /// landed.
    CommitFailed(String),
}

impl CallUnit {
    /// A unit for a call served by `graph`.
    pub(crate) fn for_graph(graph: &SharedGraph) -> Arc<Self> {
        Arc::new(Self {
            graph: graph.identity(),
            slot: Arc::new(RwLock::new(None)),
        })
    }

    /// Settle the unit once the handler has answered: commit when `succeeded`,
    /// discard otherwise. Releases the graph either way.
    pub(crate) async fn settle(&self, succeeded: bool) -> Settled {
        let mut slot = self.slot.write().await;
        let Some(mut hold) = slot.take() else {
            return Settled::Untouched;
        };
        if succeeded {
            match hold.guard.commit_unit() {
                Ok(wrote) => Settled::Committed(wrote),
                Err(e) => Settled::CommitFailed(e.to_string()),
            }
        } else {
            hold.guard.discard_unit();
            Settled::Discarded
        }
    }
}

/// The unit of the served call now running, when it is a unit for `graph`.
fn unit_for(graph: &SharedGraph) -> Option<Arc<CallUnit>> {
    CALL.try_with(|c| c.unit.clone())
        .ok()
        .flatten()
        .filter(|u| u.graph == graph.identity())
}

/// The design graph a service holds, behind its lock — reachable only
/// through the doors that know about write units (see the module note).
#[derive(Clone)]
pub(crate) struct SharedGraph(Arc<RwLock<DesignGraph>>);

impl SharedGraph {
    pub(crate) fn new(graph: DesignGraph) -> Self {
        Self(Arc::new(RwLock::new(graph)))
    }

    fn identity(&self) -> usize {
        Arc::as_ptr(&self.0) as usize
    }

    /// The graph for READING. Inside a served call that has written, this is
    /// the call's own hold, so the call reads its staged writes and never
    /// waits on the lock it holds; otherwise a shared read lock, as always.
    pub(crate) async fn read(&self) -> GraphRead<'_> {
        if let Some(unit) = unit_for(self) {
            let slot = Arc::clone(&unit.slot).read_owned().await;
            if let Ok(held) = OwnedRwLockReadGuard::try_map(slot, |s| s.as_ref().map(|h| &*h.guard))
            {
                return GraphRead::Unit(held);
            }
            // The call has not written yet: read like anybody else.
        }
        GraphRead::Shared(self.0.read().await)
    }

    /// The graph for WRITING — `ReflowService::write_lock` alone calls this
    /// (it is visible to `service.rs` and nowhere else). Inside a served write
    /// call, the first hold takes the graph's write lock and opens the unit;
    /// every later hold in the same call reuses it. Outside a served call, the
    /// plain write lock.
    pub(super) async fn hold_for_write(&self) -> GraphHold<'_> {
        if let Some(unit) = unit_for(self) {
            let mut slot = Arc::clone(&unit.slot).write_owned().await;
            if slot.is_none() {
                let mut guard = Arc::clone(&self.0).write_owned().await;
                guard.begin_unit();
                *slot = Some(UnitHold { guard });
            }
            return GraphHold::Unit(OwnedRwLockWriteGuard::map(slot, |s| {
                &mut *s
                    .as_mut()
                    .expect("the unit's hold was taken just above")
                    .guard
            }));
        }
        GraphHold::Shared(self.0.write().await)
    }

    /// A weak handle to the lock, for the code that manages the STORE'S
    /// lifetime — the write-through export's task, the drain on shutdown and
    /// the registry's idle close — none of which runs inside a call, and all
    /// of which only read or wait for the store to be released.
    ///
    /// 🛑 NOT A WRITE PATH. Writing through it would skip the unit, the
    /// crediting and the signing `write_lock` carries — the bypass this module
    /// exists to close.
    pub(crate) fn downgrade(&self) -> Weak<RwLock<DesignGraph>> {
        Arc::downgrade(&self.0)
    }
}

/// A read of the graph — see [`SharedGraph::read`].
pub(crate) enum GraphRead<'a> {
    Shared(RwLockReadGuard<'a, DesignGraph>),
    Unit(OwnedRwLockReadGuard<Option<UnitHold>, DesignGraph>),
}

impl std::ops::Deref for GraphRead<'_> {
    type Target = DesignGraph;
    fn deref(&self) -> &DesignGraph {
        match self {
            Self::Shared(g) => g,
            Self::Unit(g) => g,
        }
    }
}

/// A write hold on the graph — see [`SharedGraph::hold_for_write`].
pub(crate) enum GraphHold<'a> {
    Shared(RwLockWriteGuard<'a, DesignGraph>),
    Unit(OwnedRwLockMappedWriteGuard<Option<UnitHold>, DesignGraph>),
}

impl std::ops::Deref for GraphHold<'_> {
    type Target = DesignGraph;
    fn deref(&self) -> &DesignGraph {
        match self {
            Self::Shared(g) => g,
            Self::Unit(g) => g,
        }
    }
}

impl std::ops::DerefMut for GraphHold<'_> {
    fn deref_mut(&mut self) -> &mut DesignGraph {
        match self {
            Self::Shared(g) => g,
            Self::Unit(g) => g,
        }
    }
}
