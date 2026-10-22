//! Exact lint-only module metadata. Path/expansion attributes remain unsupported.
pub(crate) fn deprecated_expect(attribute: &syn::Meta) -> bool {
    crate::source_evidence::single_identifier_attribute(attribute, "expect", "deprecated")
}

#[cfg(test)]
mod tests {
    use super::super::*;

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
        assert!(selected_attributes(&configuration, &positive.attrs, true, true).is_ok());
        assert!(selected_attributes(&configuration, &positive.attrs, true, false).is_err());
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
                selected_attributes(&configuration, &file.attrs, true, true).is_err(),
                "{attribute}"
            );
        }
    }
}
