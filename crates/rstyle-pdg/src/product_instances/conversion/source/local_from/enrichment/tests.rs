use super::*;

fn parameter() -> crate::ParamFact {
    let (facts, _, _) = crate::product_instances::test_support::conversion();
    let mut parameter = facts.signature.params[0].clone();
    parameter.ty.spelling = "Range<usize>".into();
    parameter.pat.ty.as_mut().unwrap().spelling.clear();
    parameter
}

#[test]
fn redundant_pattern_spelling_must_match_the_original_parameter_type() {
    let original = parameter();
    let mut actual = original.clone();
    actual.pat.ty.as_mut().unwrap().spelling = "Range<usize>".into();
    parameter_spelling(&mut actual, &original).unwrap();
    assert_eq!(actual, original);

    actual.pat.ty.as_mut().unwrap().spelling = "Range<u8>".into();
    assert!(parameter_spelling(&mut actual, &original).is_err());
}

#[test]
fn changed_type_spelling_cannot_authorize_pattern_enrichment() {
    let original = parameter();
    let mut actual = original.clone();
    actual.ty.spelling = "Range<u8>".into();
    actual.pat.ty.as_mut().unwrap().spelling = "Range<u8>".into();
    assert!(parameter_spelling(&mut actual, &original).is_err());
}
