//! Source-required comparison/Default identities; no implementation effects.
use super::*;

pub(in super::super) fn resolve(
    path: &syn::Path,
    scopes: &[BTreeMap<String, bool>],
    library: Option<&crate::data_library::Registry>,
) -> Option<ReferenceTrait> {
    if path.leading_colon.is_some()
        || scopes.iter().any(|scope| {
            scope.contains_key("*") || scope.contains_key("core") || scope.contains_key("std")
        })
    {
        return None;
    }
    let mut original = path.clone();
    let arguments = std::mem::take(&mut original.segments.last_mut()?.arguments);
    if !matches!(arguments, syn::PathArguments::None)
        && !matches!(&arguments, syn::PathArguments::AngleBracketed(args)
            if args.args.len() == 1 && matches!(args.args.first(), Some(syn::GenericArgument::Type(_))))
    {
        return None;
    }
    let resolved = source_imports::resolve(&original, scopes).or_else(|| {
        let segments = original.segments.iter().collect::<Vec<_>>();
        let [segment] = segments.as_slice() else {
            return None;
        };
        let name = segment.ident.to_string();
        let module = if name == "Default" { "default" } else { "cmp" };
        (!scopes.iter().any(|scope| scope.contains_key(&name)))
            .then(|| vec!["core".into(), module.into(), name])
    })?;
    let [root, module, name] = resolved.as_slice() else {
        return None;
    };
    let comparison = module == "cmp"
        && matches!(name.as_str(), "PartialEq" | "PartialOrd")
        && library_macros::core_cmp(&resolved[..2], name, library);
    // The already pinned whole core prelude directly reexports
    // crate::default::Default. A final type argument is never valid here.
    let default = module == "default"
        && name == "Default"
        && matches!(arguments, syn::PathArguments::None)
        && library.is_some_and(pinned);
    if root != "core" || !(comparison || default) {
        return None;
    }

    Some(ReferenceTrait { _private: () })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_prelude_and_imported_comparisons_keep_drop_and_argument_boundaries() {
        let library = Some(crate::data_library::registry());
        for name in ["PartialEq", "PartialOrd"] {
            for text in [
                name.to_owned(),
                format!("{name}<u8>"),
                format!("core::cmp::{name}<u8>"),
            ] {
                let path = syn::parse_str(&text).unwrap();
                assert!(resolve(&path, &[], library).is_some(), "{text}");
                assert!(resolve(&path, &[], None).is_none());
            }
            let mut scope = BTreeMap::from([(name.into(), false)]);
            let path = syn::parse_str(&format!("{name}<u8>")).unwrap();
            assert!(resolve(&path, &[scope.clone()], library).is_none());
            let import: syn::ItemUse = syn::parse_str(&format!("use core::cmp::{name};")).unwrap();
            source_imports::record(&import.tree, &mut scope);
            assert!(resolve(&path, &[scope.clone()], library).is_some());
            scope.insert("*".into(), false);
            assert!(resolve(&path, &[scope], library).is_none());
        }
        for text in [
            "Drop",
            "PartialEq<>",
            "PartialOrd<u8,u16>",
            "PartialEq<'a>",
            "PartialOrd<Item=u8>",
            "std::cmp::PartialOrd",
            "external::PartialEq",
        ] {
            assert!(
                resolve(&syn::parse_str(text).unwrap(), &[], library).is_none(),
                "{text}"
            );
        }
    }

    #[test]
    fn cross_module_comparison_imports_cannot_hide_drop_or_shadowed_names() {
        let positive = "struct Error(()); mod sibling { use core::cmp::PartialEq; } struct External; impl PartialEq<Error> for External {} impl PartialOrd<Error> for External {}";
        assert!(closure::inspect_external_source(positive, true).is_ok());
        assert!(closure::inspect_external_source(positive, false).is_err());
        for extra in [
            "impl Drop for Error { fn drop(&mut self) {} }",
            "use core::ops::Drop as PartialEq;",
            "use core::ops::Drop as PartialOrd;",
            "macro_rules! hidden { () => { impl Drop for Error { fn drop(&mut self) {} } }; } hidden!();",
            "fn nested() { impl Drop for Error { fn drop(&mut self) {} } }",
        ] {
            assert!(
                closure::inspect_external_source(&format!("{positive} {extra}"), true).is_err(),
                "{extra}"
            );
        }
    }

    #[test]
    fn default_header_requires_exact_prelude_identity_and_no_type_arguments() {
        let library = Some(crate::data_library::registry());
        for text in ["Default", "core::default::Default"] {
            let path = syn::parse_str(text).unwrap();
            assert!(resolve(&path, &[], library).is_some());
            assert!(resolve(&path, &[], None).is_none());
            assert!(resolve(&path, &[BTreeMap::from([("core".into(), false)])], library).is_none());
        }
        let source = "struct Error(()); mod generated { macro_rules! define { () => { struct Value; }; } define!(); macro_rules! implement { () => { impl Default for Value {} }; } implement!(); }";
        assert!(closure::inspect_external_source(source, true).is_ok());
        assert!(closure::inspect_external_source(source, false).is_err());
        for extra in [
            "use core::ops::Drop as Default;",
            "impl Drop for Error { fn drop(&mut self) {} }",
        ] {
            assert!(closure::inspect_external_source(&format!("{source} {extra}"), true).is_err());
        }
        for text in ["Default<u8>", "core::cmp::Default", "external::Default"] {
            assert!(resolve(&syn::parse_str(text).unwrap(), &[], library).is_none());
        }
    }

    #[test]
    fn checked_from_identity_keeps_macro_self_and_implementation_contents_visible() {
        let source = "struct Error(()); macro_rules! owner { () => { Error }; } impl From<u8> for owner!() {}";
        assert!(closure::inspect_external_source(source, true).is_ok());
        for changed in [
            source.replace(
                "{ Error }",
                "{ { impl Drop for Error { fn drop(&mut self) {} } Error } }",
            ),
            format!("use core::ops::Drop as From; {source}"),
            source.replace("owner!()", "unknown!()"),
            source.replace("impl From<u8>", "impl Drop"),
            source.replace(
                "for owner!() {}",
                "for owner!() { fn method() { impl Drop for Error { fn drop(&mut self) {} } } }",
            ),
        ] {
            assert!(
                closure::inspect_external_source(&changed, true).is_err(),
                "{changed}"
            );
        }
    }
}
