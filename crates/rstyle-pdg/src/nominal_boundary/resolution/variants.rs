//! Resolve variant imports through the captured local namespace, including
//! nominal reexports in the middle of a path. Compiler variant identity and
//! checked Copy declaration authority are independent requirements.
use super::*;

impl Resolution {
    pub(crate) fn verify_fieldless_variant_source(
        &self,
        path: &syn::Path,
        nominal: &concrete_types::Nominal,
        variant: &str,
    ) -> Result<(), String> {
        if path.leading_colon.is_some()
            || path.segments.is_empty()
            || path
                .segments
                .iter()
                .any(|part| !matches!(part.arguments, syn::PathArguments::None))
        {
            return Err("Variant source requires an ordinary local lexical path".into());
        }
        let owner: Namespace = nominal
            .definition_path
            .split("::")
            .map(str::to_owned)
            .collect();
        self.check_nominal(&owner, nominal)?;

        let expected: Namespace = owner.iter().cloned().chain([variant.into()]).collect();
        let names = path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect();
        let names = anchored(names, &self.namespace, false)?;
        if expand(names, &self.environment.index)? != expected {
            return Err("Variant import disagrees with the complete local enum owner".into());
        }
        Ok(())
    }
}

fn expand(mut names: Namespace, index: &Index) -> Result<Namespace, String> {
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..concrete_types::MAX_DEPTH {
        if !seen.insert(names.clone()) {
            return Err("Cyclic variant import chain".into());
        }
        let replacement = (1..=names.len()).find_map(|end| {
            index
                .imports
                .get(&(names[..end - 1].to_vec(), names[end - 1].clone()))
                .map(|target| (end, target))
        });
        let Some((end, target)) = replacement else {
            return Ok(names);
        };
        names = target
            .iter()
            .cloned()
            .chain(names[end..].iter().cloned())
            .collect();
    }
    Err("Variant import exceeds the bounded namespace depth".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_variant_follows_intermediate_enum_reexport_and_rejects_cycles() {
        let mut index = Index {
            unshadowed_u32: BTreeMap::new(),
            declarations: BTreeMap::new(),
            imports: BTreeMap::new(),
            unresolved_expansions: std::collections::BTreeSet::new(),
        };
        index.imports.insert(
            (vec!["decoder".into()], "InvalidLength".into()),
            vec!["Error".into(), "InvalidLength".into()],
        );
        index.imports.insert(
            (vec![], "Error".into()),
            vec!["errors".into(), "Error".into()],
        );
        let input = vec!["decoder".into(), "InvalidLength".into()];
        assert_eq!(
            expand(input.clone(), &index).unwrap(),
            ["errors", "Error", "InvalidLength"]
        );
        index.imports.insert(
            (vec!["errors".into()], "Error".into()),
            vec!["Error".into()],
        );
        assert!(expand(input, &index).is_err());
    }
}
