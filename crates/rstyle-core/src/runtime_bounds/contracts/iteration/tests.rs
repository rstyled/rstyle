use super::{ConsumerEffect, StageEffect};
use crate::runtime_bounds::contracts::{Contracts, Handed, Runs};

const HEADER: &str = "schema = 'rstyle.library-contracts/1'\nnamespace = 't'\n";
const STAGE: &str = "[[stage]]\nbasis = 'Stores a predicate without running it'\n\
    effect = 'Filter'\narity = 1\ncallback = 0\noperations = ['I.filter']\n";
const CONSUMER: &str = "[[consumer]]\nbasis = 'Drives the iterator'\n\
    effect = 'Drain'\noperations = ['I.count']\n";

fn parse(entry: &str) -> Result<Contracts, String> {
    Contracts::parse_all(&[&format!("{HEADER}{entry}")])
}

#[test]
fn old_iteration_roles_do_not_gain_deferred_construction_by_default() {
    let contracts = parse(&format!("{STAGE}{CONSUMER}")).unwrap();
    let stage = contracts.stage("t:I.filter").unwrap();
    assert!(!stage.deferred);
    assert_eq!(stage.effect, StageEffect::Filter);
    assert_eq!((stage.arity, stage.callback), (1, Some(0)));
    let consumer = contracts.consumer("t:I.count").unwrap();
    assert!(!consumer.deferred);
    assert_eq!(consumer.effect, ConsumerEffect::Drain);
}

#[test]
fn an_explicit_stage_certificate_preserves_its_driven_callback_role() {
    let contracts = parse(&format!("{STAGE}deferred = true\n")).unwrap();
    assert!(contracts.stage("t:I.filter").unwrap().deferred);
    assert_eq!(
        contracts.handed("t:I.filter"),
        Some(Handed {
            callbacks: &[0],
            runs: Runs::PerItem,
        })
    );
    assert!(contracts.get("t:I.filter").is_none());
    assert!(contracts.extent("t:I.filter").is_none());
    assert!(contracts.callback_free("t:I.filter").is_none());
}

#[test]
fn driving_consumers_cannot_claim_deferred_construction() {
    let error = parse(&format!("{CONSUMER}deferred = true\n")).unwrap_err();
    assert_eq!(error, "Iteration effect Drain cannot defer construction");
    let ordinary = parse(&format!("{CONSUMER}deferred = false\n")).unwrap();
    assert!(!ordinary.consumer("t:I.count").unwrap().deferred);
}

#[test]
fn a_deferred_stage_still_requires_a_valid_callback_slot() {
    let deferred = format!("{STAGE}deferred = true\n");
    let missing = deferred.replace("callback = 0\n", "");
    assert!(parse(&missing).is_err());
    let outside = deferred.replace("callback = 0", "callback = 1");
    assert!(parse(&outside).is_err());
    let impossible = deferred.replace("effect = 'Filter'", "effect = 'Pass'");
    assert!(parse(&impossible).is_err());
}

#[test]
fn construction_certificates_use_an_exact_boolean_field() {
    for invalid in ["deferred = 'true'", "deferred = 1", "defer = true"] {
        assert!(parse(&format!("{STAGE}{invalid}\n")).is_err());
    }
    let duplicate = format!("{STAGE}deferred = true\ndeferred = false\n");
    assert!(parse(&duplicate).is_err());
}

#[test]
fn duplicate_operation_roles_cannot_override_the_construction_certificate() {
    let entry = format!("{STAGE}deferred = true\n{STAGE}deferred = false\n");
    assert_eq!(
        parse(&entry).unwrap_err(),
        "Library operation t:I.filter has two stage roles"
    );
}

#[test]
fn only_checked_rust_pass_and_filter_adapters_have_the_new_certificate() {
    let contracts = Contracts::embedded();
    for operation in [
        "Iterator.copied",
        "Iterator.enumerate",
        "Iterator.rev",
        "Iterator.filter",
        "Iterator.filter_map",
        "Iterator.skip_while",
    ] {
        assert!(
            contracts
                .stage(&format!("rust:{operation}"))
                .unwrap()
                .deferred
        );
    }
    for operation in [
        "Iterator.cloned",
        "Iterator.inspect",
        "Iterator.map",
        "Iterator.skip",
        "Iterator.take",
        "Iterator.take_while",
        "Iterator.flat_map",
    ] {
        assert!(
            !contracts
                .stage(&format!("rust:{operation}"))
                .unwrap()
                .deferred
        );
    }
    assert!(
        !contracts
            .consumer("rust:Iterator.collect")
            .unwrap()
            .deferred
    );
    assert!(!contracts.consumer("rust:Iterator.count").unwrap().deferred);
}
