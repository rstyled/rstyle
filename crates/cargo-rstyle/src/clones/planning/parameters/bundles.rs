//! Repeated unchanged arguments are bundle witnesses, not a safe rewrite proof.
use super::{Argument, Availability, Coverage, Execution, Plan};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct Bundle {
    target_parameters: Vec<u32>,
    witnesses: Vec<Witness>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Witness {
    caller_index: usize,
    caller_formals: Vec<u32>,
}

type Forwarding = BTreeMap<u32, u32>;

#[derive(Serialize)]
pub(super) struct Analysis {
    forwarding_bundles: Vec<Bundle>,
    bundle_support_coverage: Coverage,
    bundle_execution: Execution,
    bundle_reasons: Vec<&'static str>,
}

impl Default for Analysis {
    fn default() -> Self {
        Self {
            forwarding_bundles: Vec::new(),
            bundle_support_coverage: Coverage::Incomplete,
            bundle_execution: Execution::Completed,
            bundle_reasons: Vec::new(),
        }
    }
}

impl Analysis {
    pub(super) fn exhausted(&self) -> bool {
        self.bundle_execution == Execution::BudgetExhausted
    }
}

struct Search {
    remaining: usize,
    bundles: Vec<Bundle>,
}

impl Search {
    fn charge(&mut self, work: usize) -> Result<(), ()> {
        let granted = self.remaining >= work;
        self.remaining = self.remaining.saturating_sub(work);
        granted.then_some(()).ok_or(())
    }

    fn run(&mut self, calls: &[Forwarding]) -> Result<(), ()> {
        let candidates = self.candidates(calls)?;
        for positions in candidates {
            let witnesses = self.support(calls, &positions)?;
            if witnesses.len() >= 2 {
                self.bundles.push(Bundle {
                    target_parameters: positions,
                    witnesses,
                });
            }
        }
        Ok(())
    }

    fn candidates(&mut self, calls: &[Forwarding]) -> Result<BTreeSet<Vec<u32>>, ()> {
        let mut candidates = BTreeSet::new();
        for call in calls {
            self.charge(1 + call.len())?;
            if call.len() >= 2 {
                candidates.insert(call.keys().copied().collect::<Vec<_>>());
            }
        }

        let groups: Vec<_> = candidates.iter().cloned().collect();
        for (index, first) in groups.iter().enumerate() {
            for second in &groups[index + 1..] {
                self.charge(1 + first.len() + second.len())?;
                let shared = intersection(first, second);
                if shared.len() >= 2 {
                    candidates.insert(shared);
                }
            }
        }
        Ok(candidates)
    }

    fn support(&mut self, calls: &[Forwarding], positions: &[u32]) -> Result<Vec<Witness>, ()> {
        let mut witnesses = Vec::new();
        for (caller_index, call) in calls.iter().enumerate() {
            self.charge(1 + positions.len())?;
            if let Some(caller_formals) = positions
                .iter()
                .map(|position| call.get(position).copied())
                .collect::<Option<Vec<_>>>()
            {
                witnesses.push(Witness {
                    caller_index,
                    caller_formals,
                });
            }
        }
        Ok(witnesses)
    }
}

fn intersection(first: &[u32], second: &[u32]) -> Vec<u32> {
    let mut right = second.iter().peekable();
    first
        .iter()
        .filter(|left| {
            while right.peek().is_some_and(|value| *value < *left) {
                right.next();
            }
            right.peek().is_some_and(|value| *value == *left)
        })
        .copied()
        .collect()
}

/// Repeated uses of the same formal are excluded rather than choosing an alias.
fn forwarding(arguments: &[Argument]) -> Forwarding {
    let mut roots = BTreeMap::<u32, Vec<u32>>::new();
    for (position, argument) in arguments.iter().enumerate() {
        if let Availability::Known(Some(formal)) = argument.unchanged_formal {
            roots.entry(formal).or_default().push(position as u32);
        }
    }
    roots
        .into_iter()
        .filter_map(|(formal, positions)| match positions.as_slice() {
            [position] => Some((*position, formal)),
            _ => None,
        })
        .collect()
}

pub(super) fn evaluate(plan: &Plan, remaining: &mut usize) -> Analysis {
    let mut search = Search {
        remaining: *remaining,
        bundles: Vec::new(),
    };
    let result = inspect(&mut search, plan);
    *remaining = search.remaining;

    Analysis {
        forwarding_bundles: search.bundles,
        bundle_execution: if result.is_ok() { Execution::Completed } else { Execution::BudgetExhausted },
        bundle_support_coverage: if result.is_ok() && plan.reasons.is_empty() { Coverage::Complete } else { Coverage::Incomplete },
        bundle_reasons: result.err().map(|()| "Forwarding bundle search budget exhausted; retained bundles have completed witness scans").into_iter().collect(),
    }
}

fn inspect(search: &mut Search, plan: &Plan) -> Result<(), ()> {
    let mut calls = Vec::new();
    for caller in &plan.callers {
        search.charge(1 + caller.arguments.len())?;
        calls.push(forwarding(&caller.arguments));
    }
    search.run(&calls)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_search(calls: &[Forwarding], budget: usize) -> (Search, bool) {
        let mut search = Search {
            remaining: budget,
            bundles: Vec::new(),
        };
        let completed = search.run(calls).is_ok();
        (search, completed)
    }

    #[test]
    fn intersections_keep_exact_positions_and_permuted_roots() {
        let calls = [
            BTreeMap::from([(0, 0), (1, 1), (2, 2)]),
            BTreeMap::from([(0, 2), (1, 0), (3, 1)]),
        ];
        let (search, complete) = run_search(&calls, 100);
        assert!(complete);
        assert_eq!(search.bundles.len(), 1);
        assert_eq!(search.bundles[0].target_parameters, [0, 1]);
        assert_eq!(search.bundles[0].witnesses[0].caller_formals, [0, 1]);
        assert_eq!(search.bundles[0].witnesses[1].caller_formals, [2, 0]);
    }

    #[test]
    fn no_single_occurrence_or_cross_position_support() {
        let calls = [
            BTreeMap::from([(0, 0), (1, 1)]),
            BTreeMap::from([(1, 0), (2, 1)]),
        ];
        let (search, complete) = run_search(&calls, 100);
        assert!(complete);
        assert!(search.bundles.is_empty());
    }

    #[test]
    fn exhausted_support_never_publishes_a_partially_counted_candidate() {
        let calls = vec![BTreeMap::from([(0, 0), (1, 1)]); 3];
        let (search, complete) = run_search(&calls, 15);
        assert!(!complete);
        assert!(search.bundles.is_empty());
        let (search, complete) = run_search(&calls, 18);
        assert!(complete);
        assert_eq!(search.bundles[0].witnesses.len(), 3);
    }

    #[test]
    fn repeated_roots_are_excluded_without_losing_other_distinct_roots() {
        let arguments = [Some(0), Some(0), Some(1), Some(2), None].map(|formal| Argument {
            span: rstyle_pdg::ByteSpan { start: 0, end: 1 },
            formal: Availability::Known(formal),
            unchanged_formal: Availability::Known(formal),
        });
        assert_eq!(forwarding(&arguments), BTreeMap::from([(2, 1), (3, 2)]));
    }
}
