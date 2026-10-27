//! Independent source-semantic adversaries against the unchanged original member.
use super::*;
use syn::spanned::Spanned;
const ORIGINAL: &str = include_str!("../scalar_product_instances/proof/original-parsed-item.rs");

fn call() -> Value {
    serde_json::from_str(include_str!("original-call.json")).unwrap()
}

fn facts(source: &str) -> crate::HirFnFacts {
    let mut facts: crate::HirFnFacts = serde_json::from_str(include_str!(
        "../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    let file = syn::parse_file(source).unwrap();
    let method = file
        .items
        .iter()
        .find_map(|item| {
            let syn::Item::Impl(owner) = item else {
                return None;
            };
            owner.items.iter().find_map(|item| match item {
                syn::ImplItem::Fn(method) if method.sig.ident == "flat_map" => Some(method),
                _ => None,
            })
        })
        .unwrap();
    facts.fn_span = crate::ByteSpan {
        start: crate::source_evidence::original_span(source, method.vis.span()).start,
        end: crate::source_evidence::original_span(source, method.block.span()).end,
    };
    facts.source.bytes = source[facts.fn_span.start as usize..facts.fn_span.end as usize].into();
    facts
}

#[test]
fn original_complete_body_preserves_suffix_before_once_only_fallible_payload() {
    assert!(source::verify(ORIGINAL, &facts(ORIGINAL)).is_ok());
    for (before, after) in [
        (
            "Some(ParsedItem(self.0, f(self.1)?))",
            "Some(ParsedItem(self.0, f(self.1).unwrap()))",
        ),
        (
            "Some(ParsedItem(self.0, f(self.1)?))",
            "{ let value = f(self.1)?; Some(ParsedItem(self.0, value)) }",
        ),
        ("f(self.1)?", "f(self.0)?"),
        ("self.0, f(self.1)?", "&[], f(self.1)?"),
        ("f(self.1)?", "{ f(self.1); f(self.1)? }"),
        ("f(self.1)?", "unsafe { f(self.1)? }"),
        ("impl FnOnce(T) -> Option<U>", "impl FnMut(T) -> Option<U>"),
        ("fn flat_map<U>", "unsafe fn flat_map<U>"),
        (
            "impl<'a, T> ParsedItem<'a, T>",
            "impl<'a, T: Copy> ParsedItem<'a, T>",
        ),
        (
            "impl<'a, T> ParsedItem<'a, T>",
            "#[cfg(hidden)] impl<'a, T> ParsedItem<'a, T>",
        ),
        (
            "impl<'a, T> ParsedItem<'a, T>",
            "#[allow(unused)] impl<'a, T> ParsedItem<'a, T>",
        ),
        (
            "#[inline]\n    pub(crate) fn flat_map",
            "#[cfg(hidden)]\n    pub(crate) fn flat_map",
        ),
    ] {
        let changed = ORIGINAL.replace(before, after);
        assert_ne!(changed, ORIGINAL);
        assert!(
            source::verify(&changed, &facts(&changed)).is_err(),
            "{after}"
        );
    }
}

fn definitions(call: &Value) -> [Definition; 5] {
    let argument = &call["requested_arguments"][3]["identity"];
    [
        call["requested"].clone(),
        serde_json::json!({"crate":"time","stable_crate_id":call["requested"]["stable_crate_id"],"definition_identity":"product-source-id","path":"parsing::ParsedItem","local":true}),
        argument["signature"]["output"]["definition"].clone(),
        call["requested_arguments"][2]["identity"]["definition"].clone(),
        argument["definition"].clone(),
    ].map(|value| serde_json::from_value(value).unwrap())
}

fn check(call: &Value) -> Result<Argument, String> {
    let [member, product, option, nonzero, constructor] = definitions(&self::call());
    observation::verify(
        call,
        &Context {
            member: &member,
            product: &product,
            option: &option,
            nonzero: &nonzero,
            constructor: &constructor,
        },
    )
}

#[test]
fn complete_structural_substitutions_require_exact_function_item_and_option_result() {
    assert_eq!(check(&call()).unwrap(), Argument::U8);
    for (pointer, value) in [
        ("/status", Value::from("unsupported")),
        ("/expanded", Value::from(true)),
        ("/selected/definition_identity", Value::from("other")),
        (
            "/requested_arguments/1/identity_status",
            Value::from("unknown"),
        ),
        ("/requested_arguments/1/identity/name", Value::from("u16")),
        (
            "/requested_arguments/3/identity/kind",
            Value::from("function_pointer"),
        ),
        (
            "/selected_arguments/3/identity/signature/safety",
            Value::from("Unsafe"),
        ),
        (
            "/selected_arguments/3/identity/signature/bound_variables",
            Value::from(1),
        ),
        (
            "/selected_arguments/3/identity/signature/output/definition/path",
            Value::from("user::Option"),
        ),
    ] {
        let mut attack = call();
        *attack.pointer_mut(pointer).unwrap() = value;
        assert!(check(&attack).is_err(), "{pointer}");
    }
}

#[test]
fn display_strings_cannot_supply_identity_and_source_mismatch_fails() {
    let mut display = call();
    display["signature"] = Value::Null;
    display["requested_arguments"][3]["value"] = Value::from("arbitrary display");
    assert_eq!(check(&display).unwrap(), Argument::U8);
    let mut mismatch = facts(ORIGINAL);
    mismatch.source.bytes = "fn flat_map() {}".into();
    assert!(source::verify(ORIGINAL, &mismatch).is_err());
}

#[test]
fn consistent_but_wrong_scalar_callback_and_hidden_structural_fields_fail() {
    for (tail, value) in [
        ("/2/identity/arguments/0/name", Value::from("u16")),
        ("/3/identity/signature/inputs/0/name", Value::from("u16")),
        ("/3/identity/signature/variadic", Value::from(true)),
        ("/3/identity/signature/abi", Value::from("C")),
        (
            "/3/identity/signature/output/arguments/0/arguments/0/name",
            Value::from("u16"),
        ),
    ] {
        let mut attack = call();
        for prefix in ["/requested_arguments", "/selected_arguments"] {
            *attack.pointer_mut(&format!("{prefix}{tail}")).unwrap() = value.clone();
        }
        assert!(check(&attack).is_err(), "{tail}");
    }
    let mut hidden = call();
    for key in ["requested_arguments", "selected_arguments"] {
        hidden[key][3]["identity"]["hidden_effect"] = Value::from(true);
    }
    assert!(check(&hidden).is_err());
}

#[test]
fn checked_receipt_retains_both_bounded_arguments_and_order_without_copy_authority() {
    for argument in [Argument::U8, Argument::U16] {
        let original = include_str!("original-call.json");
        let call: Value = serde_json::from_str(&original.replace("u8", argument.as_str())).unwrap();
        let [member, product, option, nonzero, constructor] = definitions(&call);
        let context = Context {
            member: &member,
            product: &product,
            option: &option,
            nonzero: &nonzero,
            constructor: &constructor,
        };
        let receipt = verify(ORIGINAL, &facts(ORIGINAL), &call, &context).unwrap();
        assert_eq!(receipt.argument(), argument);
        assert_eq!(receipt.ordered_fields(), [0, 1]);
    }
}

fn owned_roster() -> (crate::HirFnFacts, crate::HirFnFacts) {
    let mut original = facts(ORIGINAL);
    original.name = "parsing::ParsedItem::flat_map".into();
    original.source.digest = original.source.bytes.checksum().into();
    let mut context = crate::nominal_boundary::tests::context();
    context.crate_name = "time".into();
    let bytes: crate::SourceSnapshot = ORIGINAL.into();
    let census = std::sync::Arc::new(crate::nominal_boundary::census::Census {
        identity: crate::nominal_boundary::census::IDENTITY.into(),
        context: context.clone(),
        files: vec![crate::nominal_boundary::census::UnitFile {
            path: context.root_source.clone(),
            digest: bytes.checksum().into(),
            bytes,
        }],
        expansions: vec![],
        documentation: vec![],
        external_imports: vec![],
    });
    let options = &mut original.build.frontend_options;
    options.insert_data(census);
    options.insert(
        crate::nominal_boundary::CONTEXT_KEY.into(),
        serde_json::to_value(&context).unwrap(),
    );
    options.insert(
        "source_file".into(),
        Value::from(context.root_source.to_str().unwrap()),
    );
    options.insert(
        "definition_identity".into(),
        call()["requested"]["definition_identity"].clone(),
    );
    let mut caller = original.clone();
    caller.name = "caller".into();
    caller
        .build
        .frontend_options
        .insert("definition_identity".into(), Value::from("caller-id"));
    caller.build.frontend_options.insert("rstyle.rust.call-instances".into(), serde_json::json!({
        "identity":"rstyle.rust.call-instance-observations/1", "complete":true,
        "callbacks":[{"scope":"immediate-core-option-and-then-callback-owning-body; nested owners excluded", "calls":[call()]}]
    }));
    (caller, original)
}

fn verify_owned(roster: Vec<crate::HirFnFacts>) -> Result<Checked, String> {
    let store = crate::unit_store::UnitStore::capture(roster)?;
    let analysis = store.analysis();
    let caller = analysis.function(0).ok_or("Missing fixture caller")?;
    let [member, product, option, nonzero, constructor] = definitions(&call());
    verify_in(
        &caller,
        &call(),
        &Context {
            member: &member,
            product: &product,
            option: &option,
            nonzero: &nonzero,
            constructor: &constructor,
        },
    )
}

#[test]
fn owning_lookup_requires_original_same_unit_member_source_and_separate_callback_scope() {
    let (caller, original) = owned_roster();
    assert_eq!(
        verify_owned(vec![caller.clone(), original.clone()])
            .unwrap()
            .argument(),
        Argument::U8
    );
    assert!(verify_owned(vec![caller.clone()]).is_err());
    assert!(verify_owned(vec![caller.clone(), original.clone(), original.clone()]).is_err());
    let mut changed = original.clone();
    changed.build.cfgs.push("changed_build".into());
    assert!(verify_owned(vec![caller.clone(), changed]).is_err());
    let mut changed = original.clone();
    changed.source.digest = "xxh3-64:0000000000000000".into();
    assert!(verify_owned(vec![caller.clone(), changed]).is_err());
    let mut changed = caller;
    changed.build.frontend_options.insert("rstyle.rust.call-instances".into(), serde_json::json!({
        "identity":"rstyle.rust.call-instance-observations/1", "complete":true, "calls":[call()], "callbacks":[]
    }));
    assert!(verify_owned(vec![changed, original]).is_err());
}

#[test]
fn public_source_receipt_requires_independent_nonzero_compiler_authority() {
    let (mut caller, mut original) = owned_roster();
    caller.build.rustc_commit.value = Some("unreviewed-compiler".into());
    original.build.rustc_commit = caller.build.rustc_commit.clone();
    assert!(verify_owned(vec![caller, original]).is_err());
}

#[test]
fn repeated_callback_occurrences_do_not_supply_unique_owning_call_evidence() {
    let (caller, original) = owned_roster();
    for separate_owners in [false, true] {
        let mut changed = caller.clone();
        let inventory = changed
            .build
            .frontend_options
            .get_mut("rstyle.rust.call-instances")
            .unwrap();
        if separate_owners {
            let callback = inventory["callbacks"][0].clone();
            inventory["callbacks"]
                .as_array_mut()
                .unwrap()
                .push(callback);
        } else {
            inventory["callbacks"][0]["calls"]
                .as_array_mut()
                .unwrap()
                .push(call());
        }
        assert!(verify_owned(vec![changed, original.clone()]).is_err());
    }
}

#[test]
fn conflicting_callback_metadata_at_same_hir_is_rejected() {
    let (mut caller, original) = owned_roster();
    let mut conflicting = call();
    conflicting["status"] = Value::from("unsupported");
    let inventory = caller
        .build
        .frontend_options
        .get_mut("rstyle.rust.call-instances")
        .unwrap();
    inventory["callbacks"][0]["calls"]
        .as_array_mut()
        .unwrap()
        .push(conflicting);
    assert!(verify_owned(vec![caller, original]).is_err());
}
