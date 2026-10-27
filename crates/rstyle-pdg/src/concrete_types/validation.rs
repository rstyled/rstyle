//! Reuse exact nominal shape checks within one bounded observation inventory.
//! Every occurrence still counts toward depth and entry limits.
use super::*;
use std::collections::BTreeMap;

#[derive(Default)]
struct Validation<'a> {
    entries: usize,
    shapes: BTreeMap<&'a Nominal, Vec<&'a TypeTree>>,
}

pub(super) fn tree(tree: &TypeTree) -> Result<(), String> {
    Validation::default().check(tree, 0)
}

pub(super) fn observations<'a>(
    observations: impl Iterator<Item = &'a Observation>,
) -> Result<(), String> {
    let mut validation = Validation::default();
    for observation in observations {
        match observation {
            Observation::Captured { ty } => validation.check(ty, 0)?,
            Observation::Unsupported { reason } if reason.is_empty() => {
                return Err("Unsupported concrete type lacks a reason".into());
            }
            Observation::Unsupported { .. } => {}
        }
    }

    Ok(())
}

impl<'a> Validation<'a> {
    fn check(&mut self, tree: &'a TypeTree, depth: usize) -> Result<(), String> {
        self.entries += 1;
        if depth > MAX_DEPTH || self.entries > MAX_ENTRIES {
            return Err("Concrete type observation exceeds depth/entry budget".into());
        }
        if depth != 0 && matches!(tree, TypeTree::MutableScalar(_)) {
            return Err("Mutable scalar descriptors require a complete root observation".into());
        }
        if depth != 0
            && matches!(
                tree,
                TypeTree::RangeUsize { .. }
                    | TypeTree::RangeToUsize { .. }
                    | TypeTree::RangeFromUsize { .. }
            )
        {
            return Err("Range values require a complete root result observation".into());
        }
        let root_scalar = depth == 0
            && matches!(tree, TypeTree::Scalar(token) if matches!(token.as_str(), "char" | "f32" | "f64"));
        let fresh = !root_scalar && self.shape(tree)?;
        self.children(tree, depth + 1)?;
        if fresh && let Some((nominal, _)) = tree.nominal_parts() {
            self.shapes.entry(nominal).or_default().push(tree);
        }

        Ok(())
    }

    fn shape(&self, tree: &'a TypeTree) -> Result<bool, String> {
        if let Some((nominal, _)) = tree.nominal_parts()
            && self
                .shapes
                .get(nominal)
                .is_some_and(|shapes| shapes.contains(&tree))
        {
            return Ok(false);
        }
        if tree.nominal_parts().is_none() || !crate::data_types::has_checked_shape(tree) {
            check_shape(tree)?;
        }

        Ok(tree.nominal_parts().is_some())
    }

    fn children(&mut self, tree: &'a TypeTree, depth: usize) -> Result<(), String> {
        if tree.nominal_identity().is_some() {
            return self.many(tree.nominal_fields().map(|field| &field.ty), depth);
        }
        match tree {
            TypeTree::Tuple(_) | TypeTree::Result { .. } => self.many(tree.components(), depth),
            TypeTree::Array { element, .. }
            | TypeTree::Shared(element)
            | TypeTree::Slice(element)
            | TypeTree::Option(element)
            | TypeTree::MappedIntConversionResult { ok: element } => self.check(element, depth),
            _ => Ok(()),
        }
    }

    fn many(
        &mut self,
        types: impl Iterator<Item = &'a TypeTree>,
        depth: usize,
    ) -> Result<(), String> {
        for ty in types {
            self.check(ty, depth)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nominal() -> TypeTree {
        let source: crate::SourceText = "struct Value { byte: u8 }".into();
        TypeTree::Struct {
            nominal: Nominal {
                source_identity: Some("fixture-source".into()),
                package_name: "fixture".into(),
                package_version: "1.0.0".into(),
                manifest_digest: digest(b"manifest"),
                definition_path: "fixture::Value".into(),
                declaration_digest: source.identity().into(),
            },
            source,
            fields: vec![Field {
                position: 0,
                name: "byte".into(),
                declaration: "u8".into(),
                ty: TypeTree::Scalar("u8".into()),
            }],
        }
    }

    #[test]
    fn repeated_shapes_reuse_only_exact_evidence_and_still_visit_children() {
        let original = nominal();
        let copy = original.clone();
        let mut validation = Validation::default();
        validation.check(&original, 0).unwrap();
        validation.check(&copy, 0).unwrap();
        assert_eq!(validation.shapes.len(), 1);
        assert_eq!(validation.entries, 4);
        let mut changed = original.clone();
        let TypeTree::Struct { fields, .. } = &mut changed else {
            unreachable!()
        };
        fields[0].ty = TypeTree::Scalar("u16".into());
        assert!(validation.check(&changed, 0).is_err());
        assert_eq!(validation.shapes.len(), 1);
    }

    #[test]
    fn repeated_shapes_keep_entry_and_depth_limits() {
        let value = nominal();
        crate::data_types::token(&value).unwrap();
        assert!(crate::data_types::has_checked_shape(&value));
        let mut validation = Validation::default();
        for _ in 0..MAX_ENTRIES / 2 {
            validation.check(&value, 0).unwrap();
        }
        assert!(validation.check(&value, 0).is_err());
        let mut validation = Validation::default();
        validation.check(&value, 0).unwrap();
        assert!(validation.check(&value, MAX_DEPTH).is_err());
        assert!(validation.check(&value, MAX_DEPTH + 1).is_err());
    }

    #[test]
    fn converted_shape_reuse_requires_every_declaration_and_field_byte() {
        let original = nominal();
        crate::data_types::token(&original).unwrap();
        assert!(crate::data_types::has_checked_shape(&original));
        let mut changed = original.clone();
        let TypeTree::Struct { fields, .. } = &mut changed else {
            unreachable!()
        };
        fields[0].declaration = "u16".into();
        assert!(!crate::data_types::has_checked_shape(&changed));
        assert!(tree(&changed).is_err());
        let TypeTree::Struct { source, .. } = &mut changed else {
            unreachable!()
        };
        *source = "struct Value { byte: u16 }".into();
        assert!(tree(&changed).is_err());
        assert!(tree(&original).is_ok());
    }

    #[test]
    fn unfinished_recursive_shapes_never_enter_reuse() {
        let source: crate::SourceText = "struct Recursive { child: Recursive }".into();
        let mut value = TypeTree::Scalar("u8".into());
        for _ in 0..MAX_DEPTH + 2 {
            value = TypeTree::Struct {
                nominal: Nominal {
                    source_identity: None,
                    package_name: "fixture".into(),
                    package_version: "1.0.0".into(),
                    manifest_digest: digest(b"manifest"),
                    definition_path: "fixture::Recursive".into(),
                    declaration_digest: source.identity().into(),
                },
                source: source.clone(),
                fields: vec![Field {
                    position: 0,
                    name: "child".into(),
                    declaration: "Recursive".into(),
                    ty: value,
                }],
            };
        }
        let mut validation = Validation::default();
        assert!(validation.check(&value, 0).unwrap_err().contains("budget"));
        assert!(validation.shapes.is_empty());
    }

    #[test]
    fn observation_inventory_keeps_unsupported_and_root_scalar_rules() {
        let unsupported = Observation::Unsupported { reason: "".into() };
        assert!(observations([&unsupported].into_iter()).is_err());
        let character = TypeTree::Scalar("char".into());
        let mut validation = Validation::default();
        validation.check(&character, 0).unwrap();
        assert!(validation.check(&character, 1).is_err());
    }

    #[test]
    fn mapped_conversion_counts_and_bounds_every_real_payload_component() {
        let mapped = TypeTree::MappedIntConversionResult {
            ok: Box::new(TypeTree::Tuple(vec![TypeTree::Scalar("usize".into()); 2])),
        };
        let mut validation = Validation::default();
        validation.check(&mapped, 0).unwrap();
        assert_eq!(validation.entries, 4);

        assert!(
            Validation::default()
                .check(&mapped, MAX_DEPTH - 1)
                .unwrap_err()
                .contains("budget")
        );
    }
}
