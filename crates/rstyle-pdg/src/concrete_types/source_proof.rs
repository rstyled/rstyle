//! Strict source correspondence, separate from permissive type observations.

use super::TypeTree;

/// Resolution must be scoped to the exact owning declaration and build. The
/// caller supplies independently verified evidence, never a name heuristic.
pub trait PathResolution {
    fn resolve(&self, path: &syn::TypePath) -> Result<TypeTree, String>;

    fn array_length(&self, source: &syn::Expr, expected: u64) -> Result<(), String> {
        literal_length(source, expected)
    }

    /// Default equality still requires independently scoped whole-path resolution.
    fn concrete_result(
        &self,
        path: &syn::TypePath,
        ok: &TypeTree,
        err: &TypeTree,
    ) -> Result<(), String> {
        let expected = TypeTree::Result {
            ok: Box::new(ok.clone()),
            err: Box::new(err.clone()),
        };
        resolved_type(
            self,
            path,
            expected,
            "Source path resolution disagrees with concrete Result",
        )
    }

    fn scalar_option(&self, path: &syn::TypePath, payload: &TypeTree) -> Result<(), String> {
        resolved_type(
            self,
            path,
            TypeTree::Option(Box::new(payload.clone())),
            "Source path resolution disagrees with concrete Option",
        )
    }
}

fn resolved_type(
    resolution: &(impl PathResolution + ?Sized),
    path: &syn::TypePath,
    expected: TypeTree,
    reason: &str,
) -> Result<(), String> {
    if resolution.resolve(path)? != expected {
        return Err(reason.into());
    }
    Ok(())
}

pub fn verify(tree: &TypeTree, resolution: &impl PathResolution) -> Result<(), String> {
    super::validate(tree)?;
    fields(tree, resolution)
}

fn fields(tree: &TypeTree, resolution: &impl PathResolution) -> Result<(), String> {
    match tree {
        TypeTree::ScalarProductInstance(_) => {
            return Err(
                "Scalar product instances require their scoped substitution source proof".into(),
            );
        }
        TypeTree::GeneratedStruct { .. } => {
            return Err(
                "Generated declarations require independently checked expansion source".into(),
            );
        }
        TypeTree::Struct { .. }
        | TypeTree::Enum { .. }
        | TypeTree::FieldlessEnum { .. }
        | TypeTree::MixedEnum { .. } => {
            for member in tree.nominal_fields() {
                let source = syn::parse_str(&member.declaration).map_err(|e| format!("{e}"))?;
                correspondence(&source, &member.ty, resolution)?;
                fields(&member.ty, resolution)?;
            }
        }
        TypeTree::Result { ok, err } => {
            fields(ok, resolution)?;
            fields(err, resolution)?;
        }
        TypeTree::Tuple(elements) => {
            for element in elements {
                fields(element, resolution)?;
            }
        }
        TypeTree::Option(element)
        | TypeTree::Shared(element)
        | TypeTree::Slice(element)
        | TypeTree::Array { element, .. } => {
            fields(element, resolution)?;
        }
        _ => {}
    }

    Ok(())
}

/// Prove one source-spelled type against its resolved, bounded type tree.
pub fn verify_spelling(
    source: &str,
    tree: &TypeTree,
    resolution: &impl PathResolution,
) -> Result<(), String> {
    verify_top_spelling(source, tree, resolution)?;
    fields(tree, resolution)
}

/// Verify one spelling without traversing nested nominal field declarations.
/// Their lexical owning scopes must be verified separately by the caller.
pub fn verify_top_spelling(
    source: &str,
    tree: &TypeTree,
    resolution: &impl PathResolution,
) -> Result<(), String> {
    if source.len() > super::MAX_SOURCE_BYTES {
        return Err("Source type proof exceeds source budget".into());
    }
    super::validate(tree)?;
    let parsed = syn::parse_str(source).map_err(|e| format!("{e}"))?;
    correspondence(&parsed, tree, resolution)
}

/// Internal checked-source consumers already bind this AST to original bytes.
/// This proves type correspondence only, never generated-source ownership.
pub(crate) fn verify_top_syntax(
    source: &syn::Type,
    tree: &TypeTree,
    resolution: &impl PathResolution,
) -> Result<(), String> {
    super::validate(tree)?;
    correspondence(source, tree, resolution)
}

fn correspondence(
    source: &syn::Type,
    tree: &TypeTree,
    resolution: &impl PathResolution,
) -> Result<(), String> {
    super::declarations::check(source, tree)?;
    match (source, tree) {
        (syn::Type::Paren(source), _) => correspondence(&source.elem, tree, resolution),
        (syn::Type::Group(source), _) => correspondence(&source.elem, tree, resolution),
        (syn::Type::Path(path), _) => path_proof(path, tree, resolution),
        (syn::Type::Tuple(source), TypeTree::Tuple(elements)) => {
            for (source, tree) in source.elems.iter().zip(elements) {
                correspondence(source, tree, resolution)?;
            }
            Ok(())
        }
        (syn::Type::Reference(source), TypeTree::MutableScalar(scalar))
            if source.mutability.is_some() && scalar == "usize" =>
        {
            correspondence(&source.elem, &TypeTree::Scalar(scalar.clone()), resolution)
        }
        (syn::Type::Reference(source), TypeTree::Shared(element)) => {
            correspondence(&source.elem, element, resolution)
        }
        (syn::Type::Slice(source), TypeTree::Slice(element)) => {
            correspondence(&source.elem, element, resolution)
        }
        (syn::Type::Array(source), TypeTree::Array { element, length }) => {
            resolution.array_length(&source.len, *length)?;
            correspondence(&source.elem, element, resolution)
        }
        (syn::Type::Tuple(source), TypeTree::Scalar(token))
            if source.elems.is_empty() && token == "unit" =>
        {
            Ok(())
        }
        _ => Err("Unsupported source type proof".into()),
    }
}

fn path_proof(
    path: &syn::TypePath,
    tree: &TypeTree,
    resolution: &impl PathResolution,
) -> Result<(), String> {
    if let TypeTree::Result { ok, err } = tree {
        let [ok_source, err_source] = super::result_arguments(path)?;
        correspondence(ok_source, ok, resolution)?;
        correspondence(err_source, err, resolution)?;
        return resolution.concrete_result(path, ok, err);
    }
    if matches!(
        tree,
        TypeTree::RangeToUsize { .. } | TypeTree::RangeFromUsize { .. }
    ) {
        return Err(
            "One-ended ranges are supported only as verified checked-access carriers".into(),
        );
    }
    if matches!(tree, TypeTree::RangeUsize { .. }) {
        return if resolution.resolve(path)? == *tree {
            Ok(())
        } else {
            Err("Source Range spelling disagrees with its bound observation".into())
        };
    }
    if let TypeTree::Option(payload) = tree
        && matches!(
            payload.as_ref(),
            TypeTree::Scalar(_) | TypeTree::Tuple(_) | TypeTree::Struct { .. }
        )
    {
        correspondence(
            super::option_spelling::payload(path, payload)?,
            payload,
            resolution,
        )?;
        return resolution.scalar_option(path, payload);
    } else {
        super::lifetimes::verify_path(path, tree)?;
    }

    if primitive_path(path, tree)? {
        return Ok(());
    }
    if resolution.resolve(path)? != *tree {
        return Err("Source path resolution disagrees with concrete type".into());
    }

    Ok(())
}

fn primitive_path(path: &syn::TypePath, tree: &TypeTree) -> Result<bool, String> {
    if !matches!(tree, TypeTree::Option(_))
        && path.path.leading_colon.is_none()
        && path.path.segments.len() == 1
    {
        let token = path.path.segments[0].ident.to_string();
        if matches!(token.as_str(), "char" | "f32" | "f64") {
            return if *tree == TypeTree::Scalar(token) {
                Ok(true)
            } else {
                Err("Standalone scalar spelling contradicts its resolved observation".into())
            };
        }
        if (super::scalar(&token) && token != "unit") || token == "str" {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) fn literal_length(source: &syn::Expr, expected: u64) -> Result<(), String> {
    if let syn::Expr::Lit(source) = source
        && let syn::Lit::Int(integer) = &source.lit
        && integer.base10_parse::<u64>().ok() == Some(expected)
    {
        return Ok(());
    }

    Err("Array source proof requires an exact literal length".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Missing;

    impl PathResolution for Missing {
        fn resolve(&self, _: &syn::TypePath) -> Result<TypeTree, String> {
            Err("Missing independently bound path".into())
        }
    }

    fn integer() -> TypeTree {
        TypeTree::Scalar("u32".into())
    }

    #[test]
    fn nested_literal_types_have_exact_source_proofs() {
        let tree = TypeTree::Shared(Box::new(TypeTree::Tuple(vec![
            integer(),
            TypeTree::Array {
                element: Box::new(integer()),
                length: 16,
            },
        ])));
        verify_spelling("&'a (u32, [u32; 0x10])", &tree, &Missing).unwrap();
        for source in [
            "&mut (u32, [u32; 16])",
            "&(i32, [u32; 16])",
            "&(u32, [u32; N])",
            "&(u32, [u32; 15])",
        ] {
            assert!(
                verify_spelling(source, &tree, &Missing).is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn named_paths_never_pass_without_resolution() {
        for source in ["Alias", "module::Alias", "::module::Alias", "Alias<u32>"] {
            assert!(
                verify_spelling(source, &integer(), &Missing).is_err(),
                "{source}"
            );
        }
        verify_spelling("u32", &integer(), &Missing).unwrap();
        assert!(verify_spelling("u16", &integer(), &Missing).is_err());
    }

    struct Resolved(TypeTree);

    impl PathResolution for Resolved {
        fn resolve(&self, _: &syn::TypePath) -> Result<TypeTree, String> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn supplied_resolution_must_match_the_entire_type() {
        verify_spelling("module::Alias", &integer(), &Resolved(integer())).unwrap();
        assert!(
            verify_spelling(
                "Alias",
                &integer(),
                &Resolved(TypeTree::Scalar("i32".into()))
            )
            .is_err()
        );
    }
}
