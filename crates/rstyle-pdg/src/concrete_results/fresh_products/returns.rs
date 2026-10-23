//! Explicit value roles preserve guards and reject storage or ownership reuse.
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Role {
    Ordinary,
    Return,
    Product,
}

pub(crate) fn verify_product_returns(facts: &dyn EvidenceFacts) -> Result<(), String> {
    let boundary = product_boundary(facts)?;
    super::super::reject_storage(
        facts,
        |token| shape::contains_owned(token, &boundary.ok),
        "Fresh product/Result parameters or storage require move/liveness proof",
    )?;
    let source: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    crate::source_evidence::closed_body(&source.block)?;

    walk(&facts.body, Role::Return, &boundary, facts)
}

fn walk(
    node: &HirNode,
    role: Role,
    boundary: &ProductBoundary,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if matches!(
        node.kind.as_str(),
        "expr_loop"
            | "expr_while"
            | "expr_for"
            | "expr_labelled_loop"
            | "expr_match"
            | "expr_closure"
            | "expr_break"
            | "expr_continue"
            | "expr_try_desugar"
    ) {
        return Err("Fresh product returns require supported block/if/return flow".into());
    }
    if node.kind == "expr_ret" {
        return explicit_return(node, boundary, facts);
    }

    let token = node
        .ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        .map(crate::data_types::parse)
        .transpose()?;
    check_use(token.as_ref(), role, &boundary.result, &boundary.ok)?;
    match role {
        Role::Return => returning(node, boundary, facts),
        Role::Product => product(node, boundary, facts),
        Role::Ordinary => children(node, boundary, facts),
    }
}

pub(super) fn check_use(
    token: Option<&TypeToken>,
    role: Role,
    result: &TypeToken,
    product: &TypeToken,
) -> Result<(), String> {
    let expected = match role {
        Role::Return => Some(result),
        Role::Product => Some(product),
        Role::Ordinary => None,
    };
    if let Some(expected) = expected {
        if token != Some(expected) {
            return Err("Fresh product return role has a missing or different exact type".into());
        }
    } else if token.is_some_and(|token| shape::contains_owned(token, product)) {
        return Err("Fresh product/Result cannot be stored, reused, moved or discarded".into());
    }
    Ok(())
}

fn explicit_return(
    node: &HirNode,
    boundary: &ProductBoundary,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let [value] = node.children.as_slice() else {
        return Err("Fresh product return requires exactly one complete operand".into());
    };
    walk(value, Role::Return, boundary, facts)
}

fn returning(
    node: &HirNode,
    boundary: &ProductBoundary,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    match node.kind.as_str() {
        "expr_block" => block(node, boundary, facts),
        "expr_if" if node.children.len() == 3 => {
            walk(&node.children[0], Role::Ordinary, boundary, facts)?;
            walk(&node.children[1], Role::Return, boundary, facts)?;
            walk(&node.children[2], Role::Return, boundary, facts)
        }
        "expr_call" => constructor(node, boundary, facts),
        _ => Err(
            "Fresh product Result must be returned by an exact constructor and supported flow"
                .into(),
        ),
    }
}

fn block(
    node: &HirNode,
    boundary: &ProductBoundary,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let Some((tail, prefix)) = node.children.split_last() else {
        return Err("Fresh product return block lacks its complete value".into());
    };
    for statement in prefix {
        walk(statement, Role::Ordinary, boundary, facts)?;
    }
    walk(tail, Role::Return, boundary, facts)
}

fn constructor(
    node: &HirNode,
    boundary: &ProductBoundary,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let checked = super::super::verify_constructor(node, facts)?;
    let role = if checked.construct == "data.result-ok" {
        Role::Product
    } else {
        Role::Ordinary
    };
    walk(checked.payload, role, boundary, facts)
}

fn product(
    node: &HirNode,
    boundary: &ProductBoundary,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    if node.kind != "expr_struct" || node.expansion.is_some() || node.dispatch.is_some() {
        return Err("Fresh Result success requires a direct original complete struct".into());
    }
    let syn::Expr::Struct(syntax) = crate::source_evidence::original_expression(node, facts)?
    else {
        return Err("Fresh product requires its original struct expression".into());
    };
    if syntax.rest.is_some() {
        return Err("Fresh product cannot move a struct update base".into());
    }
    children(node, boundary, facts)
}

fn children(
    node: &HirNode,
    boundary: &ProductBoundary,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    for child in &node.children {
        walk(child, Role::Ordinary, boundary, facts)?;
    }
    Ok(())
}
