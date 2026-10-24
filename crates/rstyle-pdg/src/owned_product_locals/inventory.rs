//! Full HIR use inventory: every owned occurrence has an explicit checked role.
use super::index::{Keys, SourceIndex, key};
use super::*;

pub(super) fn nodes<'a>(root: &'a HirNode, output: &mut Vec<&'a HirNode>) {
    output.push(root);
    for child in &root.children {
        nodes(child, output);
    }
}

pub(super) fn verify(
    checked: &Checked<'_>,
    plan: &source::Plan,
    facts: &dyn EvidenceFacts,
    index: &SourceIndex<'_>,
) -> Result<(), String> {
    let all = &index.nodes;
    let mut generated = Vec::new();
    nodes(checked.initializer, &mut generated);
    let generated_bindings: Keys = generated
        .iter()
        .filter(|node| node.kind == "pat_binding")
        .map(|node| key(&node.hir))
        .collect();

    let generated: Keys = generated.iter().map(|node| key(&node.hir)).collect();
    let guards = guard_nodes(plan, facts, index)?;
    let permitted = permitted(checked, &guards, facts);

    let owned_uses: Keys = checked
        .updates
        .iter()
        .map(|update| key(&update.receiver.hir))
        .chain([key(&checked.movement.hir)])
        .collect();
    statement_inventory(checked, &guards, facts)?;

    for node in all {
        if node.is_unsafe_block {
            return Err("Owned local cannot hide unsafe evaluation".into());
        }
        let owned_use = node.res.as_ref().is_some_and(|res| {
            res.kind == "local" && res.target.as_ref() == Some(&checked.binding.hir)
        });
        if owned_use && !owned_uses.contains(&key(&node.hir)) {
            return Err(
                "Owned local has an alias, escape, duplicate read or use after move".into(),
            );
        }
        // The independently checked propagation witness owns all generated
        // protocol nodes. Its helper operand and arguments were checked too.
        if generated.contains(&key(&node.hir)) {
            continue;
        }
        if owned_node(node, checked.product())? && !permitted.contains(&key(&node.hir)) {
            return Err("Owned product occurrence lacks an explicit affine role".into());
        }
    }

    binding_inventory(checked, &generated_bindings, facts)
}

fn statement_inventory(
    checked: &Checked<'_>,
    guards: &[&HirNode],
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let expected: Vec<_> = guards
        .iter()
        .copied()
        .chain([checked.declaration])
        .chain(checked.updates.iter().map(|update| update.assignment))
        .chain([checked.terminal])
        .collect();
    check_statements(&facts.body, &expected, facts)
}

fn check_statements(
    body: &HirNode,
    expected: &[&HirNode],
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if body.kind != "expr_block" || body.children.len() != expected.len() {
        return Err("Owned function loses its complete source statement inventory".into());
    }
    for (actual, expected) in body.children.iter().zip(expected) {
        let actual = statement_child(actual, facts)?;
        if actual.hir != expected.hir
            || actual.span != expected.span
            || actual.kind != expected.kind
        {
            return Err("Owned statements differ from original evaluation order".into());
        }
    }
    Ok(())
}

fn statement_child<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<&'a HirNode, String> {
    if !matches!(node.kind.as_str(), "stmt_expr" | "stmt_semi") {
        return Ok(node);
    }
    let [child] = node.children.as_slice() else {
        return Err("Owned statement wrapper has hidden operands".into());
    };

    let suffix = crate::source_evidence::byte_range(
        &facts.source.bytes,
        ByteSpan {
            start: child.span.end,
            end: node.span.end,
        },
    )?;
    if node.expansion.is_some()
        || node.dispatch.is_some()
        || node.is_unsafe_block
        || child.span.start != node.span.start
        || (node.kind == "stmt_expr" && !suffix.is_empty())
        || (node.kind == "stmt_semi" && suffix.trim() != ";")
    {
        return Err("Owned statement wrapper differs from its original child source".into());
    }
    Ok(child)
}

fn permitted(checked: &Checked<'_>, guards: &[&HirNode], facts: &dyn EvidenceFacts) -> Keys {
    let mut allowed: Keys = [
        &facts.body.hir,
        &checked.pattern.hir,
        &checked.terminal.hir,
        &checked.movement.hir,
    ]
    .into_iter()
    .map(key)
    .chain(
        checked
            .updates
            .iter()
            .map(|update| key(&update.receiver.hir)),
    )
    .collect();
    for guard in guards {
        let mut subtree = Vec::new();
        nodes(guard, &mut subtree);
        allowed.extend(subtree.iter().map(|node| key(&node.hir)));
    }
    allowed
}

fn binding_inventory(
    checked: &Checked<'_>,
    generated_bindings: &Keys,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    for binding in &facts.bindings {
        if !binding.is_param && generated_bindings.contains(&key(&binding.hir)) {
            continue;
        }
        let token = binding
            .ty
            .normalized
            .as_deref()
            .ok_or("Owned body binding lacks its complete type")?;
        if contains(&crate::data_types::parse(token)?, checked.product())
            && binding.hir != checked.binding.hir
            && (binding.is_param || !generated_bindings.contains(&key(&binding.hir)))
        {
            return Err("Owned product has unproved parameter or local storage".into());
        }
    }
    Ok(())
}

fn owned_node(node: &HirNode, product: &TypeToken) -> Result<bool, String> {
    node.ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        .map(crate::data_types::parse)
        .transpose()
        .map(|token| token.as_ref().is_some_and(|token| contains(token, product)))
}

fn contains(token: &TypeToken, product: &TypeToken) -> bool {
    if token == product {
        return true;
    }
    match token {
        TypeToken::Shared(value) | TypeToken::Option(value) => contains(value, product),
        TypeToken::Result { ok, err } => contains(ok, product) || contains(err, product),
        TypeToken::Tuple(values) => values.iter().any(|value| contains(value, product)),
        TypeToken::Struct { fields, .. } => {
            fields.iter().any(|(_, value)| contains(value, product))
        }
        _ => false,
    }
}

fn guard_nodes<'a>(
    plan: &source::Plan,
    facts: &dyn EvidenceFacts,
    index: &SourceIndex<'a>,
) -> Result<Vec<&'a HirNode>, String> {
    plan.guards
        .iter()
        .map(|span| {
            let guard = index.unique(*span, "expr_if")?;
            let [condition, branch] = guard.children.as_slice() else {
                return Err("Owned early guard has extra or missing branches".into());
            };
            if guard.expansion.is_some() || guard.dispatch.is_some() {
                return Err("Owned guard must be an original builtin branch".into());
            }
            condition_nodes(condition)?;
            error_branch(branch, facts)?;
            Ok(guard)
        })
        .collect()
}

fn error_branch(branch: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let ret = one(branch, "expr_block")?;
    let ret = statement_child(ret, facts)?;
    let constructor = one(ret, "expr_ret")?;

    let checked = crate::concrete_results::verify_constructor(constructor, facts)?;
    if checked.construct != "data.result-err"
        || checked.payload.kind != "expr_path"
        || !checked.payload.children.is_empty()
        || checked.payload.expansion.is_some()
        || checked.payload.dispatch.is_some()
    {
        return Err("Owned guard must return its exact direct Err payload".into());
    }
    Ok(())
}

fn one<'a>(node: &'a HirNode, kind: &str) -> Result<&'a HirNode, String> {
    if node.kind != kind || node.expansion.is_some() || node.dispatch.is_some() {
        return Err(format!("Owned guard requires original {kind}"));
    }
    match node.children.as_slice() {
        [child] => Ok(child),
        _ => Err("Owned guard has hidden or missing operands".into()),
    }
}

fn condition_nodes(root: &HirNode) -> Result<(), String> {
    let mut all = Vec::new();
    nodes(root, &mut all);
    for node in all {
        let shape = if node.kind == "expr_binary" {
            node.children.len() == 2
                && node
                    .dispatch
                    .as_ref()
                    .is_some_and(|dispatch| dispatch.kind == "builtin")
        } else {
            node.children.is_empty() && node.dispatch.is_none()
        };
        if !shape
            || !matches!(node.kind.as_str(), "expr_binary" | "expr_path" | "expr_lit")
            || node.expansion.is_some()
            || node.is_unsafe_block
            || !matches!(crate::data_aggregates::parsed_type(node)?, TypeToken::Scalar(ref value) if crate::concrete_types::scalar(value) && value != "unit")
        {
            return Err("Owned guard condition contains unsupported HIR evaluation".into());
        }
    }
    Ok(())
}

pub(super) fn formal_arguments(call: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if call.kind != "expr_call" || call.expansion.is_some() {
        return Err("Owned initializer requires its direct helper call".into());
    }
    let syn::Expr::Call(syntax) = crate::source_evidence::original_expression(call, facts)? else {
        return Err("Owned helper lacks direct original call syntax".into());
    };
    if !syntax.attrs.is_empty()
        || syntax.args.len() != call.children.len()
        || !matches!(syntax.func.as_ref(), syn::Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none())
    {
        return Err("Owned helper has unsupported call source".into());
    }

    for (source, argument) in syntax.args.iter().zip(&call.children) {
        if !matches!(source, syn::Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none())
        {
            return Err("Owned helper arguments require direct formals".into());
        }
        crate::source_evidence::verify_child_span(
            call,
            argument,
            syn::spanned::Spanned::span(source),
            facts,
        )?;
        formal(argument, facts)?;
    }
    Ok(())
}

fn formal(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    let target = node
        .res
        .as_ref()
        .filter(|res| res.kind == "local")
        .and_then(|res| res.target.as_ref());
    if node.kind != "expr_path"
        || !node.children.is_empty()
        || node.expansion.is_some()
        || node.dispatch.is_some()
        || !facts
            .bindings
            .iter()
            .any(|binding| binding.is_param && Some(&binding.hir) == target)
    {
        return Err("Owned expression requires one resolved direct formal".into());
    }
    Ok(())
}

pub(super) fn copy_value(
    node: &HirNode,
    ty: &TypeToken,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let scalar = match ty {
        TypeToken::Scalar(value) if crate::concrete_types::scalar(value) && value != "unit" => node,
        TypeToken::Option(value) if matches!(value.as_ref(), TypeToken::Scalar(name) if crate::concrete_types::scalar(name) && name != "unit") =>
        {
            let constructor = crate::checked_options::verify_constructor(node, facts)?;
            if constructor == "data.option-none" && node.children.is_empty() {
                return Ok(());
            }
            if constructor != "data.option-some" {
                return Err("Owned Option overwrite requires its exact scalar Some/None".into());
            }

            let [value] = node.children.as_slice() else {
                return Err("Owned Some loses its payload".into());
            };
            value
        }
        _ => return Err("Owned overwrite requires a complete Copy scalar/Option field".into()),
    };
    if scalar.kind == "expr_lit"
        && scalar.children.is_empty()
        && scalar.expansion.is_none()
        && scalar.dispatch.is_none()
    {
        return Ok(());
    }
    formal(scalar, facts)
}

#[cfg(test)]
mod tests;
