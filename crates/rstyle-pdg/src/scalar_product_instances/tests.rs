use super::*;
mod integration;
mod nonzero;

const ORIGINAL: &str =
    "#[derive(Debug)] pub(crate) struct ParsedItem<'a, T>(pub(crate) &'a [u8], pub(crate) T);";

fn observation(source: &str, argument: Argument) -> Observation {
    let source: crate::SourceText = source.into();
    let spellings = concrete_types::field_declarations(&source).unwrap();
    let view = TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(TypeTree::Scalar(
        "u8".into(),
    )))));
    let fields = [view, TypeTree::Scalar(argument.as_str().into())]
        .into_iter()
        .enumerate()
        .map(|(position, ty)| Field {
            position,
            name: position.to_string(),
            declaration: spellings
                .get(position)
                .map_or("", |(_, value)| value)
                .into(),
            ty,
        })
        .collect();
    Observation {
        nominal: Nominal {
            source_identity: Some("source-package".into()),
            package_name: "time".into(),
            package_version: "0.3.47".into(),
            manifest_digest: crate::source_identity::checksum(b"manifest"),
            definition_path: "parsing::ParsedItem".into(),
            declaration_digest: source.checksum().into(),
        },
        source,
        argument,
        fields,
    }
}

#[test]
fn original_non_copy_declaration_retains_both_concrete_substitutions() {
    let byte = observation(ORIGINAL, Argument::U8);
    let short = observation(ORIGINAL, Argument::U16);
    assert!(byte.check_shape().is_ok());
    assert!(short.check_shape().is_ok());
    assert_eq!(byte.source.as_str(), ORIGINAL);
    assert_eq!(byte.fields[0].declaration.as_str(), "&'a [u8]");
    assert_eq!(byte.fields[1].declaration.as_str(), "T");
    assert_eq!(byte.nominal, short.nominal);
    assert_ne!(byte, short);
    assert!(byte.nominal.declaration_digest.starts_with("xxh3-64:"));
}

#[test]
fn parameter_names_do_not_supply_nominal_or_copy_authority() {
    let source = "struct ParsedItem<'input, Value>(&'input [u8], Value);";
    assert!(observation(source, Argument::U16).check_shape().is_ok());
}

#[test]
fn exact_declared_lifetime_and_scalar_field_are_required() {
    for source in [
        "struct ParsedItem<'a, T>(&'static [u8], T);",
        "struct ParsedItem<'a, T>(&[u8], T);",
        "struct ParsedItem<'a, T>(&'a mut [u8], T);",
        "struct ParsedItem<'a, T>(&'a [u16], T);",
        "struct ParsedItem<'a, T>(&'a [u8], u8);",
        "struct ParsedItem<'a, T>(T, &'a [u8]);",
        "struct ParsedItem<'a, T>(&'a [u8], Option<T>);",
        "struct ParsedItem<'a, T>(&'a [u8], &'a T);",
        "struct ParsedItem<'a, T>(&'a [u8], T, bool);",
        "struct ParsedItem<'a, T> { input: &'a [u8], value: T }",
        "struct ParsedItem<'a, T>(&'a [u8], #[cfg(feature = \"x\")] T);",
    ] {
        assert!(
            observation(source, Argument::U8).check_shape().is_err(),
            "{source}"
        );
    }
}

#[test]
fn unobserved_parameters_bounds_and_defaults_cannot_hide_in_source() {
    for source in [
        "struct ParsedItem<'a, T: Copy>(&'a [u8], T);",
        "struct ParsedItem<'a, T = u8>(&'a [u8], T);",
        "struct ParsedItem<'a, T>(&'a [u8], T) where T: Copy;",
        "struct ParsedItem<'a: 'static, T>(&'a [u8], T);",
        "struct ParsedItem<'a, T, U>(&'a [u8], T);",
        "struct ParsedItem<'a, 'b, T>(&'a [u8], T);",
        "struct ParsedItem<'a, T, const N: usize>(&'a [u8], T);",
        "struct ParsedItem<T>(&'static [u8], T);",
        "struct ParsedItem<'a, u8>(&'a [u8], u8);",
    ] {
        assert!(
            observation(source, Argument::U8).check_shape().is_err(),
            "{source}"
        );
    }
}

#[test]
fn concrete_substitution_and_complete_field_roster_are_independent_checks() {
    let base = observation(ORIGINAL, Argument::U8);
    let mut wrong_argument = base.clone();
    wrong_argument.argument = Argument::U16;
    let mut wrong_scalar = base.clone();
    wrong_scalar.fields[1].ty = TypeTree::Scalar("u32".into());
    let mut wrong_view = base.clone();
    wrong_view.fields[0].ty = TypeTree::Scalar("u8".into());
    let mut reordered = base.clone();
    reordered.fields.swap(0, 1);
    let mut wrong_position = base.clone();
    wrong_position.fields[1].position = 0;
    let mut wrong_name = base.clone();
    wrong_name.fields[1].name = "value".into();
    let mut wrong_spelling = base.clone();
    wrong_spelling.fields[1].declaration = "u8".into();
    let mut missing = base.clone();
    missing.fields.pop();
    let mut extra = base.clone();
    extra.fields.push(base.fields[1].clone());
    for changed in [
        wrong_argument,
        wrong_scalar,
        wrong_view,
        reordered,
        wrong_position,
        wrong_name,
        wrong_spelling,
        missing,
        extra,
    ] {
        assert!(changed.check_shape().is_err(), "{changed:?}");
    }
    assert!(base.check_shape().is_ok());
}

#[test]
fn nominal_bytes_package_and_source_budget_are_checked() {
    let base = observation(ORIGINAL, Argument::U8);
    let mut wrong_name = base.clone();
    wrong_name.nominal.definition_path = "parsing::Other".into();
    let mut changed_bytes = base.clone();
    changed_bytes.source = ORIGINAL.replace("Debug", "Clone").into();
    let mut wrong_manifest = base.clone();
    wrong_manifest.nominal.manifest_digest = "unchecked".into();
    let mut missing_package = base.clone();
    missing_package.nominal.package_name.clear();
    let large_source = format!(
        "{}{}",
        " ".repeat(concrete_types::MAX_SOURCE_BYTES),
        ORIGINAL
    );
    let oversized = observation(&large_source, Argument::U8);
    for changed in [
        wrong_name,
        changed_bytes,
        wrong_manifest,
        missing_package,
        oversized,
    ] {
        assert!(changed.check_shape().is_err());
    }
}

#[test]
fn raw_wire_arguments_and_fields_are_closed_not_permission_flags() {
    assert_eq!(serde_json::to_string(&Argument::U8).unwrap(), "\"u8\"");
    assert_eq!(
        serde_json::from_str::<Argument>("\"u16\"").unwrap(),
        Argument::U16
    );
    for unknown in ["\"u32\"", "\"U8\"", "null", "{\"kind\":\"u8\"}"] {
        assert!(serde_json::from_str::<Argument>(unknown).is_err());
    }
    let mut raw = serde_json::to_value(observation(ORIGINAL, Argument::U8)).unwrap();
    raw["no_drop"] = serde_json::json!(true);
    assert!(serde_json::from_value::<Observation>(raw).is_err());
}
