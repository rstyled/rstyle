use super::*;
use crate::facts::{ParamFacts, sample};

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

fn found_under(facts: &[FnFacts], rule: &PrimitiveObsessionConfig) -> Vec<Vec<Diagnostic>> {
    let mut found = vec![Vec::new(); facts.len()];
    add(facts, rule, &mut found);
    found
}

fn found(facts: &[FnFacts]) -> Vec<Vec<Diagnostic>> {
    found_under(facts, &PrimitiveObsessionConfig::default())
}

/// What a name means by the default words.
fn means(name: &str) -> Option<String> {
    meaning(name, &PrimitiveObsessionConfig::default().meanings).map(str::to_owned)
}

fn has_a_diagnostic(found: &[Vec<Diagnostic>]) -> bool {
    found.iter().any(|function| !function.is_empty())
}

#[test]
fn a_meaning_is_a_whole_word_of_a_name() {
    assert_eq!(means("user_email").as_deref(), Some("email"));
    assert_eq!(means("userEmail").as_deref(), Some("email"));
    assert_eq!(means("user-email").as_deref(), Some("email"));
    assert_eq!(means("userID").as_deref(), Some("id"));
    assert_eq!(means("id2"), None);
    assert_eq!(means("valid"), None);
    assert_eq!(means("count"), None);
}

#[test]
fn a_project_says_what_a_name_can_mean_and_how_often_it_counts() {
    let facts = [
        function(
            "a",
            &[("iban", Some(Scalar::Text)), ("email", Some(Scalar::Text))],
        ),
        function(
            "b",
            &[("iban", Some(Scalar::Text)), ("email", Some(Scalar::Text))],
        ),
    ];
    assert!(
        has_a_diagnostic(&found(&facts)),
        "the default words hold both"
    );

    let words =
        |input: &str| -> PrimitiveObsessionConfig { toml::from_str(input).expect("a valid rule") };
    let only_iban = found_under(&facts, &words("meanings = [\"iban\"]"));
    assert_eq!(only_iban[0].len(), 1, "{only_iban:?}");
    assert!(only_iban[0][0].message.contains("`iban`"));

    let none = found_under(&facts, &words("meanings = []"));
    assert!(!has_a_diagnostic(&none), "{none:?}");

    let three = found_under(&facts, &words("min_functions = 3"));
    assert!(!has_a_diagnostic(&three), "{three:?}");

    let alone = [function("a", &[("email", Some(Scalar::Text))])];
    for one in ["min_functions = 1", "min_functions = 0"] {
        let found = found_under(&alone, &words(one));
        assert!(found[0][0].actual > found[0][0].limit, "{one}: {found:?}");
    }
}

#[test]
fn a_meaning_two_functions_hold_as_plain_text_is_reported_in_each() {
    let facts = [
        function("a", &[("email", Some(Scalar::Text))]),
        function("b", &[("userEmail", Some(Scalar::Text))]),
    ];
    let found = found(&facts);
    let [first] = &found[0][..] else {
        panic!("{found:?}");
    };
    assert_eq!(first.rule_id, RULE);
    assert!(first.actual > first.limit);
    assert!(
        first.message.contains("`email`") && first.message.contains("1 other"),
        "{}",
        first.message
    );
    assert!(found[1][0].message.contains("`userEmail`"));
}

#[test]
fn one_function_says_nothing_about_a_codebase() {
    let facts = [function("a", &[("email", Some(Scalar::Text))])];
    assert!(found(&facts)[0].is_empty());
}

#[test]
fn a_named_type_says_what_it_holds_and_only_text_and_integers_count() {
    for scalar in [None, Some(Scalar::Bool), Some(Scalar::Float)] {
        let facts = [
            function("a", &[("email", scalar)]),
            function("b", &[("email", scalar)]),
        ];
        assert!(found(&facts).iter().all(Vec::is_empty), "{scalar:?}");
    }
}

#[test]
fn text_and_integers_holding_one_meaning_are_told_apart() {
    let facts = [
        function("a", &[("id", Some(Scalar::Text))]),
        function("b", &[("id", Some(Scalar::Integer))]),
    ];
    assert!(found(&facts).iter().all(Vec::is_empty));
}

#[test]
fn a_function_reports_a_meaning_once_however_many_parameters_hold_it() {
    let parameters = [
        ("from_id", Some(Scalar::Text)),
        ("to_id", Some(Scalar::Text)),
    ];
    let facts = [function("a", &parameters), function("b", &parameters)];
    let found = found(&facts);
    assert_eq!(found[0].len(), 1, "{found:?}");
    assert!(found[0][0].message.contains("`from_id`"));
}

#[test]
fn it_runs_only_when_a_project_asks_and_an_allow_comment_covers_it() {
    let facts = vec![
        function("a", &[("email", Some(Scalar::Text))]),
        function("b", &[("email", Some(Scalar::Text))]),
    ];
    let mut located = facts.clone();
    located[1].line = 9;
    let rules = |facts: Vec<FnFacts>, config: &crate::config::Config| {
        let report = crate::evaluate(facts, config).unwrap();
        report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.rule_id == RULE)
            .count()
    };
    let mut config = crate::config::Config::default();
    assert_eq!(rules(located.clone(), &config), 0);

    config.strict_style.primitive_obsession.enabled = true;
    assert_eq!(rules(located.clone(), &config), 2);

    located[1].preceding_line =
        "// rstyle-allow(primitive_obsession): the wire's own format".into();
    assert_eq!(rules(located, &config), 1);
}
