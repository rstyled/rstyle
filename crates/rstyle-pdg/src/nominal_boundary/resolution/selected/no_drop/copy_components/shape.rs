//! Closed finite shape grammar, independent of admission and Copy claims.
use super::*;
use crate::concrete_types::VariantShape;

pub(super) fn supported(tree: &TypeTree) -> bool {
    component(tree, 0)
}

fn component(tree: &TypeTree, depth: usize) -> bool {
    if depth > concrete_types::MAX_DEPTH {
        return false;
    }
    match tree {
        TypeTree::Scalar(token) => concrete_types::scalar(token) && token != "unit",
        TypeTree::Shared(referent) => referent.as_ref() == &TypeTree::Str,
        TypeTree::Struct { fields, .. } => {
            fields.len() <= concrete_types::MAX_FIELDS
                && fields.iter().all(|field| component(&field.ty, depth + 1))
        }
        TypeTree::MixedEnum { variants, .. } => {
            !variants.is_empty()
                && variants.len() <= concrete_types::MAX_VARIANTS
                && variants.iter().all(|variant| match &variant.shape {
                    VariantShape::Unit {} => true,
                    VariantShape::SingleTuple { field } => component(&field.ty, depth + 1),
                    VariantShape::Named { fields } => {
                        !fields.is_empty()
                            && fields.len() <= concrete_types::MAX_FIELDS
                            && fields
                                .iter()
                                .all(|(_, field)| component(&field.ty, depth + 1))
                    }
                })
        }
        _ => false,
    }
}
