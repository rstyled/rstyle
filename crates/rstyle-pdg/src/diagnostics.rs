//! Opt-in performance observations, separate from deterministic evidence.
//! `RSTYLE_PERF=1` emits JSON lines to stderr; failures to log are nonfatal.
use serde::Serialize;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;
use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};
use std::time::Instant;

#[derive(Default)]
struct Counts {
    depth: usize,
    visits: BTreeMap<String, usize>,
}

thread_local! {
    static COUNTS: RefCell<Counts> = RefCell::default();
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
struct ObservationId(u64);

impl ObservationId {
    fn next() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Serialize)]
struct Event<'a> {
    schema: &'static str,
    observation_id: ObservationId,
    stage: &'a str,
    identity: &'a str,
    pid: u32,
    event: &'a str,
    depth: usize,
    visit: usize,
    evidence_bytes: usize,
    elapsed_seconds: f64,
    verification_policy: &'static str,
}

impl Event<'_> {
    fn emit(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            let _ = writeln!(std::io::stderr().lock(), "rstyle-perf {json}");
        }
    }
}

struct Observation<'a> {
    event: Event<'a>,
    started: Instant,
}

impl<'a> Observation<'a> {
    fn start(stage: &'a str, identity: &'a str, evidence: Option<&[u8]>) -> Self {
        let (depth, visit) = COUNTS.with_borrow_mut(|counts| {
            counts.depth += 1;
            let visit = counts
                .visits
                .entry(format!("{stage}:{identity}"))
                .or_default();
            *visit += 1;
            (counts.depth, *visit)
        });
        let event = Event {
            schema: "rstyle.performance-event/2",
            observation_id: ObservationId::next(),
            stage,
            identity,
            pid: std::process::id(),
            event: "start",
            depth,
            visit,
            evidence_bytes: evidence.map_or(0, <[u8]>::len),
            elapsed_seconds: 0.0,
            verification_policy: if crate::verification_policy::is_generated() {
                "trusted-generated"
            } else {
                "strict"
            },
        };
        event.emit();

        Self {
            event,
            started: Instant::now(),
        }
    }

    fn finish(&mut self, success: bool) {
        self.event.event = if success { "finish" } else { "failure" };
    }
}

impl Observation<'_> {
    fn final_event(&self) -> Event<'_> {
        let event = if self.event.event != "start" {
            self.event.event
        } else if std::thread::panicking() {
            "interrupted"
        } else {
            "failure"
        };

        Event {
            event,
            elapsed_seconds: self.started.elapsed().as_secs_f64(),
            ..self.event.clone()
        }
    }
}

impl Drop for Observation<'_> {
    fn drop(&mut self) {
        self.final_event().emit();
        COUNTS.with_borrow_mut(|counts| counts.depth -= 1);
    }
}

/// Metadata for one fallible operation, separate from its immutable inputs.
/// Evidence is borrowed only to record its length, never scanned or fingerprinted.
/// Observation IDs are process-local event handles, never verification authority.
pub struct Stage<'a> {
    stage: &'a str,
    identity: &'a str,
    evidence: Option<&'a [u8]>,
}

impl<'a> Stage<'a> {
    pub fn new(stage: &'a str, identity: &'a str, evidence: Option<&'a [u8]>) -> Self {
        Self {
            stage,
            identity,
            evidence,
        }
    }

    /// Enter a stage whose early returns must also emit a failure event.
    pub fn enter(self) -> Guard<'a> {
        Guard(enabled().then(|| Observation::start(self.stage, self.identity, self.evidence)))
    }

    /// Invoke once, retaining the original result even if diagnostic IO fails.
    pub fn observe<T, E>(self, operation: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
        if enabled() {
            Observation::start(self.stage, self.identity, self.evidence).run(operation)
        } else {
            operation()
        }
    }
}

/// A stage guard; early return emits failure and unwinding emits interruption.
pub struct Guard<'a>(Option<Observation<'a>>);

impl Guard<'_> {
    pub fn finish<T, E>(mut self, result: Result<T, E>) -> Result<T, E> {
        if let Some(observation) = &mut self.0 {
            observation.finish(result.is_ok());
        }
        result
    }
}

impl Observation<'_> {
    fn run<T, E>(mut self, operation: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
        let result = operation();
        self.finish(result.is_ok());

        result
    }
}

/// Read an explicitly named opt-in diagnostic switch.
pub fn switch(name: &str) -> bool {
    std::env::var_os(name).is_some_and(|value| value == "1")
}

fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| switch("RSTYLE_PERF"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observation_ids_and_labeled_visits_survive_nested_failure() {
        let mut outer = Observation::start("test.verify", "caller", Some(b"proof"));
        let mut repeated = Observation::start("test.verify", "caller", Some(b"other"));
        let distinct = Observation::start("test.verify", "caller", Some(b"tampered"));
        assert_eq!(repeated.event.visit, outer.event.visit + 1);
        assert_eq!(repeated.event.depth, outer.event.depth + 1);
        assert_ne!(distinct.event.observation_id, outer.event.observation_id);
        assert_ne!(repeated.event.observation_id, outer.event.observation_id);
        assert_eq!(distinct.event.visit, repeated.event.visit + 1);
        let event = serde_json::to_value(&distinct.event).unwrap();
        assert_eq!(event["schema"], "rstyle.performance-event/2");
        assert!(event.get("evidence_digest").is_none());
        assert_eq!(distinct.event.evidence_bytes, 8);
        repeated.finish(false);
        assert_eq!(repeated.event.event, "failure");
        assert_eq!(
            repeated.final_event().observation_id,
            repeated.event.observation_id
        );
        drop(distinct);
        drop(repeated);
        outer.finish(true);
        assert_eq!(outer.event.event, "finish");
        drop(outer);
        COUNTS.with_borrow(|counts| assert_eq!(counts.depth, 0));
    }
}
