//! Affine stack ownership for a complete checked fresh-product function family.
mod index;
mod inventory;
mod source;
#[cfg(test)]
mod tests;

use crate::concrete_results::fresh_products::{ProductBoundary, product_boundary};
use crate::{
    BindingFact, ByteSpan, HirNode, HirRef, data_types::TypeToken, unit_store::EvidenceFacts,
};

pub(crate) const CAPABILITY: &str = "rust.linear-product-result-flow";

pub(crate) struct Checked<'a> {
    boundary: ProductBoundary,
    binding: &'a BindingFact,
    declaration: &'a HirNode,
    pattern: &'a HirNode,
    initializer: &'a HirNode,
    operand: &'a HirNode,
    updates: Vec<CheckedUpdate<'a>>,
    terminal: &'a HirNode,
    movement: &'a HirNode,
}

#[derive(Clone)]
pub(crate) struct CheckedUpdate<'a> {
    assignment: &'a HirNode,
    receiver: &'a HirNode,
    value: &'a HirNode,
    position: usize,
}

impl<'a> Checked<'a> {
    pub(crate) fn product(&self) -> &TypeToken {
        &self.boundary.ok
    }
    pub(crate) fn binding(&self) -> &'a BindingFact {
        self.binding
    }
    pub(crate) fn initializer_operand(&self) -> &'a HirNode {
        self.operand
    }
    pub(crate) fn updates(&self) -> &[CheckedUpdate<'a>] {
        &self.updates
    }
    pub(crate) fn update_for(&self, node: &HirNode) -> Option<&CheckedUpdate<'a>> {
        self.updates
            .iter()
            .find(|update| update.assignment.hir == node.hir && update.assignment.span == node.span)
    }
    pub(crate) fn owns_binding(&self, hir: &HirRef) -> bool {
        &self.binding.hir == hir
    }
}

impl<'a> CheckedUpdate<'a> {
    pub(crate) fn assignment(&self) -> &'a HirNode {
        self.assignment
    }
    pub(crate) fn receiver(&self) -> &'a HirNode {
        self.receiver
    }
    pub(crate) fn value(&self) -> &'a HirNode {
        self.value
    }
    pub(crate) fn field_position(&self) -> usize {
        self.position
    }
}

/// A candidate is never permission: malformed candidates must fail verification.
pub(crate) fn candidate(facts: &dyn EvidenceFacts) -> bool {
    crate::extraction_profile::facts_support(facts, CAPABILITY)
        && crate::concrete_results::fresh_products::interface_used(facts).unwrap_or(false)
        && facts.bindings.iter().any(|binding| {
            !binding.is_param
                && binding.is_mut
                && binding
                    .ty
                    .normalized
                    .as_deref()
                    .and_then(|ty| crate::data_types::parse(ty).ok())
                    .is_some_and(|ty| matches!(ty, TypeToken::Struct { .. }))
        })
}

pub(crate) fn verify(facts: &dyn EvidenceFacts) -> Result<Checked<'_>, String> {
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Owned product locals require their registered capability".into());
    }

    let boundary = product_boundary(facts)?;
    let plan = source::plan(facts)?;
    let index = index::SourceIndex::new(&facts.body)?;

    let declaration = index.unique(plan.declaration, "stmt_let")?;
    let pattern = index.unique(plan.pattern, "pat_binding")?;
    let binding = binding(pattern, &plan.name, facts, &boundary.ok)?;
    let initializer = index.unique(plan.initializer, "expr_try_desugar")?;

    if declaration.children.len() != 2
        || declaration.children[0].hir != pattern.hir
        || declaration.children[1].hir != initializer.hir
        || declaration.expansion.is_some()
    {
        return Err("Owned declaration has extra or reordered operands".into());
    }

    let operand = initializer_operand(initializer, facts, &boundary)?;
    let updates = plan
        .updates
        .iter()
        .map(|plan| update(plan, binding, facts, &index))
        .collect::<Result<_, _>>()?;
    let terminal = index.unique(plan.terminal, "expr_call")?;
    let movement = terminal_move(terminal, binding, facts, &boundary)?;

    let checked = Checked {
        boundary,
        binding,
        declaration,
        pattern,
        initializer,
        operand,
        updates,
        terminal,
        movement,
    };

    inventory::verify(&checked, &plan, facts, &index)?;
    Ok(checked)
}

/// Returns authority only for the exact assignment in a complete checked body.
pub(crate) fn update_for<'a>(
    node: &HirNode,
    facts: &'a dyn EvidenceFacts,
) -> Result<Option<CheckedUpdate<'a>>, String> {
    if node.kind != "expr_assign" || !candidate(facts) {
        return Ok(None);
    }
    let checked = verify(facts)?;
    Ok(checked.update_for(node).cloned())
}

fn binding<'a>(
    pattern: &HirNode,
    name: &str,
    facts: &'a dyn EvidenceFacts,
    product: &TypeToken,
) -> Result<&'a BindingFact, String> {
    let bindings: Vec<_> = facts
        .bindings
        .iter()
        .filter(|binding| binding.hir == pattern.hir)
        .collect();
    let [binding] = bindings.as_slice() else {
        return Err("Owned local lacks one exact binding".into());
    };
    if binding.is_param
        || !binding.is_mut
        || binding.name != name
        || binding.decl_span != pattern.span
        || pattern.binding_mut != Some(true)
        || pattern.binding.as_ref() != Some(&binding.symbol)
        || !pattern.children.is_empty()
        || binding
            .ty
            .normalized
            .as_deref()
            .map(crate::data_types::parse)
            .transpose()?
            .as_ref()
            != Some(product)
        || crate::data_aggregates::parsed_type(pattern)? != *product
    {
        return Err("Owned local binding/source/type authority disagrees".into());
    }
    Ok(binding)
}

fn initializer_operand<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
    boundary: &ProductBoundary,
) -> Result<&'a HirNode, String> {
    let propagation = crate::product_result_propagation::verify(node, facts)?;
    if crate::data_aggregates::parsed_type(node)? != boundary.ok
        || crate::data_aggregates::parsed_type(propagation.operand)? != boundary.result
    {
        return Err("Owned initializer loses its exact product Result boundary".into());
    }

    let call = propagation.operand;
    inventory::formal_arguments(call, facts)?;
    let helper = crate::helper_component_outputs::verify_product_call(call, facts)?;

    if helper.result != boundary.result
        || helper.ok != boundary.ok
        || helper.error != boundary.error
    {
        return Err("Owned initializer helper summary has different complete types".into());
    }
    Ok(call)
}

fn update<'a>(
    plan: &source::Update,
    binding: &BindingFact,
    facts: &'a dyn EvidenceFacts,
    index: &index::SourceIndex<'a>,
) -> Result<CheckedUpdate<'a>, String> {
    let assignment = index.unique(plan.span, "expr_assign")?;
    let (target, receiver, value) = update_source(assignment, plan)?;

    local_use(receiver, binding)?;
    if target.kind != "expr_field" {
        return Err("Owned update must name its direct field".into());
    }
    let product = crate::data_aggregates::parsed_type(receiver)?;
    let TypeToken::Struct { fields, .. } = &product else {
        return Err("Owned update lacks its struct".into());
    };

    let matches: Vec<_> = fields
        .iter()
        .enumerate()
        .filter(|(_, (name, _))| name == &plan.field)
        .collect();
    let [(position, (_name, ty))] = matches.as_slice() else {
        return Err("Owned field lacks one canonical declaration slot".into());
    };
    if crate::data_aggregates::parsed_type(target)? != *ty
        || crate::data_aggregates::parsed_type(value)? != *ty
    {
        return Err("Owned overwrite differs from its declared field type".into());
    }

    // Reuse the existing exact source member/resolved declaration slot proof;
    // selecting this target grants no field read or independent Copy authority.
    if crate::data_value_fields::index(target, facts)? != *position {
        return Err("Owned overwrite changes the resolved canonical field slot".into());
    }
    inventory::copy_value(value, ty, facts)?;

    Ok(CheckedUpdate {
        assignment,
        receiver,
        value,
        position: *position,
    })
}

fn update_source<'a>(
    assignment: &'a HirNode,
    plan: &source::Update,
) -> Result<(&'a HirNode, &'a HirNode, &'a HirNode), String> {
    let [target, value] = assignment.children.as_slice() else {
        return Err("Owned update needs exact target/value operands".into());
    };
    let [receiver] = target.children.as_slice() else {
        return Err("Owned field needs one receiver".into());
    };
    if target.span != plan.target
        || receiver.span != plan.receiver
        || value.span != plan.value
        || assignment.expansion.is_some()
        || assignment.dispatch.is_some()
        || target.expansion.is_some()
        || target.dispatch.is_some()
    {
        return Err("Owned field operands differ from original source slots".into());
    }
    Ok((target, receiver, value))
}

fn terminal_move<'a>(
    node: &'a HirNode,
    binding: &BindingFact,
    facts: &dyn EvidenceFacts,
    boundary: &ProductBoundary,
) -> Result<&'a HirNode, String> {
    let checked = crate::concrete_results::verify_constructor(node, facts)?;
    if checked.construct != "data.result-ok"
        || crate::data_aggregates::parsed_type(node)? != boundary.result
    {
        return Err("Owned terminal needs its exact Ok constructor".into());
    }
    local_use(checked.payload, binding)?;
    Ok(checked.payload)
}

fn local_use(node: &HirNode, binding: &BindingFact) -> Result<(), String> {
    if crate::source_evidence::resolved_local_leaf(node) != Some(&binding.hir)
        || node.dispatch.is_some()
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_ref()) != binding.ty.normalized.as_ref()
    {
        return Err("Owned value use loses its exact owner binding".into());
    }
    Ok(())
}
