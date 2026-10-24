//! Structural source checks only. The enclosing Option path still requires
//! independently scoped resolution; its name supplies no core authority.

use super::TypeTree;

pub(super) fn payload<'a>(
    path: &'a syn::TypePath,
    tree: &TypeTree,
) -> Result<&'a syn::Type, String> {
    if !matches!(tree, TypeTree::Scalar(token) if super::scalar(token) && token != "unit")
        && !matches!(
            tree,
            TypeTree::Tuple(_) | TypeTree::Struct { .. } | TypeTree::ScalarProductInstance(_)
        )
    {
        return Err("Option source construction requires one bool/integer payload".into());
    }
    type_arguments(path, 1)?
        .first()
        .copied()
        .ok_or_else(|| "Option source requires a type argument".into())
}

/// Syntax alone provides no authority for the Result path or either payload.
pub(crate) fn result_arguments(path: &syn::TypePath) -> Result<[&syn::Type; 2], String> {
    let arguments = type_arguments(path, 2)?;
    Ok([arguments[0], arguments[1]])
}

fn type_arguments(path: &syn::TypePath, count: usize) -> Result<Vec<&syn::Type>, String> {
    if path.qself.is_some()
        || path.path.segments.is_empty()
        || path
            .path
            .segments
            .iter()
            .take(path.path.segments.len().saturating_sub(1))
            .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
    {
        return Err("Core sum source requires an unqualified-self type path".into());
    }
    let last = path
        .path
        .segments
        .last()
        .ok_or("Empty core sum type path")?;
    let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
        return Err("Core sum source requires explicit type arguments".into());
    };
    if arguments.args.len() != count || arguments.colon2_token.is_some() {
        return Err("Core sum source type argument count or turbofish disagrees".into());
    }

    arguments
        .args
        .iter()
        .map(|argument| match argument {
            syn::GenericArgument::Type(ty) => Ok(ty),
            _ => Err("Core sum source requires only type arguments".into()),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::concrete_types::source_proof::{PathResolution, verify_top_spelling};

    struct Scope {
        token: &'static str,
    }

    impl PathResolution for Scope {
        fn resolve(&self, path: &syn::TypePath) -> Result<TypeTree, String> {
            let names: Vec<_> = path
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect();
            if names == ["Option"]
                || names == ["core", "option", "Option"]
                || names == ["std", "option", "Option"]
            {
                Ok(option(self.token))
            } else {
                Err("Test scope has no authority for this binding".into())
            }
        }
    }

    fn option(token: &str) -> TypeTree {
        TypeTree::Option(Box::new(TypeTree::Scalar(token.into())))
    }

    #[test]
    fn explicit_scalar_arguments_preserve_the_complete_resolved_option() {
        for token in [
            "bool", "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128",
            "isize",
        ] {
            for prefix in ["Option", "core::option::Option", "::std::option::Option"] {
                let source = format!("{prefix}<{token}>");
                verify_top_spelling(&source, &option(token), &Scope { token }).unwrap();
            }
        }
    }

    #[test]
    fn neither_argument_shape_nor_a_familiar_name_supplies_authority() {
        let scope = Scope { token: "usize" };
        for source in [
            "Option",
            "Option<>",
            "Option<u8>",
            "Option<usize, usize>",
            "Option<'static>",
            "Option<8>",
            "Option<Item = usize>",
            "Option<&usize>",
            "Option<()>",
            "Option<Option<usize>>",
            "Option<ScalarAlias>",
            "module::<usize>::Option<usize>",
            "<Owner as Trait>::Option<usize>",
            "Option::<usize>",
            "user::Option<usize>",
            "Renamed<usize>",
            "usize<usize>",
        ] {
            assert!(
                verify_top_spelling(source, &option("usize"), &scope).is_err(),
                "{source}"
            );
        }
        // A valid argument cannot compensate for the wrong resolved payload.
        assert!(
            verify_top_spelling("Option<usize>", &option("usize"), &Scope { token: "u8" }).is_err()
        );
    }

    struct Missing;

    impl PathResolution for Missing {
        fn resolve(&self, _: &syn::TypePath) -> Result<TypeTree, String> {
            Err("No lexical authority".into())
        }
    }

    #[test]
    fn structural_option_spelling_never_bypasses_independent_resolution() {
        for source in [
            "Option<usize>",
            "core::option::Option<usize>",
            "::std::option::Option<usize>",
        ] {
            assert!(verify_top_spelling(source, &option("usize"), &Missing).is_err());
        }
    }
}
