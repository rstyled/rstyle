//! Bounded selected deprecation metadata; no expansion or runtime permission.
use syn::{Token, punctuated::Punctuated};

pub(super) fn deprecated(attribute: &syn::Meta) -> bool {
    let syn::Meta::List(list) = attribute else {
        return false;
    };
    if !list.path.is_ident("deprecated") {
        return false;
    }

    let Ok(arguments) =
        list.parse_args_with(Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated)
    else {
        return false;
    };
    if arguments.len() != 2 {
        return false;
    }

    let mut keys = std::collections::BTreeSet::new();
    for argument in arguments {
        let Some(name) = argument.path.get_ident() else {
            return false;
        };
        if !matches!(name.to_string().as_str(), "since" | "note") || !keys.insert(name.to_string())
        {
            return false;
        }
        let syn::Expr::Lit(syn::ExprLit {
            attrs,
            lit: syn::Lit::Str(value),
        }) = argument.value
        else {
            return false;
        };
        if !attrs.is_empty() || value.value().is_empty() || value.value().len() > 512 {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_function_and_import_metadata_keep_historical_policies_closed() {
        let mut facts: crate::HirFnFacts =
            serde_json::from_str(include_str!("function_cleanup_tests/facts.json")).unwrap();
        let function: syn::ItemFn =
            syn::parse_str("#[deprecated(since=\"0.3.37\",note=\"use another helper\")] fn f() {}")
                .unwrap();
        let import: syn::Item =
            syn::parse_str("#[expect(deprecated)] use crate::Instant;").unwrap();
        let owner = crate::nominal_boundary::census::Module {
            path: "/sample/src/lib.rs".into(),
            namespace: Vec::new(),
            items: vec![import.clone()],
            verified_macro_imports: Default::default(),
            verified_external_imports: Default::default(),
        };
        let configuration = crate::conditional_source::Configuration::new(&[]);
        assert!(super::super::function_attributes(&facts, &function.attrs).is_ok());
        assert!(super::super::module_item_for(&facts, &import, &owner, &configuration).is_ok());
        for attribute in ["expect(other)", "path=\"other.rs\"", "custom"] {
            let item = syn::parse_str(&format!("#[{attribute}] use crate::Instant;")).unwrap();
            assert!(
                super::super::module_item_for(&facts, &item, &owner, &configuration).is_err(),
                "{attribute}"
            );
        }
        facts
            .build
            .frontend_options
            .insert("extraction_profile".into(), "hir-data-v2".into());
        assert!(super::super::function_attributes(&facts, &function.attrs).is_err());
        assert!(super::super::module_item_for(&facts, &import, &owner, &configuration).is_err());
    }

    #[test]
    fn exact_since_and_note_are_lint_metadata_only() {
        let source: syn::ItemFn = syn::parse_str(
            "#[deprecated(since=\"0.3.37\", note=\"use another helper\")] fn f() {}",
        )
        .unwrap();
        assert!(deprecated(&source.attrs[0].meta));
        for attribute in [
            "deprecated",
            "deprecated()",
            "custom::deprecated(since=\"1\",note=\"x\")",
            "deprecated(since=\"1\",since=\"2\")",
            "deprecated(since=\"1\",other=\"x\")",
            "deprecated(since=1,note=\"x\")",
            "deprecated(since=\"1\",note=concat!(\"x\"))",
            "deprecated(since=\"1\",note=\"\")",
            "deprecated = \"x\"",
        ] {
            let item: syn::ItemFn =
                syn::parse_str(&format!("#[{attribute}] fn f() {{}} ")).unwrap();
            assert!(!deprecated(&item.attrs[0].meta), "{attribute}");
        }
    }
}
