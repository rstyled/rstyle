//! What sibling methods' calls say of an argument that differs, in the rule's own terms.

use super::*;
use crate::config::MethodCount;

const RECORD: &str = "go::w.record";
const OTHER: &str = "go::w.other";

/// `record(false, <value>, method)` on `line`, where `value` reads `variable` and `available`
/// hold values at the call.
fn passing(value: &str, variable: Option<&str>, available: &[&str]) -> Argument {
    Argument {
        callee: RECORD.into(),
        name: "record".into(),
        position: 1,
        line: 1,
        value: value.into(),
        text: value.into(),
        ty: Some("string".into()),
        call: vec!["false".into(), value.into(), "method".into()],
        variable: variable.map(Into::into),
        available: available.iter().map(|name| (*name).into()).collect(),
        sources: BTreeSet::new(),
    }
}

/// The variable a sibling reads, with everything a method of this shape has at the call.
fn kind() -> Argument {
    passing(
        "options.Kind",
        Some("options"),
        &["options", "name", "method"],
    )
}

fn method(identity: &str, owner: Option<&str>, arguments: Vec<Argument>) -> MethodArguments {
    let callees = arguments.iter().map(|argument| argument.callee.clone());
    MethodArguments {
        identity: identity.into(),
        path: "w.go".into(),
        allowed: false,
        arguments: Arguments {
            owner: owner.map(Into::into),
            callees: callees.collect(),
            arguments,
        },
    }
}

/// Four siblings passing `options.Kind`, and `Delete` passing `deviant`.
fn writer(deviant: Argument) -> Vec<MethodArguments> {
    let mut methods: Vec<MethodArguments> = ["Create", "Get", "List", "Update"]
        .into_iter()
        .map(|name| method(name, Some("Writer"), vec![kind()]))
        .collect();
    methods.push(method("Delete", Some("Writer"), vec![deviant]));
    methods
}

fn found(methods: &[MethodArguments]) -> Vec<(String, String)> {
    found_with(methods, &DeviantArgumentConfig::default())
}

fn found_with(
    methods: &[MethodArguments],
    limits: &DeviantArgumentConfig,
) -> Vec<(String, String)> {
    find(methods, limits)
        .into_iter()
        .map(|finding| {
            (
                methods[finding.method].identity.clone(),
                finding.defect.message,
            )
        })
        .collect()
}

fn name() -> Argument {
    passing("name", Some("name"), &["options", "name", "method"])
}

#[test]
fn a_variable_passed_where_every_sibling_passes_another_is_reported() {
    assert_eq!(
        found(&writer(name())),
        [(
            "Delete".to_owned(),
            "`record` is passed `name` here, where 4 of its 5 calls in sibling methods pass \
             `options.Kind`"
                .to_owned()
        )]
    );
}

#[test]
fn too_few_siblings_say_nothing() {
    let mut methods = writer(name());
    assert_eq!(found(&methods).len(), 1, "four methods are enough");

    methods.remove(0);
    methods.remove(0);
    assert_eq!(found(&methods), [], "three are not");
}

#[test]
fn a_project_can_ask_for_more_siblings() {
    let methods = writer(name());
    let asking = |count| DeviantArgumentConfig {
        min_methods: MethodCount(count),
        ..DeviantArgumentConfig::default()
    };
    assert_eq!(
        found_with(&methods, &asking(5)).len(),
        1,
        "five methods make the call"
    );
    assert_eq!(found_with(&methods, &asking(6)), [], "six are asked for");
}

#[test]
fn a_constant_is_a_choice_not_a_slip() {
    assert_eq!(found(&writer(passing("204", None, &["options"]))), []);
}

#[test]
fn a_call_that_differs_elsewhere_too_is_another_call() {
    let mut deviant = name();
    deviant.call[2] = "\"delete\"".into();
    assert_eq!(found(&writer(deviant)), []);
}

#[test]
fn a_variable_that_holds_nothing_yet_at_the_call_could_not_have_been_meant() {
    let deviant = passing("name", Some("name"), &["name", "method"]);
    assert_eq!(found(&writer(deviant)), []);
}

#[test]
fn a_variable_of_another_type_than_the_siblings_pass_is_another_value() {
    let mut deviant = name();
    deviant.ty = Some("*ResponseDTO".into());
    assert_eq!(found(&writer(deviant)), []);
}

#[test]
fn a_value_computed_from_the_siblings_variable_is_deliberate() {
    let mut deviant = name();
    deviant.sources.insert("options".into());
    assert_eq!(found(&writer(deviant)), []);
}

#[test]
fn methods_that_call_different_things_are_not_siblings() {
    let mut methods = writer(name());
    for unrelated in ["Get", "List", "Update"] {
        let other = Argument {
            callee: OTHER.into(),
            ..kind()
        };
        let index = methods
            .iter()
            .position(|m| m.identity == unrelated)
            .unwrap();
        methods[index] = method(
            unrelated,
            Some("Writer"),
            vec![kind(), other.clone(), other],
        );
        methods[index]
            .arguments
            .callees
            .extend(["go::a", "go::b"].map(String::from));
    }
    assert_eq!(found(&methods), []);
}

#[test]
fn functions_of_no_type_are_never_compared() {
    let methods: Vec<MethodArguments> = writer(name())
        .into_iter()
        .map(|mut method| {
            method.arguments.owner = None;
            method
        })
        .collect();
    assert_eq!(found(&methods), []);
}

#[test]
fn siblings_that_disagree_among_themselves_agree_on_nothing() {
    let mut methods = writer(name());
    methods[0].arguments.arguments[0] = passing("other", Some("other"), &["other"]);
    assert_eq!(found(&methods), []);
}
