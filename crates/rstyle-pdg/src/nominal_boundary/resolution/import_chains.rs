//! Follow exact local nominal reexports in the independently closed lexical index.
use super::*;

pub(super) fn resolve(mut names: Namespace, index: &Index) -> Result<Namespace, String> {
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..crate::concrete_types::MAX_DEPTH {
        if !seen.insert(names.clone()) {
            return Err("Cyclic local nominal import chain".into());
        }
        // Imports can bind a value with a module's name. Do not reject that
        // unrelated namespace, but require unambiguous type/module ownership
        // for every prefix actually used by this selected nominal chain.
        if (1..names.len()).any(|length| index.ambiguous_module_imports.contains(&names[..length]))
        {
            return Err("Local nominal path crosses an ambiguous module import".into());
        }
        if index.declarations.contains_key(&names) {
            return Ok(names);
        }
        let (name, namespace) = names.split_last().ok_or("Empty local nominal import")?;
        names = index
            .imports
            .get(&(namespace.to_vec(), name.clone()))
            .cloned()
            .ok_or("Local nominal import lacks an exact declaration or reexport")?;
    }
    Err("Local nominal import exceeds the bounded alias depth".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment(source: &str) -> Result<Environment, String> {
        let mut census = (*super::super::environment::tests::census()).clone();
        census.files[0].bytes = source.into();
        census.files[0].digest = concrete_types::digest(source.as_bytes());
        Environment::with_options(Arc::new(census), false, Some(&[]), false)
    }

    #[test]
    fn original_local_reexport_chain_reaches_the_complete_declaration() {
        let env = environment("mod month { pub enum Month { January } } pub use crate::month::Month; mod util { use crate::Month as Calendar; }").unwrap();
        assert_eq!(
            resolve(vec!["util".into(), "Calendar".into()], &env.index).unwrap(),
            vec!["month", "Month"]
        );
    }

    #[test]
    fn cycles_missing_targets_and_unresolved_external_names_do_not_supply_a_type() {
        for source in [
            "use crate::B as A; use crate::A as B;",
            "use crate::missing::Month as A;",
            "use external::Month as A;",
        ] {
            let env = environment(source).unwrap();
            assert!(resolve(vec!["A".into()], &env.index).is_err(), "{source}");
        }
        assert!(environment("enum A { X } use crate::other::A;").is_err());
    }
    #[test]
    fn loaded_child_reexports_use_the_lexical_module_not_a_root_namesake() {
        let env = environment("#[cfg(all())] mod selected_marker {} mod child { pub struct T; } mod api { mod child { pub struct T; } pub use child::T as Local; pub use crate::child::T as Root; pub use self::child::T as Explicit; }").unwrap();
        for (alias, expected) in [
            ("Local", vec!["api", "child", "T"]),
            ("Explicit", vec!["api", "child", "T"]),
            ("Root", vec!["child", "T"]),
        ] {
            assert_eq!(
                resolve(vec!["api".into(), alias.into()], &env.index).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn missing_external_and_inactive_modules_do_not_supply_a_lexical_declaration() {
        for source in [
            "mod api { pub use absent::T; }",
            "mod api { pub use ::external::T; }",
            "mod api { #[cfg(any())] mod child { pub struct T; } pub use child::T; }",
        ] {
            let env = environment(source).unwrap();
            assert!(
                resolve(vec!["api".into(), "T".into()], &env.index).is_err(),
                "{source}"
            );
        }
        let env = environment("#[cfg(all())] mod selected_marker {} mod api { mod child { pub struct T; } use crate::other as child; pub use child::T; }").unwrap();
        assert!(resolve(vec!["api".into(), "T".into()], &env.index).is_err());
    }
}
