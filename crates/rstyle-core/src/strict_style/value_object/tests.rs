use super::*;
use crate::facts::{ParamFacts, sample};

const TEXT: Option<Scalar> = Some(Scalar::Text);
const INT: Option<Scalar> = Some(Scalar::Integer);

fn function(name: &str, parameters: &[(&str, Option<Scalar>)]) -> FnFacts {
    let mut f = sample(name);
    f.param_list = parameters
        .iter()
        .map(|&(name, scalar)| ParamFacts {
            name: name.into(),
            ty: String::new(),
            uses: 1,
            scalar,
        })
        .collect();
    f
}

fn found(facts: &[FnFacts]) -> Vec<Vec<Diagnostic>> {
    let mut found = vec![Vec::new(); facts.len()];
    add(facts, &mut found);
    found
}

fn three_with(parameters: &[(&str, Option<Scalar>)]) -> Vec<FnFacts> {
    ["a", "b", "c"]
        .iter()
        .map(|name| function(name, parameters))
        .collect()
}

#[test]
fn values_three_functions_share_are_reported_in_each() {
    let found = found(&three_with(&[("host", TEXT), ("port", INT)]));
    for function in &found {
        let [diagnostic] = &function[..] else {
            panic!("{found:?}");
        };
        assert_eq!(diagnostic.rule_id, RULE);
        assert!(diagnostic.actual > diagnostic.limit);
        assert!(
            diagnostic.message.contains("`host`, `port`") && diagnostic.message.contains("2 other"),
            "{}",
            diagnostic.message
        );
    }
}

#[test]
fn two_functions_could_share_names_by_chance() {
    let parameters = [("host", TEXT), ("port", INT)];
    let facts = [function("a", &parameters), function("b", &parameters)];
    assert!(found(&facts).iter().all(Vec::is_empty));
}

#[test]
fn a_function_with_more_still_holds_the_group() {
    let facts = [
        function("a", &[("host", TEXT), ("port", INT)]),
        function("b", &[("host", TEXT), ("port", INT), ("timeout", INT)]),
        function("c", &[("port", INT), ("host", TEXT), ("user", TEXT)]),
    ];
    let found = found(&facts);
    assert!(
        found.iter().all(|function| function.len() == 1),
        "{found:?}"
    );
    assert!(found[1][0].message.contains("`host`, `port`"));
}

#[test]
fn only_the_largest_group_is_reported() {
    let parameters = [("host", TEXT), ("port", INT), ("user", TEXT)];
    let found = found(&three_with(&parameters));
    let [diagnostic] = &found[0][..] else {
        panic!("{found:?}");
    };
    assert!(
        diagnostic.message.contains("`host`, `port`, `user`"),
        "{}",
        diagnostic.message
    );
}

#[test]
fn a_named_type_already_says_what_it_holds() {
    let found = found(&three_with(&[("host", None), ("port", None)]));
    assert!(found.iter().all(Vec::is_empty), "{found:?}");
}

#[test]
fn one_shared_value_has_nothing_to_travel_with() {
    let facts = [
        function("a", &[("host", TEXT), ("a", INT)]),
        function("b", &[("host", TEXT), ("b", INT)]),
        function("c", &[("host", TEXT), ("c", INT)]),
    ];
    assert!(found(&facts).iter().all(Vec::is_empty));
}

#[test]
fn placeholders_describe_nothing_unless_a_name_among_them_does() {
    let nothing = found(&three_with(&[("x", INT), ("y", INT)]));
    assert!(nothing.iter().all(Vec::is_empty), "{nothing:?}");
    let numbered = found(&three_with(&[("x1", INT), ("y1", INT)]));
    assert!(numbered.iter().all(Vec::is_empty), "{numbered:?}");

    let something = found(&three_with(&[("x", INT), ("width", INT)]));
    assert!(something.iter().all(|function| function.len() == 1));
}

#[test]
fn a_name_written_to_be_ignored_is_not_part_of_a_group() {
    let found = found(&three_with(&[("_host", TEXT), ("port", INT)]));
    assert!(found.iter().all(Vec::is_empty), "{found:?}");
}

#[test]
fn it_runs_only_when_a_project_asks() {
    let mut facts = three_with(&[("host", TEXT), ("port", INT)]);
    for (line, function) in facts.iter_mut().enumerate() {
        function.line = line + 1;
    }
    let count = |config: &crate::config::Config| {
        let report = crate::evaluate(facts.clone(), config).unwrap();
        let ours = report.diagnostics.iter();
        ours.filter(|diagnostic| diagnostic.rule_id == RULE).count()
    };
    let mut config = crate::config::Config::default();
    assert_eq!(count(&config), 0);

    config.strict_style.value_object = true;
    assert_eq!(count(&config), 3);
}
