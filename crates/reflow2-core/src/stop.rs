//! A long read stops when its caller says so: at a named check point, never
//! part-way through a write and never by the core reading a clock.
//!
//! `loop_status` recomputes whole-design analyses on every call. On reflow2's
//! own design (8,696 nodes, 2026-11-07) it took 38.1 s on flo2.io's hardware,
//! past the 30 s its gateway waits, and each earlier crossing had been fixed
//! one hot spot at a time
//! (fact:root-cause-loop-status-on-reflow2s-design-takes-38-s-on-flo2-io-past-its-30-s-door-gaps-and-defects-each-about-half-2026-11-07).
//! The class fix (req:loop-status-answers-within-a-time-budget-and-names-the-sections-it-did-not-compute)
//! is a read that can be TOLD TO STOP: when its time budget runs out, or when
//! the caller has gone, at the next check point.
//!
//! ⭐ THE CORE NEVER READS THE CLOCK. It asks, at each [`checkpoint`], a
//! question the CALLER answers ([`within`]): the MCP layer answers it from a
//! deadline and the request's cancellation, and a test answers it however it
//! likes. So the core stays deterministic for any deterministic answer, which
//! is the invariant crates/reflow2-core/AGENTS.md states.
//!
//! ⭐ NOTHING IS ASKED UNLESS A CALLER ASKED FOR IT. With no [`within`] open
//! on this thread, [`checkpoint`] returns `Ok` at once, so every existing
//! caller of `detect_gaps`, `open_defects` or `loop_status_for` computes
//! exactly what it always did.
//!
//! ⭐ A STOP IS AN ERROR, [`DynoError::Stopped`], so it unwinds through every
//! `?` between the check point and whoever opened the scope, and nothing
//! half-computed is kept: the per-generation memos of gaps and defects are
//! written only when a sweep finishes. Whoever opened the scope catches it
//! and says what was not computed — `loop_status_for` names each section.
//!
//! Thread-local, like the reasoning memo's scope (`inference::memo`): a read
//! runs on one thread from start to end, holding the design's read guard, and
//! the scope is opened and closed around that synchronous call.

use std::cell::RefCell;
use std::rc::Rc;

use crate::foundation::core::DynoError;

/// Why a read was told to stop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The time the caller gave this call ran out.
    Budget,
    /// The caller gave up on the call: its client cancelled it or went away.
    CallerGone,
}

impl StopReason {
    /// The word a reply carries (`budget`, `caller_gone`).
    pub fn as_str(self) -> &'static str {
        match self {
            StopReason::Budget => "budget",
            StopReason::CallerGone => "caller_gone",
        }
    }
}

/// The question asked at every check point: should the read stop here, and
/// why? `at` names the check point (`gaps:unsatisfied_requirements`), so a
/// caller can also log where the time went.
type Check = Rc<dyn Fn(&'static str) -> Option<StopReason>>;

thread_local! {
    static CHECK: RefCell<Option<Check>> = const { RefCell::new(None) };
}

/// Run `work` with `check` asked at every [`checkpoint`] it reaches on this
/// thread. The scope closes when `work` returns, however it returns, and a
/// scope already open is restored after it, so scopes nest.
pub fn within<T>(
    check: impl Fn(&'static str) -> Option<StopReason> + 'static,
    work: impl FnOnce() -> T,
) -> T {
    struct Restore(Option<Check>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let prev = self.0.take();
            CHECK.with(|c| *c.borrow_mut() = prev);
        }
    }
    let prev = CHECK.with(|c| c.borrow_mut().replace(Rc::new(check)));
    let _restore = Restore(prev);
    work()
}

/// A place where a long read may stop. `Ok(())` when no scope is open or the
/// caller says go on; [`DynoError::Stopped`] when it says stop.
pub fn checkpoint(at: &'static str) -> Result<(), DynoError> {
    // Cloned out of the cell before it is called, so a check that itself
    // reaches a check point (it should not) cannot find the cell borrowed.
    let check = CHECK.with(|c| c.borrow().clone());
    match check.and_then(|check| check(at)) {
        None => Ok(()),
        Some(reason) => Err(DynoError::Stopped {
            reason: reason.as_str().to_string(),
            at: at.to_string(),
        }),
    }
}

/// A point the caller HEARS ABOUT and cannot stop at: a part of a report that
/// is always computed (`loop_status`'s floor), named so a caller timing the
/// read can still say where its time went.
pub fn mark(at: &'static str) {
    let check = CHECK.with(|c| c.borrow().clone());
    if let Some(check) = check {
        let _ = check(at);
    }
}

/// The reason a [`DynoError::Stopped`] carries, or `None` for any other error.
pub fn stopped(err: &DynoError) -> Option<StopReason> {
    match err {
        DynoError::Stopped { reason, .. } => Some(if reason == "caller_gone" {
            StopReason::CallerGone
        } else {
            StopReason::Budget
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn a_checkpoint_with_no_scope_never_stops() {
        assert!(checkpoint("anywhere").is_ok());
    }

    #[test]
    fn a_scope_stops_at_the_point_its_check_names_and_closes_after() {
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = seen.clone();
        let r = within(
            move |at| {
                log.borrow_mut().push(at);
                (at == "b").then_some(StopReason::Budget)
            },
            || -> Result<(), DynoError> {
                checkpoint("a")?;
                checkpoint("b")?;
                checkpoint("c")?;
                Ok(())
            },
        );
        let err = r.unwrap_err();
        assert_eq!(stopped(&err), Some(StopReason::Budget));
        assert!(err.to_string().contains("at b"), "{err}");
        assert_eq!(*seen.borrow(), vec!["a", "b"]);
        assert!(
            checkpoint("after").is_ok(),
            "the scope closed with its work"
        );
    }

    #[test]
    fn scopes_nest_and_the_outer_one_comes_back() {
        let outer_asked = Rc::new(Cell::new(0));
        let n = outer_asked.clone();
        within(
            move |_| {
                n.set(n.get() + 1);
                None
            },
            || {
                within(
                    |_| Some(StopReason::CallerGone),
                    || {
                        assert_eq!(
                            stopped(&checkpoint("inner").unwrap_err()),
                            Some(StopReason::CallerGone)
                        );
                    },
                );
                assert!(checkpoint("outer").is_ok());
            },
        );
        assert_eq!(outer_asked.get(), 1);
    }
}
