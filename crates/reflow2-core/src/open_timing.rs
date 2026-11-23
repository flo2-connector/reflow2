//! How long opening a design on disk took, step by step
//! (`cap:an-open-reports-how-long-each-of-its-steps-took`).
//!
//! An open does more than open RocksDB: it settles the design's identity,
//! checks and stamps provenance, runs the repairs that run on every open, and
//! checks the search index against the store. Until 2026-11-23 none of that was
//! timed, so the only way to split an open was to stop it under a debugger
//! (`fact:profile-2026-11-23-where-an-open-and-loop-status-spend-their-time-on-reflow2s-own-design`),
//! and whether a hosted reopen paid the same steps could not be told at all.
//!
//! Now every on-disk open keeps an [`OpenTiming`] on the graph it returns
//! ([`crate::graph::DesignGraph::opened_in`]) and writes it as ONE `INFO` line,
//! `opened a design`, with the design's id, the total and each step in
//! milliseconds. A `--call` prints that line on stderr; a hosted engine writes
//! it to its log. Timing changes nothing the open does, or the order it does it
//! in.

use std::time::{Duration, Instant};

/// One step of an open and how long it took.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenStep {
    /// The step's name, stable across releases so a log reader can rely on
    /// it: `schema`, `identity`, `store`, `identity_probe`, `provenance`,
    /// `identity_write`, `stamps`, `type_index`, `authored_by_roles`,
    /// `edge_rewrites`, `stored_twins`, `search_index`.
    pub name: &'static str,
    /// Wall time from the end of the previous step to the end of this one.
    pub elapsed: Duration,
    /// For a repair step, how many records it changed. `None` for a step
    /// that changes nothing countable.
    pub changed: Option<usize>,
}

/// Every step of one open, in the order the open ran them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenTiming {
    /// The steps in order. Their elapsed times add up to [`Self::total`],
    /// because each step starts where the one before it ended.
    pub steps: Vec<OpenStep>,
    /// Wall time from the start of the open to the end of its last step.
    pub total: Duration,
}

impl OpenTiming {
    /// How long the named step took, or `None` if this open had no such step.
    pub fn step(&self, name: &str) -> Option<Duration> {
        self.steps
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.elapsed)
    }

    /// The steps as one line: `schema=3 identity=0 store=118 ...`, in
    /// milliseconds. A repair that changed something adds what it changed in
    /// brackets, as in `authored_by_roles=2310(changed 4)`.
    pub fn summary(&self) -> String {
        self.steps
            .iter()
            .map(|s| match s.changed {
                Some(n) if n > 0 => format!("{}={}(changed {n})", s.name, s.elapsed.as_millis()),
                _ => format!("{}={}", s.name, s.elapsed.as_millis()),
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// The stopwatch an open carries: each `lap` ends one step.
pub(crate) struct OpenClock {
    started: Instant,
    last: Instant,
    steps: Vec<OpenStep>,
}

impl OpenClock {
    pub(crate) fn start() -> Self {
        let now = Instant::now();
        OpenClock {
            started: now,
            last: now,
            steps: Vec::new(),
        }
    }

    /// End the step called `name`: it took the time since the last lap.
    pub(crate) fn lap(&mut self, name: &'static str) {
        self.lap_counting(name, None);
    }

    /// End a repair step, saying how many records it changed.
    pub(crate) fn lap_counting(&mut self, name: &'static str, changed: Option<usize>) {
        let now = Instant::now();
        self.steps.push(OpenStep {
            name,
            elapsed: now - self.last,
            changed,
        });
        self.last = now;
    }

    pub(crate) fn finish(self) -> OpenTiming {
        OpenTiming {
            total: self.last - self.started,
            steps: self.steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steps_add_up_to_the_total_and_keep_their_order() {
        let mut clock = OpenClock::start();
        clock.lap("schema");
        std::thread::sleep(Duration::from_millis(5));
        clock.lap_counting("authored_by_roles", Some(3));
        clock.lap_counting("edge_rewrites", Some(0));
        let t = clock.finish();
        let names: Vec<_> = t.steps.iter().map(|s| s.name).collect();
        assert_eq!(names, ["schema", "authored_by_roles", "edge_rewrites"]);
        let sum: Duration = t.steps.iter().map(|s| s.elapsed).sum();
        assert_eq!(sum, t.total);
        assert!(t.step("authored_by_roles").unwrap() >= Duration::from_millis(5));
        assert!(t.step("nothing").is_none());
        let line = t.summary();
        assert!(line.starts_with("schema="), "{line}");
        assert!(line.contains("(changed 3)"), "{line}");
        assert!(
            !line.contains("edge_rewrites=0(changed"),
            "a repair that changed nothing says nothing: {line}"
        );
    }
}
