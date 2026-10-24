//! Exact fresh leaf, map constructor, map_err constructor, whole return grammar.
use crate::{HirFnFacts, HirNode, concrete_types::TypeTree};

pub(super) struct Chain<'a> {
    pub root: &'a HirNode,
    pub map: &'a HirNode,
    pub leaf: &'a HirNode,
    pub callbacks: [&'a HirNode; 2],
    pub argument: &'a HirNode,
}

pub(super) fn chain(facts: &HirFnFacts) -> Result<Chain<'_>, String> {
    let [root] = facts.body.children.as_slice() else {
        return Err("Constructor Result transport requires exactly one whole tail return".into());
    };
    let [map, error_callback] = root.children.as_slice() else {
        return Err("Result map_err requires one receiver and constructor".into());
    };
    let [leaf, callback] = map.children.as_slice() else {
        return Err("Result map requires one receiver and constructor".into());
    };
    let [argument] = leaf.children.as_slice() else {
        return Err("Result transport leaf requires one scalar argument".into());
    };
    if facts.body.kind != "expr_block"
        || root.kind != "expr_method_call"
        || map.kind != "expr_method_call"
        || leaf.kind != "expr_call"
        || facts.signature.has_self
        || facts.signature.params.len() != 1
        || [callback, error_callback, argument]
            .iter()
            .any(|node| node.kind != "expr_path" || !node.children.is_empty())
        || !constructors_only([callback, error_callback])
    {
        return Err("Result transport is outside the complete constructor chain grammar".into());
    }

    Ok(Chain {
        root,
        map,
        leaf,
        callbacks: [callback, error_callback],
        argument,
    })
}

fn constructors_only(nodes: [&HirNode; 2]) -> bool {
    nodes.iter().all(|node| {
        node.dispatch.is_none()
            && node.binding.is_none()
            && node.lit.is_none()
            && node.ty.as_ref().is_some_and(|ty| ty.normalized.is_none())
    })
}

pub(super) fn unary(tree: &TypeTree) -> Result<&TypeTree, String> {
    let fields: Vec<_> = tree.nominal_fields().collect();
    let [field] = fields.as_slice() else {
        return Err("Result constructor wrapper requires exactly one tuple field".into());
    };
    if tree.nominal_identity().is_none() || field.position != 0 || field.name != "0" {
        return Err("Result constructor wrapper is not a concrete unary tuple struct".into());
    }
    Ok(&field.ty)
}

pub(super) fn scalar_product(tree: &TypeTree) -> bool {
    // Conversion erases generated origin, so reject it before sharing the
    // token grammar. Ordinary product components retain their full shape.
    ordinary_components(tree)
        && super::types::scalar_product(&crate::data_types::TypeToken::from_tree(tree))
}

fn ordinary_components(tree: &TypeTree) -> bool {
    tree.generated_parts().is_none()
        && tree
            .components()
            .chain(tree.nominal_fields().map(|field| &field.ty))
            .all(ordinary_components)
}

pub(super) fn error_product(tree: &TypeTree) -> bool {
    matches!(tree, TypeTree::Struct { fields, .. } if !fields.is_empty()
        && fields.iter().all(|field| matches!(&field.ty, TypeTree::Scalar(token)
            if crate::concrete_types::scalar(token) && token != "unit")))
}
