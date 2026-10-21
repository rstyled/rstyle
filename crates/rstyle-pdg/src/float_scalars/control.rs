//! Exact source/HIR control evidence for closed primitive dependency leaves.
use crate::{HirFnFacts, HirNode, frontend::normalized};
use syn::spanned::Spanned;

pub(crate) const CAPABILITY: &str = "rust.closed-scalar-control-dependencies";

pub(crate) fn guard(expression: &syn::Expr) -> Result<(&syn::ExprPath, &syn::LitInt), String> {
    let syn::Expr::Binary(value) = ungroup(expression)? else {
        return Err("Scalar control requires an original integer equality".into());
    };
    if !value.attrs.is_empty() || !matches!(value.op, syn::BinOp::Eq(_)) {
        return Err("Scalar control supports only attribute-free integer equality".into());
    }
    let operands = (ungroup(&value.left)?, ungroup(&value.right)?);
    let (path, literal) = match operands {
        (syn::Expr::Path(path), syn::Expr::Lit(literal))
        | (syn::Expr::Lit(literal), syn::Expr::Path(path)) => (path, literal),
        _ => return Err("Scalar control requires one formal and one integer literal".into()),
    };
    let syn::Lit::Int(integer) = &literal.lit else {
        return Err("Scalar control literal is not an integer".into());
    };

    if !path.attrs.is_empty()
        || path.qself.is_some()
        || path.path.get_ident().is_none()
        || !literal.attrs.is_empty()
        || !matches!(integer.suffix(), "" | "i32")
        || integer.base10_parse::<i32>().is_err()
    {
        return Err("Scalar control requires an exact bare i32 formal/literal".into());
    }
    Ok((path, integer))
}

fn ungroup(expression: &syn::Expr) -> Result<&syn::Expr, String> {
    match expression {
        syn::Expr::Paren(value) if value.attrs.is_empty() => ungroup(&value.expr),
        syn::Expr::Group(value) if value.attrs.is_empty() => ungroup(&value.expr),
        syn::Expr::Paren(_) | syn::Expr::Group(_) => {
            Err("Scalar control cannot hide attributed source wrappers".into())
        }
        _ => Ok(expression),
    }
}

pub(crate) fn verify(facts: &HirFnFacts) -> Result<(), String> {
    for branch in crate::field_locations::operations(&facts.body, &["expr_if"]) {
        verify_branch(branch, facts)?;
    }
    Ok(())
}

fn verify_branch(node: &HirNode, facts: &HirFnFacts) -> Result<(), String> {
    let [condition, then, otherwise] = node.children.as_slice() else {
        return Err("Scalar branch requires its guard and both complete alternatives".into());
    };
    let source = crate::source_evidence::original_expression(node, facts)?;
    let syn::Expr::If(source) = ungroup(&source)? else {
        return Err("Scalar branch disagrees with its original source kind".into());
    };
    let (_, alternative) = source
        .else_branch
        .as_ref()
        .ok_or("Scalar branch lacks its original else alternative")?;

    let ty = normalized(node).ok_or("Scalar branch lacks its result type")?;
    if node.expansion.is_some()
        || node.is_unsafe_block
        || !super::is_float(ty)
        || normalized(then) != Some(ty)
        || normalized(otherwise) != Some(ty)
    {
        return Err("Scalar branch loses safe equal-width float alternatives".into());
    }
    for (span, child) in [
        (source.cond.span(), condition),
        (source.then_branch.span(), then),
        (alternative.span(), otherwise),
    ] {
        crate::source_evidence::verify_child_span(node, child, span, facts)?;
    }
    verify_guard(condition, facts)
}

fn verify_guard(node: &HirNode, facts: &HirFnFacts) -> Result<(), String> {
    let source = crate::source_evidence::original_expression(node, facts)?;
    let (path, _) = guard(&source)?;
    let syn::Expr::Binary(source) = ungroup(&source)? else {
        unreachable!();
    };
    if node.kind != "expr_binary"
        || node.expansion.is_some()
        || node.is_unsafe_block
        || !crate::builtin_scalars::matches(node, &["i32", "i32"], "bool", &["=="])
    {
        return Err("Scalar control loses its exact builtin i32 equality".into());
    }

    let [left, right] = node.children.as_slice() else {
        return Err("Scalar control requires two saved equality operands".into());
    };
    crate::source_evidence::verify_child_span(node, left, source.left.span(), facts)?;
    crate::source_evidence::verify_child_span(node, right, source.right.span(), facts)?;
    let (formal, constant) = if matches!(ungroup(&source.left)?, syn::Expr::Path(_)) {
        (left, right)
    } else {
        (right, left)
    };
    verify_formal(formal, path, facts)?;
    verify_literal(constant, facts)
}

fn verify_formal(node: &HirNode, source: &syn::ExprPath, facts: &HirFnFacts) -> Result<(), String> {
    let target = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == "local")
        .and_then(|resolution| resolution.target.as_ref())
        .ok_or("Scalar control formal lacks its resolved local identity")?;
    let binding = facts
        .bindings
        .iter()
        .find(|binding| &binding.hir == target)
        .ok_or("Scalar control formal lacks its binding declaration")?;
    let name = source.path.get_ident().unwrap().to_string();

    if node.kind != "expr_path"
        || !node.children.is_empty()
        || node.expansion.is_some()
        || node.is_unsafe_block
        || normalized(node) != Some("i32")
        || !binding.is_param
        || binding.is_mut
        || binding.name != name
        || binding.ty.normalized.as_deref() != Some("i32")
        || !original_parameter(binding, &name, facts)
    {
        return Err("Scalar control path loses its exact original immutable i32 formal".into());
    }
    Ok(())
}

fn original_parameter(binding: &crate::BindingFact, name: &str, facts: &HirFnFacts) -> bool {
    facts
        .signature
        .params
        .iter()
        .enumerate()
        .filter(|(position, parameter)| {
            parameter.symbol == binding.symbol
                && parameter.name_span == binding.decl_span
                && parameter.ty.normalized.as_deref() == Some("i32")
                && parameter.position as usize == *position
                && parameter.pat.hir == binding.hir
                && parameter.pat.binding.as_ref() == Some(&binding.symbol)
                && parameter.pat.binding_mut == Some(false)
                && parameter.pat.param_index == Some(parameter.position)
                && parameter.pat.kind == "pat_binding"
                && parameter.pat.children.is_empty()
                && normalized(&parameter.pat) == Some("i32")
                && facts
                    .source
                    .bytes
                    .get(parameter.name_span.start as usize..parameter.name_span.end as usize)
                    == Some(name)
        })
        .count()
        == 1
}

fn verify_literal(node: &HirNode, facts: &HirFnFacts) -> Result<(), String> {
    let source = crate::byte_literals::source(node, facts)
        .ok_or("Scalar control literal lacks original source")?;
    if node.kind != "expr_lit"
        || !node.children.is_empty()
        || node.expansion.is_some()
        || node.is_unsafe_block
        || node.res.is_some()
        || node.dispatch.is_some()
        || normalized(node) != Some("i32")
        || node.lit.as_deref() != Some(source)
    {
        return Err("Scalar control literal loses its original spelling, value or i32 type".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn guard_grammar_requires_one_formal_and_one_in_range_original_i32_literal() {
        for source in [
            "iy == 0",
            "0i32 == iy",
            "(iy) == (2147483647)",
            "iy == 0xff",
        ] {
            let expression = syn::parse_str(source).unwrap();
            assert!(super::guard(&expression).is_ok(), "{source}");
        }
        for source in [
            "iy != 0",
            "iy == other",
            "0 == 0",
            "iy == 0u32",
            "iy == 2147483648",
            "iy == -1",
            "-iy == 0",
            "module::iy == 0",
            "iy == helper()",
        ] {
            let expression = syn::parse_str(source).unwrap();
            assert!(super::guard(&expression).is_err(), "{source}");
        }
    }
}
