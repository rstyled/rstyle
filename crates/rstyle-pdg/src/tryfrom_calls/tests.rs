//! Independently specified syntax/identity expectations, separate from admission.
use super::{model, owner, selection, syntax};
use serde_json::{Value, json};

const IMPL: &str = "impl TryFrom<usize> for Index { type Error = IndexError; fn try_from(i: usize) -> Result<Index, IndexError> { if i > 7 { return Err(IndexError { value: i }); } Ok(Index { value: i }) } }";

fn syntax(bytes: &str) -> Result<syntax::Paths, String> {
    let item: syn::ItemImpl = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let method = item
        .items
        .iter()
        .find_map(|member| match member {
            syn::ImplItem::Fn(method) => Some(method),
            _ => None,
        })
        .ok_or("Missing method")?;
    syntax::implementation(&item, method, "usize")
}

#[test]
fn concrete_source_preserves_arbitrary_body_for_independent_effect_verification() {
    for body in [
        IMPL.to_string(),
        IMPL.replace(
            "Ok(Index { value: i })",
            "panic!(\"body must be verified\")",
        ),
    ] {
        let paths = syntax(&body).unwrap();
        assert!(paths.self_type.path.is_ident("Index"));
        assert!(paths.error_type.path.is_ident("IndexError"));
        assert!(syntax::result_arguments(&paths.result_type).is_ok());
    }
}

#[test]
fn concrete_source_rejects_unbounded_signatures_and_incomplete_rosters() {
    for (before, after) in [
        ("impl TryFrom", "impl<T> TryFrom"),
        ("impl TryFrom", "unsafe impl TryFrom"),
        ("TryFrom<usize>", "TryFrom<u32>"),
        ("for Index", "for Index<u32>"),
        ("type Error = IndexError;", "type Error = IndexError<u32>;"),
        ("type Error = IndexError;", ""),
        (
            "type Error = IndexError;",
            "type Error = IndexError; const X:usize=0;",
        ),
        ("fn try_from(i: usize)", "fn try_from<T>(i: usize)"),
        ("fn try_from(i: usize)", "fn try_from(&self, i: usize)"),
        ("fn try_from(i: usize)", "unsafe fn try_from(i: usize)"),
        (
            "fn try_from(i: usize)",
            "extern \"C\" fn try_from(i: usize)",
        ),
        (
            "fn try_from(i: usize)",
            "fn try_from((i, j): (usize, usize))",
        ),
        (
            "Result<Index, IndexError>",
            "Result<Index<u32>, IndexError>",
        ),
        (
            "Result<Index, IndexError>",
            "Result<Index, IndexError, usize>",
        ),
    ] {
        assert!(syntax(&IMPL.replace(before, after)).is_err(), "{after}");
    }
}

pub(super) fn definition(path: &str, local: bool) -> Value {
    json!({"path":path, "definition_identity":format!("definition:{path}"),
        "crate":if local {"sample"} else {"core"},
        "stable_crate_id":if local {"sample-build"} else {"core-build"}, "local":local})
}

fn argument(identity: Value) -> Value {
    json!({"kind":"type", "value":"display is not type authority", "display_only":true,
        "identity_status":"known", "identity":identity})
}

#[test]
fn substitution_identity_cannot_be_inferred_from_usize_display() {
    let baseline = argument(json!({"kind":"builtin", "name":"usize"}));
    let checked = |value: Value| -> Result<(), String> {
        let argument: model::Argument =
            serde_json::from_value(value).map_err(|error| error.to_string())?;
        super::input::structural(argument.known()?, "usize")
    };
    assert!(checked(baseline.clone()).is_ok());
    for (key, value) in [
        ("identity_status", json!("unknown")),
        ("display_only", json!(false)),
        ("kind", json!("const")),
        ("identity", json!({"kind":"builtin", "name":"u32"})),
    ] {
        let mut attack = baseline.clone();
        attack[key] = value;
        attack["value"] = json!("usize");
        assert!(checked(attack).is_err());
    }
}

pub(super) fn node() -> crate::HirNode {
    serde_json::from_value(json!({
        "hir":{"krate":0,"owner":4,"local":8},"span":{"start":10,"end":24},
        "kind":"expr_call","snippet":"Index::try_from(i)", "ty":null,
        "res":null,"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,
        "lit":null,"param_index":null,"is_unsafe_block":false,"children":[]
    }))
    .unwrap()
}

fn inventory() -> Value {
    json!({"identity":"rstyle.rust.call-instance-observations/1","complete":true,"calls":[{
        "hir":{"krate":0,"owner":4,"local":8},"span":{"start":10,"end":24},
        "expanded":false,"status":"resolved",
        "requested":definition("core::convert::TryFrom::try_from", false),
        "selected":definition("<Index as core::convert::TryFrom<usize>>::try_from", true),
        "requested_arguments":[argument(json!({"kind":"adt", "definition":definition("Index", true), "arguments":[]})),
            argument(json!({"kind":"builtin", "name":"usize"}))],
        "selected_arguments":[], "signature":{"display_only":true,"inputs":["usize"],"output":"Result<Index, IndexError>"}
    }]})
}

#[test]
fn raw_selection_requires_one_complete_exact_hir_and_span_pair() {
    let baseline = inventory();
    assert!(selection::original_call(&baseline, &node()).is_ok());
    let mut attacks = Vec::new();
    for (key, value) in [("complete", json!(false)), ("identity", json!("unknown"))] {
        let mut attack = baseline.clone();
        attack[key] = value;
        attacks.push(attack);
    }
    for (key, value) in [
        ("expanded", json!(true)),
        ("status", json!("unknown")),
        ("compiled_dependency", json!({})),
        ("hir", json!({"krate":0,"owner":5,"local":8})),
        ("span", json!({"start":11,"end":24})),
    ] {
        let mut attack = baseline.clone();
        attack["calls"][0][key] = value;
        attacks.push(attack);
    }
    let mut duplicate = baseline.clone();
    duplicate["calls"]
        .as_array_mut()
        .unwrap()
        .push(baseline["calls"][0].clone());
    attacks.extend([
        duplicate,
        json!({"identity":"rstyle.rust.call-instance-observations/1","complete":true,"calls":[]}),
    ]);
    for attack in attacks {
        assert!(selection::original_call(&attack, &node()).is_err());
    }
}

#[test]
fn raw_selection_rejects_unknown_identity_fields_instead_of_discarding_them() {
    let mut attack = inventory();
    attack["calls"][0]["selected"]["unknown_semantics"] = json!(true);
    assert!(selection::original_call(&attack, &node()).is_err());
}

pub(super) fn facts() -> crate::HirFnFacts {
    serde_json::from_str(include_str!(
        "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap()
}

#[test]
fn nominal_structural_identity_rejects_display_aliases_generics_and_missing_owners() {
    let nominal = crate::concrete_types::Nominal {
        source_identity: Some("sample-package".into()),
        package_name: "sample".into(),
        package_version: "1".into(),
        manifest_digest: "not-authority".into(),
        definition_path: "Index".into(),
        declaration_digest: "not-authority".into(),
    };
    let baseline =
        json!({"kind":"adt", "definition":definition("sample::Index", true), "arguments":[]});
    let checked = |raw: Value| -> Result<(), String> {
        let structural: model::Structural =
            serde_json::from_value(raw).map_err(|error| error.to_string())?;
        owner::nominal(&structural, &nominal).map(|_| ())
    };
    assert!(checked(baseline.clone()).is_ok());
    for (key, value) in [
        ("path", json!("sample::Other")),
        ("definition_identity", json!("")),
        ("stable_crate_id", json!("")),
        ("crate", json!("")),
        ("local", json!(false)),
    ] {
        let mut attack = baseline.clone();
        attack["definition"][key] = value;
        assert!(checked(attack).is_err());
    }
    let mut generic = baseline;
    generic["arguments"] = json!([{"kind":"builtin","name":"usize"}]);
    assert!(checked(generic).is_err());
}

#[test]
fn core_names_do_not_replace_mandatory_independent_authority() {
    use super::{CoreAuthority, CoreRole};
    struct Denied;
    impl CoreAuthority for Denied {
        fn verify(
            &self,
            _: &crate::HirFnFacts,
            _: CoreRole,
            _: &model::Definition,
        ) -> Result<(), String> {
            Err("independent authority denied".into())
        }
    }
    let definition: model::Definition =
        serde_json::from_value(definition("core::convert::TryFrom", false)).unwrap();
    assert_eq!(
        owner::core(&facts(), CoreRole::Trait, &definition, &Denied).unwrap_err(),
        "independent authority denied"
    );
    assert!(
        super::RegistryAuthority
            .verify(&facts(), CoreRole::Trait, &definition)
            .is_err()
    );
}

#[test]
fn unsigned_source_input_must_agree_with_trait_and_parameter() {
    for input in ["usize", "u16", "u32", "u64"] {
        let original = IMPL.replace("usize", input);
        let item: syn::ItemImpl = syn::parse_str(&original).unwrap();
        let syn::ImplItem::Fn(method) = &item.items[1] else {
            panic!("method")
        };
        syntax::implementation(&item, method, input).unwrap();
        for other in ["usize", "u16", "u32", "u64", "i32", "bool"] {
            if input != other {
                assert!(syntax::implementation(&item, method, other).is_err());
            }
        }
    }
}

#[test]
fn added_unsigned_inputs_require_their_registered_profile() {
    let mut facts = facts();
    facts.signature.params = vec![crate::ParamFact {
        symbol: "input".into(),
        position: 0,
        name_span: node().span,
        ty_span: node().span,
        ty: crate::ResolvedTy {
            normalized: Some("usize".into()),
            observed: "usize".into(),
            spelling: "usize".into(),
        },
        pat: node(),
    }];
    let mut historical = crate::extraction_profile::ExtractionProfile::DataV3.policy();
    historical.version = "3.116.0".into();
    assert!(!crate::extraction_profile::policy_supports(
        &historical,
        super::input::CAPABILITY
    ));
    for scalar in ["u16", "u32", "u64"] {
        facts.signature.params[0].ty.normalized = Some(scalar.into());
        facts
            .build
            .frontend_options
            .insert("extraction_profile".into(), json!("hir-data-v3"));
        assert_eq!(super::input::captured(&facts).unwrap(), scalar);
        facts
            .build
            .frontend_options
            .insert("extraction_profile".into(), json!("hir-data-v2"));
        assert!(super::input::captured(&facts).is_err());
    }
    for scalar in ["i64", "bool", "u8", "u128", "T", ""] {
        facts.signature.params[0].ty.normalized = Some(scalar.into());
        facts
            .build
            .frontend_options
            .insert("extraction_profile".into(), json!("hir-data-v3"));
        assert!(super::input::captured(&facts).is_err());
    }
}
