use super::*;
use crate::clones::{
    diagnostic_selection::{self, Rule},
    review::{
        ReviewStore, baseline_common, compiler_baseline,
        compiler_diagnostics::{retained, rules::Entry},
        numeric_workflow::Retained,
        status,
    },
};
use baseline_common::CompilerInventory;
use std::cell::Cell;

const EXHAUSTED: &str = "Compiler diagnostic owner payload budget exhausted";

fn function(index: usize, params: usize) -> FnFacts {
    serde_json::from_value(serde_json::json!({
        "id":format!("sample::f{index}"), "path":"src/lib.rs", "line":index + 1,
        "column":1, "name":format!("f{index}"), "params":params,
        "has_self":false, "statements":1, "source":"fn go(){}",
        "preceding_line":"", "mutations":[], "calls":[], "effects":[]
    }))
    .unwrap()
}

fn owner() -> Owner {
    serde_json::from_value(serde_json::json!({
        "locator":{
            "package":{"identity":"Cargo.toml","name":"sample","version":"1.0.0"},
            "source":"src/lib.rs", "qualified_name":"sample::original",
            "declaration_kind":"compiler-function", "discriminator":"unique-declaration"
        },
        "context":{
            "compiler_commit":"compiler", "edition":"2024", "data_layout":"layout",
            "compilation_target":{"name":"sample","kind":["lib"],"test":false},
            "target_triple":"aarch64-unknown-linux-gnu", "pointer_width":64,
            "features":[], "cfgs":["unix"], "opt_level":"0", "debug_assertions":true,
            "overflow_checks":true, "panic_strategy":"unwind", "extraction_policy":"hir-data/1"
        },
        "signature":{
            "parameters":[], "result":{"normalized":"u128","observed":"u128","spelling":"u128"},
            "method":null, "abi":{"status":"known","value":"Rust"}, "generics":[],
            "has_self":false, "is_variadic":false, "is_async":false, "is_unsafe":false,
            "is_const":false
        }
    }))
    .unwrap()
}

fn request<'a>(demand: &'a Demand<'a>) -> Request<'a> {
    Request {
        policy: Policy {
            config: &demand.policy,
            metrics: rstyle_core::advisory::METRICS,
        },
        demand,
        rules: Some(diagnostic_selection::Request {
            rules: vec![Rule::FnParams],
            ..Default::default()
        }),
    }
}

fn resolve(facts: &FnFacts, calls: &Cell<usize>, template: &Owner) -> Result<Rc<Owner>, String> {
    calls.set(calls.get() + 1);
    let mut owner = template.clone();
    owner.locator.qualified_name = rstyle_core::locator::base(facts);
    Ok(Rc::new(owner))
}

fn baseline(policy: &Rc<serde_json::Value>, owner: &Owner) -> compiler_baseline::Baseline {
    compiler_baseline::Baseline::new(
        Policy {
            config: policy,
            metrics: rstyle_core::advisory::METRICS,
        },
        vec![owner.context.clone()],
    )
    .unwrap()
}

fn finding(index: usize, policy: &Rc<serde_json::Value>, owner: &Owner) -> Snapshot {
    Snapshot {
        owner: resolve(&function(index, 5), &Cell::new(0), owner).unwrap(),
        versions: super::super::super::local_adapter::versions("compiler-diagnostics"),
        metrics: rstyle_core::advisory::METRICS.into(),
        policy: Rc::clone(policy),
        rule_id: "fn_params".into(),
        actual: 5,
        limit: 4,
        details: vec![],
    }
}

fn positive(count: usize) -> Evaluation {
    rstyle_core::evaluate_with_scopes(
        (0..count).map(|index| function(index, 5)).collect(),
        &rstyle_core::config::Config::default(),
    )
    .unwrap()
}

fn late_evaluation() -> Evaluation {
    let mut evaluation = rstyle_core::evaluate_with_scopes(
        (0..4101)
            .map(|index| function(index, usize::from(index == 4100) * 5))
            .collect(),
        &rstyle_core::config::Config::default(),
    )
    .unwrap();
    evaluation
        .scopes
        .as_mut()
        .unwrap()
        .excluded
        .push(function(4101, 0));
    evaluation
}

fn resolved_name(owners: &Owners, index: Index) -> &str {
    &owners[&index].as_ref().unwrap().locator.qualified_name
}

fn retained_state(owners: &[Assessed], previous: &Snapshot) -> status::State {
    let assessed = owners
        .iter()
        .find(|owner| {
            owner.qualified_name() == Some(previous.owner.locator.qualified_name.as_str())
        })
        .unwrap();
    retained::Row {
        assessed,
        unique: true,
        snapshots: &[],
        policy: &previous.policy,
        metrics: rstyle_core::advisory::METRICS,
        selected: true,
        effect_scope_complete: true,
        request: None,
    }
    .assess(previous)
    .0
}

fn retained_outcomes(
    owners: &[Assessed],
    baseline: &compiler_baseline::Baseline,
    store: &ReviewStore,
) {
    assert_eq!(
        retained_state(owners, &baseline.entries[0]),
        status::State::NoFinding
    );
    assert_eq!(
        retained_state(owners, &store.compiler_diagnostic_entries[0].finding),
        status::State::NoFinding
    );
    assert_eq!(
        retained_state(owners, &store.compiler_diagnostic_entries[1].finding),
        status::State::Unavailable
    );
}

#[test]
fn late_positive_and_retained_selected_and_excluded_owners_skip_unrelated_functions() {
    let owner = owner();
    let policy = Rc::new(serde_json::json!({}));
    let mut baseline = baseline(&policy, &owner);
    baseline.entries.push(finding(4099, &policy, &owner));
    let store = ReviewStore {
        compiler_diagnostic_entries: [4098, 4101]
            .map(|index| Entry {
                finding: finding(index, &policy, &owner),
                reason: "Reviewed fixture".into(),
            })
            .into(),
        ..Default::default()
    };
    let demand = Demand::new(policy, Some(&baseline), Some(&store));
    let request = request(&demand);
    let mut evaluation = late_evaluation();
    let scopes = evaluation.scopes.take().unwrap();
    let calls = Cell::new(0);

    let (owners, truncated) = resolve_owners(&evaluation, Some(&scopes), &request, |facts| {
        resolve(facts, &calls, &owner)
    });
    assert_eq!(calls.get(), 4);
    assert!(!truncated);
    let expected = [
        Index::Selected(4098),
        Index::Selected(4099),
        Index::Selected(4100),
        Index::Excluded(0),
    ];
    assert!(owners.keys().copied().eq(expected));
    assert_eq!(
        resolved_name(&owners, Index::Selected(4100)),
        "sample::f4100"
    );
    assert_eq!(resolved_name(&owners, Index::Excluded(0)), "sample::f4101");
    let snapshots = snapshots(&evaluation, &owners, &request.policy);
    assert_eq!(snapshots.len(), 1);
    assert_eq!(
        snapshots[0].as_ref().unwrap().owner.locator.qualified_name,
        "sample::f4100"
    );
    let assessed = assessed(&evaluation, scopes, owners);
    retained_outcomes(&assessed, &baseline, &store);
}

#[test]
fn exactly_4096_materialized_owners_succeed_and_share_the_budget_with_the_next_unit() {
    let owner = owner();
    let demand = Demand::new(Rc::new(serde_json::json!({})), None, None);
    let request = request(&demand);
    let evaluation = positive(4096);
    let calls = Cell::new(0);

    let (owners, truncated) = resolve_owners(&evaluation, None, &request, |facts| {
        resolve(facts, &calls, &owner)
    });
    assert_eq!(calls.get(), 4096);
    assert_eq!(owners.len(), 4096);
    assert!(owners.values().all(Result::is_ok));
    assert!(!truncated);
    let next_unit = positive(1);
    let (owners, truncated) = resolve_owners(&next_unit, None, &request, |facts| {
        resolve(facts, &calls, &owner)
    });
    assert_eq!(calls.get(), 4096);
    assert!(truncated);
    assert_eq!(owners[&Index::Selected(0)].as_ref().unwrap_err(), EXHAUSTED);
}

#[test]
fn diagnostic_owner_deduplication_and_rule_selection_preserve_the_exact_export_boundary() {
    let owner = owner();
    let policy = Rc::new(serde_json::json!({}));
    let demand = Demand::new(Rc::clone(&policy), None, None);
    let request = request(&demand);
    let mut evaluation = positive(4098);
    let mut duplicate = evaluation.report.diagnostics[0].clone();
    duplicate.rule_id = "fn_length".into();
    evaluation.report.diagnostics.insert(0, duplicate);
    evaluation.diagnostic_owners.insert(0, 4097);
    evaluation.report.diagnostics[4098].rule_id = "fn_length".into();
    let duplicate = evaluation.report.diagnostics[1].clone();
    evaluation.report.diagnostics.push(duplicate);
    evaluation
        .diagnostic_owners
        .push(evaluation.diagnostic_owners[1]);
    let calls = Cell::new(0);

    let (owners, truncated) = resolve_owners(&evaluation, None, &request, |facts| {
        resolve(facts, &calls, &owner)
    });
    assert_eq!(calls.get(), 4096);
    assert_eq!(owners.len(), 4097);
    assert!(truncated);
    assert!(!owners.contains_key(&Index::Selected(4097)));
    assert!(owners.values().take(4096).all(Result::is_ok));
    assert_eq!(
        owners[&Index::Selected(4096)].as_ref().unwrap_err(),
        EXHAUSTED
    );
    let snapshots = snapshots(&evaluation, &owners, &request.policy);
    assert_eq!(
        snapshots[0].as_ref().unwrap_err(),
        "Compiler diagnostic rule was not requested"
    );
    let requested = snapshots.into_iter().skip(1).take(4097).collect();
    let result = baseline_common::publication(baseline(&policy, &owner), true, || requested);
    assert_eq!(result.unwrap_err(), EXHAUSTED);
}
