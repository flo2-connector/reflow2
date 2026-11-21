//! How long `loop_status` may take, and the question its check points ask.
//!
//! **Why it exists.** `loop_status` on reflow2's own design (8,696 nodes) took
//! 38.1 s on flo2.io's hardware on 2026-11-07, and the gateway in front of it
//! gives up at 30 s, so the call every session is told to make answered
//! nothing at all, twenty times in twelve days
//! (`fact:root-cause-loop-status-on-reflow2s-design-takes-38-s-on-flo2-io-past-its-30-s-door-gaps-and-defects-each-about-half-2026-11-07`).
//! The class fix, accepted that day
//! (`req:loop-status-answers-within-a-time-budget-and-names-the-sections-it-did-not-compute`),
//! is a budget: the call computes its floor and then its whole-design analyses
//! cheapest first, and when the time is gone it answers with what it has and
//! names the rest (`reflow2_core::LoopStatus::not_computed`).
//!
//! **The default is 20 s, two thirds of the door** (`DEFAULT_MS`). The clock
//! starts when the CALL arrives (`crate::service::call_started`), before the
//! handler waits for the design's lock, so a wait behind a writer is spent from
//! the budget rather than added to it — and a hub's view, which runs one call
//! on the hub and then on each member in turn, spends ONE budget across all of
//! them rather than a fresh one per design. The
//! other 10 s of flo2.io's 30 are for what this clock cannot see: opening a
//! design the server does not hold open yet (3.8 s for opening plus a trivial
//! call on that hardware, measured the same day), the stretch from the last
//! check point to the end of the analysis it stops (the analyses check
//! between their detectors), and the reply's serialisation and transport.
//!
//! **Three ways to set it, nearest wins:** the call's own `time_budget_ms`;
//! the server's `REFLOW2_LOOP_STATUS_BUDGET_MS` (an operator serving behind a
//! different door); this default. `0` is a budget of nothing: the floor alone.
//!
//! **And a caller who has gone stops it too**, whatever the budget: the same
//! check points ask whether rmcp has cancelled the request
//! ([`crate::service::CallerGone`]).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use reflow2_core::stop::StopReason;
use serde_json::{Value, json};

/// The budget a `loop_status` call gets when neither it nor the server names one.
pub const DEFAULT_MS: u64 = 20_000;

/// The server-wide budget, in milliseconds, for an operator whose door is not
/// flo2.io's 30 s.
pub const ENV: &str = "REFLOW2_LOOP_STATUS_BUDGET_MS";

/// Where a call's budget came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetBy {
    /// The call's own `time_budget_ms`.
    Argument,
    /// The server's [`ENV`].
    Server,
    /// [`DEFAULT_MS`].
    Default,
}

impl SetBy {
    fn as_str(self) -> &'static str {
        match self {
            SetBy::Argument => "argument",
            SetBy::Server => "server",
            SetBy::Default => "default",
        }
    }
}

/// The server's budget: [`ENV`] when it holds a whole number of milliseconds,
/// else [`DEFAULT_MS`]. Read once per process. A value that is not a number is
/// said on stderr once and ignored, never read as zero.
fn server_budget() -> (u64, SetBy) {
    static READ: std::sync::OnceLock<(u64, SetBy)> = std::sync::OnceLock::new();
    *READ.get_or_init(|| match std::env::var(ENV) {
        Ok(v) => match v.trim().parse::<u64>() {
            Ok(ms) => (ms, SetBy::Server),
            Err(_) => {
                eprintln!(
                    "reflow2: {ENV}={v:?} is not a whole number of milliseconds; loop_status \
                     keeps its default budget of {DEFAULT_MS} ms"
                );
                (DEFAULT_MS, SetBy::Default)
            }
        },
        Err(_) => (DEFAULT_MS, SetBy::Default),
    })
}

/// One call's budget, started when the handler starts.
pub struct CallBudget {
    started: Instant,
    budget: Duration,
    set_by: SetBy,
    gone: Option<crate::service::CallerGone>,
    /// Set the first time a check point is told to stop, so the log says it once.
    told: Arc<AtomicBool>,
}

impl CallBudget {
    /// Start the clock for one `loop_status` call. `asked` is its
    /// `time_budget_ms`.
    pub fn start(asked: Option<u64>) -> Self {
        let (ms, set_by) = match asked {
            Some(ms) => (ms, SetBy::Argument),
            None => server_budget(),
        };
        CallBudget {
            // The CALL's arrival when one is being served (`call_started`), so
            // a hub's view spends one budget across its designs.
            started: crate::service::call_started().unwrap_or_else(Instant::now),
            budget: Duration::from_millis(ms),
            set_by,
            gone: crate::service::caller_gone(),
            told: Arc::new(AtomicBool::new(false)),
        }
    }

    /// No budget at all, only the caller: for a read that has no budget of
    /// its own and still stops when nobody is waiting (`detect_gaps`,
    /// `detect_defects`).
    pub fn caller_only() -> Self {
        CallBudget {
            started: Instant::now(),
            budget: Duration::MAX,
            set_by: SetBy::Default,
            gone: crate::service::caller_gone(),
            told: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Run `work` with this budget asked at every check point it reaches
    /// (`reflow2_core::stop`).
    pub fn run<T>(&self, work: impl FnOnce() -> T) -> T {
        let started = self.started;
        let budget = self.budget;
        let gone = self.gone.clone();
        let told = self.told.clone();
        reflow2_core::stop::within(
            move |at| {
                let elapsed = started.elapsed();
                // Where the time goes, at debug: RUST_LOG=reflow2_mcp::time_budget=debug.
                tracing::debug!(at, elapsed_ms = elapsed.as_millis() as u64, "check point");
                let reason = if gone.as_ref().is_some_and(|g| g()) {
                    Some(StopReason::CallerGone)
                } else if elapsed >= budget {
                    Some(StopReason::Budget)
                } else {
                    None
                };
                if let Some(r) = reason
                    && !told.swap(true, Ordering::Relaxed)
                {
                    tracing::info!(
                        at,
                        elapsed_ms = elapsed.as_millis() as u64,
                        budget_ms = budget.as_millis() as u64,
                        why = r.as_str(),
                        "a long read stopped at a check point"
                    );
                }
                reason
            },
            work,
        )
    }

    /// The reply's `time_budget` block: what the call was given, what it
    /// spent, and whether the answer is whole.
    pub fn block(&self, not_computed: &[reflow2_core::NotComputed]) -> Value {
        let mut b = json!({
            "budget_ms": self.budget.as_millis() as u64,
            "set_by": self.set_by.as_str(),
            "elapsed_ms": self.started.elapsed().as_millis() as u64,
            "complete": not_computed.is_empty(),
        });
        if !not_computed.is_empty() {
            b["note"] = json!(format!(
                "This answer is PARTIAL: {} of the {} whole-design analyses were not computed \
                 within the budget (or the caller gave up), named in `not_computed` with the call \
                 that computes each alone. Their counts read null, never zero, and the answer is \
                 not clean. The floor and the analyses before them were computed in full.",
                not_computed.len(),
                reflow2_core::LOOP_SECTIONS.len()
            ));
        }
        b
    }
}
