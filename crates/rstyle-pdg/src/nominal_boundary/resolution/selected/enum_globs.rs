//! Exact original `self::Enum::*` exports; no imported enum/module aliases.
use super::*;

pub(super) struct Original<'a> {
    pub namespace: &'a Namespace,
    pub key: &'a str,
    pub target: &'a Namespace,
}

pub(super) fn verify(
    facts: &HirFnFacts,
    environment: &Environment,
    original: Original<'_>,
) -> Result<bool, String> {
    if !supports(facts, concrete_types::EXPLICIT_U8_CAPABILITY)? {
        return Ok(false);
    }

    let Original {
        namespace,
        key,
        target,
    } = original;

    let Some(name) = target.last() else {
        return Ok(false);
    };
    let expected: Namespace = namespace.iter().cloned().chain([name.clone()]).collect();
    if target != &expected || key != format!("*::self::{name}") {
        return Ok(false);
    }

    let Some(declaration) = environment.index.declarations.get(target) else {
        return Ok(false);
    };

    let owner = &environment.modules[declaration.module_index];
    let syn::Item::Enum(item) = &owner.items[declaration.item_index] else {
        return Ok(false);
    };
    if owner.namespace != *namespace
        || environment
            .index
            .imports
            .contains_key(&(namespace.clone(), name.clone()))
        || environment
            .modules
            .iter()
            .any(|module| &module.namespace == target)
    {
        return Err("Enum glob owner is shadowed by an import or module".into());
    }

    original_import(facts, environment, namespace, name)?;
    exports(item)?;
    Ok(true)
}

fn original_import(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &Namespace,
    name: &str,
) -> Result<(), String> {
    let configuration = crate::conditional_source::Configuration::new(&facts.build.cfgs);
    let items = environment
        .modules
        .iter()
        .filter(|module| module.namespace == *namespace)
        .flat_map(|module| &module.items);

    crate::source_evidence::unique_active(
        items,
        |item| {
            if let syn::Item::Use(item) = item {
                Ok(configuration.active(&item.attrs)? && original(item, name))
            } else {
                Ok(false)
            }
        },
        "Enum glob lacks one exact active original self import",
    )
}

fn original(item: &syn::ItemUse, name: &str) -> bool {
    if item.leading_colon.is_some() {
        return false;
    }
    let syn::UseTree::Path(root) = &item.tree else {
        return false;
    };
    let syn::UseTree::Path(owner) = root.tree.as_ref() else {
        return false;
    };
    root.ident == "self"
        && owner.ident == name
        && matches!(owner.tree.as_ref(), syn::UseTree::Glob(_))
}

fn exports(item: &syn::ItemEnum) -> Result<(), String> {
    let variants = concrete_types::explicit_u8_fieldless_declaration(item)?;
    for (name, _) in variants {
        if imports::protected(&name)
            || matches!(
                name.as_str(),
                "Option" | "Result" | "Some" | "None" | "Ok" | "Err" | "Drop"
            )
        {
            return Err("Enum glob variant shadows a protected source binding".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment(source: &str) -> Result<Environment, String> {
        let mut census =
            (*crate::nominal_boundary::resolution::environment::tests::census()).clone();
        census.files[0].bytes = source.into();
        census.files[0].digest = concrete_types::digest(source.as_bytes());
        Environment::with_options(std::sync::Arc::new(census), true, Some(&[]), false)
    }

    fn witness(facts: &HirFnFacts, environment: &Environment) -> Result<bool, String> {
        verify(
            facts,
            environment,
            Original {
                namespace: &vec!["month".into()],
                key: "*::self::Month",
                target: &vec!["month".into(), "Month".into()],
            },
        )
    }

    #[test]
    fn lexical_witness_rejects_aliases_duplicates_and_historical_profiles() {
        let source = "mod month { #[repr(u8)] enum Month { #[expect(missing_docs)] January = 1 } use self::Month::*; }";
        let mut facts: HirFnFacts =
            serde_json::from_str(include_str!("function_cleanup_tests/facts.json")).unwrap();
        let positive = environment(source).unwrap();
        assert_eq!(witness(&facts, &positive), Ok(true));
        for replacement in [
            "use self::Other::*;",
            "use ::external::*;",
            "use self::Month::*; use self::Month::*;",
            "use self::Month::*; use unrelated::Other as Month;",
            "#[cfg(any())] use self::Month::*;",
        ] {
            let changed = source.replace("use self::Month::*;", replacement);
            assert!(
                environment(&changed)
                    .and_then(|environment| witness(&facts, &environment))
                    .is_err(),
                "{replacement}"
            );
        }
        facts
            .build
            .frontend_options
            .insert("extraction_profile".into(), "hir-data-v2".into());
        assert_eq!(witness(&facts, &positive), Ok(false));
    }

    #[test]
    fn exports_are_complete_values_and_keep_protected_names_closed() {
        let item = syn::parse_str(
            "#[repr(u8)] enum Month { #[expect(missing_docs)] January = 1, February = 2 }",
        )
        .unwrap();
        assert!(exports(&item).is_ok());
        for source in [
            "#[repr(u8)] enum E { Copy }",
            "#[repr(u8)] enum E { u8 }",
            "#[repr(u8)] enum E { core }",
            "#[repr(u8)] enum E { Some }",
            "#[repr(u8)] enum E { Drop }",
            "enum E { January }",
            "#[repr(u8)] enum E { #[cfg(any())] January, February }",
            "#[repr(u8)] enum E { January(u8) }",
        ] {
            assert!(
                exports(&syn::parse_str(source).unwrap()).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn only_original_self_enum_glob_supplies_a_witness() {
        assert!(original(
            &syn::parse_str("use self::Month::*;").unwrap(),
            "Month"
        ));
        for source in [
            "use ::self::Month::*;",
            "use Month::*;",
            "use crate::Month::*;",
            "use self::Other::*;",
            "use self::Month::{January, February};",
            "use self::Month::{*};",
            "use self::Month as Alias;",
        ] {
            assert!(
                !original(&syn::parse_str(source).unwrap(), "Month"),
                "{source}"
            );
        }
    }
}
