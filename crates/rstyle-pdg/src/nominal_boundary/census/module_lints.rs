//! Exact lint-only module metadata. Path/expansion attributes remain unsupported.
#[derive(Default)]
pub(super) struct Policy {
    pub deprecated: bool,
    pub expectations: bool,
}

impl Policy {
    pub(super) fn accepts(&self, attribute: &syn::Meta) -> bool {
        self.deprecated && deprecated_expect(attribute)
            || self.expectations && expectation(attribute)
    }
}

pub(crate) fn deprecated_expect(attribute: &syn::Meta) -> bool {
    crate::source_evidence::single_identifier_attribute(attribute, "expect", "deprecated")
}

/// Parse lint-only expectations without interpreting or discarding source items.
/// Capability activation belongs to the caller; the historical rule stays exact.
pub(crate) fn expectation(attribute: &syn::Meta) -> bool {
    let syn::Meta::List(attribute) = attribute else {
        return false;
    };
    if !attribute.path.is_ident("expect") {
        return false;
    }
    let Ok(operands) = attribute.parse_args_with(
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
    ) else {
        return false;
    };
    let mut lints = 0;

    for (position, operand) in operands.iter().enumerate() {
        match operand {
            syn::Meta::Path(path)
                if path.leading_colon.is_none()
                    && !path.segments.is_empty()
                    && path
                        .segments
                        .iter()
                        .all(|segment| segment.arguments.is_empty()) =>
            {
                lints += 1;
            }
            syn::Meta::NameValue(value)
                if value.path.is_ident("reason")
                    && position + 1 == operands.len()
                    && matches!(&value.value, syn::Expr::Lit(literal) if literal.attrs.is_empty() && matches!(literal.lit, syn::Lit::Str(_))) =>
                {}
            _ => return false,
        }
    }
    lints > 0
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::Policy;

    #[test]
    fn plain_lint_expectations_allow_tool_paths_and_a_final_literal_reason() {
        for source in [
            "expect(dead_code)",
            "expect(dead_code, unused_variables,)",
            "expect(clippy::needless_return)",
            "expect(dead_code, clippy::needless_return, reason = \"reviewed\",)",
            "expect(dead_code, reason = \"may be used in the future and has minimal overhead\")",
        ] {
            let meta: syn::Meta = syn::parse_str(source).unwrap();
            assert!(super::expectation(&meta), "{source}");
        }
    }

    #[test]
    fn expectation_metadata_cannot_hide_non_lint_syntax_or_evaluated_reasons() {
        for source in [
            "expect",
            "expect()",
            "expect = \"dead_code\"",
            "custom::expect(dead_code)",
            "::expect(dead_code)",
            "allow(dead_code)",
            "expect(::dead_code)",
            "expect(reason = \"alone\")",
            "expect(reason = \"first\", dead_code)",
            "expect(dead_code, reason = \"one\", reason = \"two\")",
            "expect(dead_code, reason = \"middle\", unused_variables)",
            "expect(dead_code = \"value\")",
            "expect(dead_code, tool::reason = \"qualified\")",
            "expect(dead_code, reason = 1)",
            "expect(dead_code, reason = b\"bytes\")",
            "expect(dead_code, reason = concat!(\"evaluated\"))",
            "expect(dead_code, reason = REASON)",
            "expect(dead_code, reason = { \"block\" })",
            "expect(dead_code, reason = (\"parenthesized\"))",
            "expect(dead_code(extra))",
            "expect(\"literal\")",
            "expect(some_macro!())",
        ] {
            let meta: syn::Meta = syn::parse_str(source).unwrap();
            assert!(!super::expectation(&meta), "{source}");
        }
    }

    fn sample(root: &str) -> Census {
        Census {
            identity: IDENTITY.into(),
            context: crate::nominal_boundary::tests::context(),
            expansions: Vec::new(), documentation: Vec::new(), external_imports: Vec::new(),
            files: [("/sample/src/lib.rs", root), ("/sample/src/peer.rs", "#![expect(deprecated)] struct Data; impl Drop for Data { fn drop(&mut self) {} } unknown!();")]
                .into_iter().map(|(path, bytes)| UnitFile {
                    path: path.into(), bytes: bytes.into(), digest: digest(bytes.as_bytes()),
                }).collect(),
        }
    }

    fn hint_sample(root: &str, metadata: &str) -> Census {
        let mut census = sample(root);
        let source = format!(
            "#![{metadata}] struct Data; impl Drop for Data {{ fn drop(&mut self) {{}} }} unknown!();"
        );
        census.files[1].digest = digest(source.as_bytes());
        census.files[1].bytes = source.into();
        census
    }

    #[test]
    fn hint_expectation_keeps_drop_and_unknown_macro_neighbors_in_complete_source() {
        let census = hint_sample(
            "mod peer;",
            "expect(dead_code, reason = \"may be used in the future and has minimal overhead\")",
        );
        let modules = verify_generated_namespace(&census, &[], false).unwrap();
        assert_eq!(modules.len(), 2);
        let peer = modules
            .iter()
            .find(|module| module.namespace == ["peer"])
            .unwrap();
        assert_eq!(peer.items.len(), 3);
        assert!(matches!(&peer.items[0], syn::Item::Struct(item) if item.ident == "Data"));
        assert!(matches!(&peer.items[1], syn::Item::Impl(item)
            if item.trait_.as_ref().is_some_and(|(_, path, _)| path.is_ident("Drop"))));
        assert!(
            matches!(&peer.items[2], syn::Item::Macro(item) if item.mac.path.is_ident("unknown"))
        );
        assert!(verify_with_module_lints(&census, &[], false).is_err());
        assert!(verify_selected(&census, &[]).is_err());
    }

    #[test]
    fn generated_namespace_expectation_does_not_grant_path_or_expression_authority() {
        for metadata in [
            "expect(dead_code, reason = concat!(\"evaluated\"))",
            "expect(dead_code, reason = \"one\", reason = \"two\")",
            "expect(reason = \"first\", dead_code)",
            "path = \"other.rs\"",
            "cfg_attr(unix, path = \"other.rs\")",
        ] {
            let census = hint_sample("mod peer;", metadata);
            assert!(
                verify_generated_namespace(&census, &["unix".into()], false).is_err(),
                "{metadata}"
            );
        }
    }

    #[test]
    fn lint_expectations_cannot_hide_missing_active_or_orphan_inactive_files() {
        let census = hint_sample(
            "mod peer; #[cfg(feature=\"hidden\")] mod hidden;",
            "expect(dead_code, reason = \"retained\")",
        );
        assert!(verify_generated_namespace(&census, &[], false).is_ok());
        assert!(
            verify_generated_namespace(&census, &["feature=\"hidden\"".into()], false).is_err()
        );
        let mut orphan = census;
        orphan.files.push(UnitFile {
            path: "/sample/src/hidden.rs".into(),
            bytes: "struct Hidden;".into(),
            digest: digest(b"struct Hidden;"),
        });
        assert!(verify_generated_namespace(&orphan, &[], false).is_err());
    }

    #[test]
    fn lint_closure_keeps_neighbors_and_rejects_active_path_omission() {
        let source = "mod peer; #[cfg(feature=\"offset\")] #[path=\"other.rs\"] mod hidden;";
        let proof = sample(source);
        let modules = verify_with_module_lints(&proof, &[], false).unwrap();
        let peer = modules
            .iter()
            .find(|module| module.namespace == ["peer"])
            .unwrap();
        assert_eq!(peer.items.len(), 3);
        assert!(matches!(&peer.items[1], syn::Item::Impl(item) if item.trait_.is_some()));
        assert!(matches!(&peer.items[2], syn::Item::Macro(_)));
        assert!(verify_selected(&proof, &[]).is_err());
        assert!(verify_with_module_lints(&proof, &["feature=\"offset\"".into()], false).is_err());
        let mut orphan = proof;
        orphan.files.push(UnitFile {
            path: "/sample/src/other.rs".into(),
            bytes: "".into(),
            digest: digest(b""),
        });
        assert!(verify_with_module_lints(&orphan, &[], false).is_err());
    }

    #[test]
    fn deprecated_expectation_is_exact_and_does_not_authorize_paths() {
        let configuration = crate::conditional_source::Configuration::new(&[]);
        let positive: syn::File = syn::parse_str("#![expect(deprecated)] fn f() {}").unwrap();
        assert!(
            selected_attributes(
                &configuration,
                &positive.attrs,
                true,
                Policy {
                    deprecated: true,
                    expectations: false
                }
            )
            .is_ok()
        );
        assert!(
            selected_attributes(&configuration, &positive.attrs, true, Policy::default()).is_err()
        );
        for attribute in [
            "expect",
            "expect()",
            "expect = \"deprecated\"",
            "custom::expect(deprecated)",
            "expect(deprecated, reason = \"x\")",
            "expect(other)",
            "expect(deprecated, other)",
            "path = \"other.rs\"",
            "cfg_attr(unix, path = \"other.rs\")",
        ] {
            let source = format!("#![{attribute}] fn f() {{}}");
            let file: syn::File = syn::parse_str(&source).unwrap();
            let cfgs = vec!["unix".into()];
            let configuration = crate::conditional_source::Configuration::new(&cfgs);
            assert!(
                selected_attributes(
                    &configuration,
                    &file.attrs,
                    true,
                    Policy {
                        deprecated: true,
                        expectations: false
                    }
                )
                .is_err(),
                "{attribute}"
            );
        }
    }
}
