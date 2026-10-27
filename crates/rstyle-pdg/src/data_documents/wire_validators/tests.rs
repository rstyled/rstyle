//! Independent source expectations precede any graph admission measurement.
use super::*;

const VERSION: &str = r#"fn version(slice: &[u8], expected: u32) -> Result<usize, Error> {
    let (n, nr) = reader(slice, "version")?;
    assert_eq!(nr, size());
    if n != expected { return Err(Error::mismatch(expected, n)); }
    Ok(nr)
}"#;
const ENDIAN: &str = r#"fn endian(slice: &[u8]) -> Result<usize, Error> {
    let (n, nr) = reader(slice, "endianness check")?;
    assert_eq!(nr, size());
    if n != 0xFEFF { return Err(Error::mismatch(0xFEFF, n)); }
    Ok(nr)
}"#;

#[test]
fn complete_validator_source_topology_is_independent_of_callable_names() {
    for text in [VERSION, ENDIAN] {
        assert!(source::syntax_roles(&syn::parse_str(text).unwrap()).is_ok());
        let renamed = text
            .replace("reader", "renamed")
            .replace("mismatch", "other");
        assert!(source::syntax_roles(&syn::parse_str(&renamed).unwrap()).is_ok());
    }
}

#[test]
fn extra_work_missing_assertion_else_and_attributed_source_remain_closed() {
    for (before, after) in [
        ("assert_eq!(nr, size());", ""),
        (
            "assert_eq!(nr, size());",
            "side_effect(); assert_eq!(nr, size());",
        ),
        ("assert_eq!(nr, size());", "debug_assert_eq!(nr, size());"),
        ("let (n, nr)", "#[allow(unused)] let (n, nr)"),
        (
            "if n != expected { return Err(Error::mismatch(expected, n)); }",
            "if n != expected { return Err(Error::mismatch(expected, n)); } else { side_effect(); }",
        ),
        (
            "return Err(Error::mismatch(expected, n));",
            "side_effect(); return Err(Error::mismatch(expected, n));",
        ),
        ("fn version(", "unsafe fn version("),
    ] {
        let text = VERSION.replace(before, after);
        assert!(
            source::syntax_roles(&syn::parse_str(&text).unwrap()).is_err(),
            "{text}"
        );
    }
}

#[test]
fn error_route_requires_exact_saved_message_and_explicit_error_case() {
    use crate::copy_tagged_values::origins::CaseKind;

    let original = route_fixture();
    assert!(source::error_route(&original).is_ok());
    for mutation in 0..5 {
        let mut changed = original.clone();
        match mutation {
            0 => changed.alternatives[0].leaves[0].root.parameter = 0,
            1 => changed.alternatives[0].leaves[0].input.clear(),
            2 => {
                changed.alternatives[0].leaves[0].output.remove(0);
            }
            3 => changed.alternatives[0].leaves.clear(),
            _ => changed.alternatives[0].output = CaseKind::ResultOk,
        };
        assert!(
            source::error_route(&changed).is_err(),
            "mutation {mutation}"
        );
    }
}

fn route_fixture() -> crate::copy_tagged_values::origins::WireContract {
    use crate::copy_tagged_values::origins::{
        Alternative, CaseKind, Component, Condition, FormalRoot, Leaf, WireContract,
    };
    WireContract {
        input_cases: vec![],
        output_cases: vec![],
        alternatives: vec![Alternative {
            input: CaseKind::Whole,
            output: CaseKind::ResultErr,
            condition: Condition::Always,
            leaves: vec![Leaf {
                root: FormalRoot {
                    parameter: 1,
                    binding: crate::HirRef {
                        krate: 0,
                        owner: 1,
                        local: 2,
                    },
                },
                input: vec![Component::SharedReferent],
                output: vec![Component::ResultErr, Component::SharedReferent],
                guard: None,
            }],
        }],
    }
}
