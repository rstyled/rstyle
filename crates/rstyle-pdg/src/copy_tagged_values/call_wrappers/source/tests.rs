//! Syntax rejection tests confer no helper or nominal authority.
use super::*;

#[test]
fn whole_return_shape_rejects_evaluation_and_control_flow_changes() {
    for body in [
        "let saved = value; Output::from(saved)",
        "Output::from(value);",
        "return Output::from(value)",
        "{ Output::from(value) }",
        "if true { Output::from(value) } else { Output::from(value) }",
        "let _ = Output::from(value); Output::from(value)",
    ] {
        let source = format!("fn wrap(value: Input) -> Output {{ {body} }}");
        assert!(
            shape(&syn::parse_str(&source).unwrap(), false).is_err(),
            "{body}"
        );
    }
}

#[test]
fn source_signature_shape_rejects_extra_authority() {
    for signature in [
        "unsafe fn wrap(value: Input) -> Output",
        "async fn wrap(value: Input) -> Output",
        "const fn wrap(value: Input) -> Output",
        "extern \"C\" fn wrap(value: Input) -> Output",
        "fn wrap<T>(value: Input) -> Output",
        "fn wrap(value: Input) -> Output where Input: Copy",
        "fn wrap(value: Input, other: Input) -> Output",
        "#[inline] fn wrap(value: Input) -> Output",
    ] {
        let source = format!("{signature} {{ Output::from(value) }}");
        assert!(
            shape(&syn::parse_str(&source).unwrap(), false).is_err(),
            "{signature}"
        );
    }
}

#[test]
fn whole_return_shape_is_only_a_candidate_and_requires_independent_proofs() {
    let source = syn::parse_str("fn wrap(value: Input) -> Output { Output::from(value) }").unwrap();
    assert!(shape(&source, false).is_ok());
}

#[test]
fn source_binding_rejects_mutability_borrowing_and_subpatterns() {
    for pattern in ["mut value", "ref value", "ref mut value", "value @ _"] {
        let source = format!("fn wrap({pattern}: Input) -> Output {{ Output::from(value) }}");
        let source: syn::ItemFn = syn::parse_str(&source).unwrap();
        let syn::FnArg::Typed(parameter) = &source.sig.inputs[0] else {
            unreachable!()
        };
        let syn::Pat::Ident(binding) = parameter.pat.as_ref() else {
            unreachable!()
        };

        assert!(immutable_binding(binding).is_err(), "{pattern}");
    }
}
