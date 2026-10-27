//! Nonzero payloads retain exact core identity and their original generic field.
use super::*;
use crate::nonzero_values::{Definition, Observation as NonZero, Scalar};

fn payload(scalar: Scalar) -> TypeTree {
    TypeTree::NonZero(NonZero {
        definition: Definition {
            path: "core::num::NonZero".into(),
            definition_identity:
                "DefPathHash(Fingerprint(5614226571623188065, 17233351565658270785))".into(),
            crate_name: "core".into(),
            stable_crate_id: "StableCrateId(5614226571623188065)".into(),
            local: false,
        },
        scalar,
    })
}

#[test]
fn exact_nonzero_substitution_preserves_the_original_non_copy_product() {
    for (scalar, argument) in [
        (Scalar::U8, Argument::NonZeroU8),
        (Scalar::U16, Argument::NonZeroU16),
    ] {
        let mut instance = observation(ORIGINAL, Argument::U8);
        instance.argument = argument;
        instance.fields[1].ty = payload(scalar);
        instance.check_shape().unwrap();
        let tree = TypeTree::ScalarProductInstance(Box::new(instance.clone()));
        let token = crate::data_types::token(&tree).unwrap();
        assert_eq!(
            crate::data_types::parse(&token).unwrap(),
            crate::data_types::TypeToken::from_tree(&tree)
        );
        assert_eq!(instance.source.as_str(), ORIGINAL);

        instance.fields[1].ty = TypeTree::Scalar(scalar.spelling().into());
        assert!(
            instance.check_shape().is_err(),
            "nonzero cannot become an unrestricted scalar"
        );
    }
}

#[test]
fn nonzero_width_and_nominal_identity_cannot_be_substituted() {
    let mut instance = observation(ORIGINAL, Argument::U8);
    instance.argument = Argument::NonZeroU8;
    instance.fields[1].ty = payload(Scalar::U16);
    assert!(instance.check_shape().is_err());
    instance.fields[1].ty = payload(Scalar::U8);
    let TypeTree::NonZero(observed) = &mut instance.fields[1].ty else {
        unreachable!()
    };
    observed.definition.local = true;
    assert!(instance.check_shape().is_err());
}

#[test]
fn paired_transformation_preserves_nominal_source_and_exact_suffix_type() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../reference_instances/operations/fixtures.json"
    ))
    .unwrap();
    let facts: crate::HirFnFacts = serde_json::from_value(fixture[0]["canonical"].clone()).unwrap();

    for (scalar, source_arg, result_arg) in [
        (Scalar::U8, Argument::U8, Argument::NonZeroU8),
        (Scalar::U16, Argument::U16, Argument::NonZeroU16),
    ] {
        let input = observation(ORIGINAL, source_arg);
        let mut output = input.clone();
        output.argument = result_arg;
        output.fields[1].ty = payload(scalar);
        let checked = transformation::verify(&input, &output, &facts).unwrap();
        assert_eq!(checked.input(), &input);
        assert_eq!(checked.output(), &output);
        assert!(checked.payload().construct(0).unwrap().is_none());
        assert_eq!(checked.payload().construct(1).unwrap().unwrap().get(), 1);

        output.nominal.package_name = "unrelated-owner".into();
        assert!(transformation::verify(&input, &output, &facts).is_err());
    }
}
