use super::*;

#[test]
fn original_question_mark_requires_the_complete_expression() {
    for text in ["check()?", "(check())?", "(check()?)"] {
        let expression: syn::Expr = syn::parse_str(text).unwrap();
        source::question_mark(&expression).unwrap();
    }
    for text in [
        "check()",
        "match check() { _ => () }",
        "Ok(check())",
        "#[other] check()?",
    ] {
        let expression: syn::Expr = syn::parse_str(text).unwrap();
        assert!(source::question_mark(&expression).is_err(), "{text}");
    }
}

#[test]
fn supplemental_witness_is_closed_and_requires_all_selection_evidence() {
    for text in [
        "{}",
        "{\"identity\":\"rstyle.rust.same-error-result-propagation/1\"}",
        "{\"identity\":\"rstyle.rust.same-error-result-propagation/1\",\"unknown\":0}",
    ] {
        assert!(serde_json::from_str::<capture::Witness>(text).is_err());
    }
}
