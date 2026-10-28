//! Ordered capability witnesses are advisory; pairing establishes no behavior equivalence.
use super::Execution;
use rstyle_pdg::consumer_facts::dispatch::{
    MethodOrigin,
    capabilities::{OwnerKey, Wrapper},
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub(super) struct Report {
    pub execution: Execution,
    pub complete: bool,
    pub expected_pairs: Option<usize>,
    pub assessed_pairs: usize,
    pub proposals: Vec<Proposal>,
    pub suppressed: Vec<PairDecision>,
    pub unavailable: Vec<PairDecision>,
    pub reason: Option<&'static str>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct Proposal {
    pub left: usize,
    pub right: usize,
    pub roles: [Role; 2],
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(super) struct Role {
    pub left_method: usize,
    pub right_method: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct PairDecision {
    pub left: usize,
    pub right: usize,
    pub reason: &'static str,
}

enum Decision {
    Propose,
    Suppress(&'static str),
    Unavailable(&'static str),
}

/// Input positions refer to original function order within one immutable source index.
pub(super) fn evaluate(wrappers: &[&Wrapper], max_pairs: usize) -> Result<Report, String> {
    validate(wrappers)?;
    let owners = owner_groups(wrappers);
    let expected = expected_pairs(&owners);
    if expected.is_none_or(|count| count > max_pairs) {
        return Ok(Report::exhausted(expected));
    }

    let count = expected.expect("bounded pair population");
    let pairs = candidate_pairs(&owners, count)
        .into_iter()
        .map(|(left, right)| EvaluatedPair {
            left,
            right,
            decision: classify(wrappers[left], wrappers[right]),
        })
        .collect();

    Ok(Report::completed(count, pairs))
}

fn validate(wrappers: &[&Wrapper]) -> Result<(), String> {
    for wrapper in wrappers {
        if wrapper.methods.len() != 2 {
            return Err(
                "Capability wrapper must retain exactly two ordered method witnesses".into(),
            );
        }
        if wrapper.methods[0].callee_header == wrapper.methods[1].callee_header {
            return Err("Capability wrapper repeats its original method definition".into());
        }
    }

    Ok(())
}

fn owner_groups<'a>(wrappers: &[&'a Wrapper]) -> BTreeMap<&'a OwnerKey, Vec<usize>> {
    wrappers.iter().enumerate().fold(
        BTreeMap::<&OwnerKey, Vec<usize>>::new(),
        |mut owners, (index, wrapper)| {
            owners.entry(&wrapper.owner).or_default().push(index);
            owners
        },
    )
}

struct Population {
    members: usize,
    pairs: usize,
}

impl Population {
    fn include(self, members: usize) -> Option<Self> {
        let additional = self.members.checked_mul(members)?;
        Some(Self {
            members: self.members.checked_add(members)?,
            pairs: self.pairs.checked_add(additional)?,
        })
    }
}

fn expected_pairs(owners: &BTreeMap<&OwnerKey, Vec<usize>>) -> Option<usize> {
    owners
        .values()
        .try_fold(
            Population {
                members: 0,
                pairs: 0,
            },
            |population, group| population.include(group.len()),
        )
        .map(|population| population.pairs)
}

fn candidate_pairs(owners: &BTreeMap<&OwnerKey, Vec<usize>>, count: usize) -> Vec<(usize, usize)> {
    let groups: Vec<_> = owners.values().collect();
    let candidates = groups.iter().enumerate().flat_map(|(ordinal, first)| {
        groups[ordinal + 1..]
            .iter()
            .flat_map(move |second| ordered_pairs(first, second))
    });

    let mut pairs = Vec::with_capacity(count);
    pairs.extend(candidates);
    pairs.sort_unstable();

    pairs
}

fn ordered_pairs<'a>(
    first: &'a [usize],
    second: &'a [usize],
) -> impl Iterator<Item = (usize, usize)> + 'a {
    first.iter().flat_map(move |left| {
        second
            .iter()
            .map(move |right| ((*left).min(*right), (*left).max(*right)))
    })
}

fn classify(left: &Wrapper, right: &Wrapper) -> Decision {
    let inherent = |wrapper: &Wrapper| {
        wrapper
            .methods
            .iter()
            .all(|method| matches!(method.origin, MethodOrigin::Inherent))
    };
    if inherent(left) && inherent(right) {
        return Decision::Propose;
    }

    let same_scope = left.owner.source_owner == right.owner.source_owner
        && left.owner.module == right.owner.module;
    let existing = left
        .methods
        .iter()
        .zip(&right.methods)
        .all(|(first, second)| same_trait_member(&first.origin, &second.origin));
    if same_scope && existing {
        return Decision::Suppress(
            "Both ordered roles already belong to the same exact local trait members",
        );
    }

    Decision::Unavailable(
        "Mixed, partial or different local trait coverage needs a broader capability contract",
    )
}

fn same_trait_member(left: &MethodOrigin, right: &MethodOrigin) -> bool {
    matches!(
        (left, right),
        (
            MethodOrigin::LocalTrait { .. },
            MethodOrigin::LocalTrait { .. }
        )
    ) && left == right
}

struct EvaluatedPair {
    left: usize,
    right: usize,
    decision: Decision,
}

impl EvaluatedPair {
    fn proposal(self) -> Proposal {
        Proposal {
            left: self.left,
            right: self.right,
            roles: [
                Role {
                    left_method: 0,
                    right_method: 0,
                },
                Role {
                    left_method: 1,
                    right_method: 1,
                },
            ],
        }
    }

    fn decision(self) -> PairDecision {
        let reason = match self.decision {
            Decision::Suppress(reason) | Decision::Unavailable(reason) => reason,
            Decision::Propose => unreachable!("proposal was partitioned from pair decisions"),
        };

        PairDecision {
            left: self.left,
            right: self.right,
            reason,
        }
    }
}

impl Report {
    fn completed(expected: usize, pairs: Vec<EvaluatedPair>) -> Self {
        let (proposals, remaining): (Vec<_>, Vec<_>) = pairs
            .into_iter()
            .partition(|pair| matches!(pair.decision, Decision::Propose));
        let (suppressed, unavailable): (Vec<_>, Vec<_>) = remaining
            .into_iter()
            .partition(|pair| matches!(pair.decision, Decision::Suppress(_)));

        Self {
            execution: Execution::Completed,
            complete: unavailable.is_empty(),
            expected_pairs: Some(expected),
            assessed_pairs: expected,
            proposals: proposals.into_iter().map(EvaluatedPair::proposal).collect(),
            suppressed: suppressed
                .into_iter()
                .map(EvaluatedPair::decision)
                .collect(),
            unavailable: unavailable
                .into_iter()
                .map(EvaluatedPair::decision)
                .collect(),
            reason: None,
        }
    }

    fn exhausted(expected: Option<usize>) -> Self {
        Self {
            execution: Execution::BudgetExhausted,
            complete: false,
            expected_pairs: expected,
            assessed_pairs: 0,
            proposals: vec![],
            suppressed: vec![],
            unavailable: vec![],
            reason: Some("Capability pair budget exhausted before candidate evaluation"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstyle_pdg::{ByteSpan, consumer_facts::dispatch::capabilities::MethodWitness};

    fn span(start: u32) -> ByteSpan {
        ByteSpan {
            start,
            end: start + 1,
        }
    }

    fn wrapper(name: &str) -> Wrapper {
        Wrapper {
            owner: OwnerKey {
                source_owner: 0,
                module: 0,
                name: name.into(),
            },
            owner_name: name.into(),
            caller_header: span(0),
            methods: vec![
                MethodWitness {
                    name: "first".into(),
                    call_span: span(10),
                    callee_header: span(100),
                    origin: MethodOrigin::Inherent,
                },
                MethodWitness {
                    name: "second".into(),
                    call_span: span(20),
                    callee_header: span(200),
                    origin: MethodOrigin::Inherent,
                },
            ],
        }
    }

    fn trait_origin(member: &str, header: u32) -> MethodOrigin {
        MethodOrigin::LocalTrait {
            trait_name: "Existing".into(),
            member: member.into(),
            trait_header: span(header),
        }
    }

    fn existing(wrapper: &mut Wrapper) {
        wrapper.methods[0].origin = trait_origin("first", 50);
        wrapper.methods[1].origin = trait_origin("second", 50);
    }

    #[test]
    fn inherent_roles_follow_original_input_and_method_order() {
        let first = wrapper("First");
        let mut second = wrapper("Second");
        second.methods.swap(0, 1);
        let report = evaluate(&[&second, &first], 1).unwrap();
        assert!(report.complete);
        assert_eq!(report.assessed_pairs, 1);
        assert_eq!(
            report.proposals,
            vec![Proposal {
                left: 0,
                right: 1,
                roles: [
                    Role {
                        left_method: 0,
                        right_method: 0
                    },
                    Role {
                        left_method: 1,
                        right_method: 1
                    }
                ]
            }]
        );
    }

    #[test]
    fn same_owner_exclusion_uses_exact_source_and_module_identity() {
        let first = wrapper("Same");
        let mut other_module = wrapper("Same");
        other_module.owner.module = 1;
        let mut other_file = wrapper("Same");
        other_file.owner.source_owner = 1;
        assert_eq!(
            evaluate(&[&first, &first], 0).unwrap().expected_pairs,
            Some(0)
        );
        assert_eq!(
            evaluate(&[&first, &other_module, &other_file], 3)
                .unwrap()
                .proposals
                .len(),
            3
        );
    }

    #[test]
    fn exact_existing_trait_roles_are_suppressed_without_a_proposal() {
        let mut first = wrapper("First");
        let mut second = wrapper("Second");
        existing(&mut first);
        existing(&mut second);
        let report = evaluate(&[&first, &second], 1).unwrap();
        assert!(report.complete);
        assert!(report.proposals.is_empty());
        assert_eq!(report.suppressed.len(), 1);
        assert!(report.unavailable.is_empty());
    }

    #[test]
    fn trait_header_scope_and_member_identity_all_matter() {
        let mut first = wrapper("First");
        existing(&mut first);
        for change in 0..5 {
            let mut second = wrapper("Second");
            existing(&mut second);
            match change {
                0 => second.owner.source_owner = 1,
                1 => second.owner.module = 1,
                2 => second.methods[1].origin = trait_origin("second", 51),
                3 => second.methods[1].origin = trait_origin("different", 50),
                _ => {
                    second.methods[1].origin = MethodOrigin::LocalTrait {
                        trait_name: "Different".into(),
                        member: "second".into(),
                        trait_header: span(50),
                    }
                }
            }
            let report = evaluate(&[&first, &second], 1).unwrap();
            assert!(!report.complete);
            assert!(report.proposals.is_empty());
            assert!(report.suppressed.is_empty());
            assert_eq!(report.unavailable.len(), 1);
        }
    }

    #[test]
    fn mixed_and_partial_trait_coverage_stays_unavailable() {
        let first = wrapper("First");
        let mut second = wrapper("Second");
        second.methods[0].origin = trait_origin("first", 50);
        assert_eq!(
            evaluate(&[&first, &second], 1).unwrap().unavailable.len(),
            1
        );
        let mut third = wrapper("Third");
        existing(&mut third);
        assert_eq!(
            evaluate(&[&second, &third], 1).unwrap().unavailable.len(),
            1
        );
    }

    #[test]
    fn interleaved_owner_groups_keep_original_pair_order() {
        let last_name = wrapper("Zed");
        let first_name = wrapper("Alpha");
        let middle_name = wrapper("Middle");
        let report = evaluate(&[&last_name, &first_name, &last_name, &middle_name], 5).unwrap();
        let pairs: Vec<_> = report
            .proposals
            .iter()
            .map(|proposal| (proposal.left, proposal.right))
            .collect();
        assert_eq!(pairs, vec![(0, 1), (0, 3), (1, 2), (1, 3), (2, 3)]);
    }

    #[test]
    fn pair_population_budget_is_preflighted_with_no_partial_proposals() {
        let first = wrapper("First");
        let second = wrapper("Second");
        let third = wrapper("Third");
        let report = evaluate(&[&first, &first, &second, &third], 4).unwrap();
        assert_eq!(report.execution, Execution::BudgetExhausted);
        assert_eq!(report.expected_pairs, Some(5));
        assert_eq!(report.assessed_pairs, 0);
        assert!(!report.complete);
        assert!(report.proposals.is_empty());
        assert_eq!(
            evaluate(&[&first, &first, &second, &third], 5)
                .unwrap()
                .proposals
                .len(),
            5
        );
    }

    #[test]
    fn invalid_witness_rosters_fail_before_budget_or_owner_exclusion() {
        let mut invalid = wrapper("Invalid");
        invalid.methods.pop();
        assert!(evaluate(&[&invalid, &invalid], 0).is_err());
        let mut duplicate = wrapper("Duplicate");
        duplicate.methods[1].callee_header = duplicate.methods[0].callee_header;
        assert!(evaluate(&[&duplicate], 0).is_err());
        let report = evaluate(&[], 0).unwrap();
        assert!(report.complete);
        assert_eq!(report.expected_pairs, Some(0));
    }
}
