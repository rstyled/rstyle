//! Built-in scalar indexing with explicit normal-completion bounds semantics.

mod source;

use crate::{
    HirNode,
    data_types::TypeToken,
    field_locations::{Projection, derived::Located},
    unit_store::EvidenceFacts,
};
use serde_json::Value;

pub(crate) const BOUNDS_KEY: &str = "rstyle.data:index-bounds";

pub(crate) struct Checked<'a> {
    pub base: &'a HirNode,
    pub index: &'a HirNode,
    pub shared: bool,
    length: Option<u64>,
}

pub(crate) fn candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    node.kind == "expr_index"
        && crate::extraction_profile::ExtractionProfile::from_facts(facts)
            .is_ok_and(|profile| profile.supports("rust.scalar-index-reads"))
}

pub(crate) fn delayed_base(node: &HirNode) -> bool {
    match node.kind.as_str() {
        "expr_path" => node
            .res
            .as_ref()
            .is_some_and(|resolution| resolution.kind == "local"),
        "expr_field" => node.children.first().is_some_and(delayed_base),
        "expr_unary"
            if node
                .dispatch
                .as_ref()
                .is_some_and(|dispatch| dispatch.op == "*") =>
        {
            node.children.first().is_some_and(delayed_base)
        }
        _ => false,
    }
}

pub(crate) fn shared_base(node: &HirNode) -> bool {
    node.children
        .first()
        .and_then(|base| crate::data_aggregates::token(base).ok())
        .and_then(|token| crate::data_types::parse(token).ok())
        .is_some_and(|ty| matches!(ty, TypeToken::Shared(_)))
}

pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    if !candidate(node, facts)
        || !node.dispatch.as_ref().is_some_and(|dispatch| {
            dispatch.kind == "builtin" && dispatch.op == "[]" && dispatch.operand_tys.len() == 2
        })
    {
        return Err("Indexing requires registered built-in array/slice semantics".into());
    }
    let [base, index] = node.children.as_slice() else {
        return Err("Indexing lacks exact base and index operands".into());
    };
    if crate::data_aggregates::token(index)? != "usize" {
        return Err("Scalar indexing requires an evaluated usize index".into());
    }
    let base_ty = crate::data_types::parse(crate::data_aggregates::token(base)?)?;
    let (container, shared) = match base_ty {
        TypeToken::Shared(value) => (*value, true),
        value => (value, false),
    };
    let (element, length) = match container {
        TypeToken::Array { element, length } => (*element, Some(length)),
        TypeToken::Slice(element) if shared => (*element, None),
        _ => return Err("Index base requires an array value or a shared array/slice".into()),
    };
    if !matches!(element, TypeToken::Scalar(_))
        || crate::data_types::parse(crate::data_aggregates::token(node)?)? != element
    {
        return Err("Index result requires an exact supported scalar element type".into());
    }

    source::verify(node, facts)?;
    Ok(Checked {
        base,
        index,
        shared,
        length,
    })
}

impl Checked<'_> {
    pub fn bounds(&self) -> Result<Value, String> {
        crate::data_bounds::Bounds::index(self.length).value()
    }

    pub fn locations(&self, facts: &dyn EvidenceFacts) -> Result<Vec<Located>, String> {
        if !self.shared {
            return Ok(Vec::new());
        }

        let roots = crate::data_origins::origins(self.base, facts)?;
        let mut locations = crate::field_locations::derived::referents(self.base, facts, roots)?;
        for location in &mut locations {
            let element = match &location.value {
                crate::concrete_types::TypeTree::Array { element, .. }
                | crate::concrete_types::TypeTree::Slice(element) => element.as_ref().clone(),
                _ => return Err("Index origin contradicts its array/slice type".into()),
            };
            location.path.push(Projection::IndexedElement {
                operand_position: 1,
            });
            location.value = element;
        }

        if locations.is_empty() {
            return Err("Indexed reference lacks complete formal origins".into());
        }
        Ok(locations)
    }
}
