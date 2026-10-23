//! Complete original statement inventory for the bounded affine local family.
use super::*;
use syn::spanned::Spanned;

pub(super) struct Plan {
    pub declaration: ByteSpan,
    pub pattern: ByteSpan,
    pub initializer: ByteSpan,
    pub updates: Vec<Update>,
    pub terminal: ByteSpan,
    pub guards: Vec<ByteSpan>,
    pub name: String,
}

pub(super) struct Update {
    pub span: ByteSpan,
    pub target: ByteSpan,
    pub receiver: ByteSpan,
    pub value: ByteSpan,
    pub field: String,
}

pub(super) fn plan(facts: &dyn EvidenceFacts) -> Result<Plan, String> {
    let source: syn::ItemFn = syn::parse_str(&facts.source.bytes).map_err(|e| e.to_string())?;
    crate::source_evidence::closed_body(&source.block)?;
    let (terminal, prefix) = source
        .block
        .stmts
        .split_last()
        .ok_or("Missing owned terminal")?;

    let position = prefix
        .iter()
        .position(|statement| matches!(statement, syn::Stmt::Local(_)))
        .ok_or("Missing owned initialization")?;
    let (guards, remaining) = prefix.split_at(position);
    let (declaration, updates) = remaining
        .split_first()
        .ok_or("Missing owned initialization")?;
    if updates.is_empty() {
        return Err("Missing owned field updates".into());
    }
    let local = initialized_local(declaration)?;

    let name = binding(local)?;
    let init = local
        .init
        .as_ref()
        .ok_or("Owned local lacks initialization")?;

    let updates = updates
        .iter()
        .map(|update| update_plan(update, &name, facts))
        .collect::<Result<_, _>>()?;
    terminal_move(terminal, &name)?;
    for guard in guards {
        guard_source(guard)?;
    }

    let span = |value: proc_macro2::Span| {
        crate::source_evidence::original_span(&facts.source.bytes, value)
    };
    Ok(Plan {
        declaration: span(declaration.span()),
        pattern: span(match &local.pat {
            syn::Pat::Ident(binding) => binding.ident.span(),
            _ => unreachable!(),
        }),
        initializer: span(init.expr.span()),
        updates,
        terminal: span(statement_expression(terminal)?.span()),
        guards: guards.iter().map(|guard| span(guard.span())).collect(),
        name,
    })
}

fn initialized_local(statement: &syn::Stmt) -> Result<&syn::Local, String> {
    let syn::Stmt::Local(local) = statement else {
        return Err("Owned product needs one original initialized local".into());
    };
    let init = local
        .init
        .as_ref()
        .ok_or("Owned local lacks initialization")?;
    if init.diverge.is_some() || !matches!(init.expr.as_ref(), syn::Expr::Try(_)) {
        return Err("Owned initializer requires one checked question mark".into());
    }
    Ok(local)
}

fn binding(local: &syn::Local) -> Result<String, String> {
    let syn::Pat::Ident(binding) = &local.pat else {
        return Err("Owned local needs a plain binding".into());
    };
    if !local.attrs.is_empty()
        || !binding.attrs.is_empty()
        || binding.by_ref.is_some()
        || binding.mutability.is_none()
        || binding.subpat.is_some()
    {
        return Err("Owned local needs one mutable non-reference binding".into());
    }
    Ok(binding.ident.to_string())
}

fn update_plan(
    statement: &syn::Stmt,
    name: &str,
    facts: &dyn EvidenceFacts,
) -> Result<Update, String> {
    let syn::Stmt::Expr(syn::Expr::Assign(update), Some(_)) = statement else {
        return Err("Owned product requires one direct field assignment".into());
    };
    let syn::Expr::Field(field) = update.left.as_ref() else {
        return Err("Owned update requires an original named field".into());
    };
    if !update.attrs.is_empty() || !field.attrs.is_empty() || !plain_local(&field.base, name) {
        return Err("Owned update must target exactly its local".into());
    }

    let syn::Member::Named(member) = &field.member else {
        return Err("Owned update needs a named field".into());
    };
    scalar_value(&update.right)?;
    let span = |value| crate::source_evidence::original_span(&facts.source.bytes, value);

    Ok(Update {
        span: span(update.span()),
        target: span(update.left.span()),
        receiver: span(field.base.span()),
        value: span(update.right.span()),
        field: member.to_string(),
    })
}

fn terminal_move(statement: &syn::Stmt, name: &str) -> Result<(), String> {
    let syn::Stmt::Expr(syn::Expr::Call(call), None) = statement else {
        return Err("Owned terminal must be a direct tail constructor".into());
    };
    if !call.attrs.is_empty() || call.args.len() != 1 || !plain_local(&call.args[0], name) {
        return Err("Owned terminal must move its whole local once".into());
    }
    Ok(())
}

fn plain_local(expression: &syn::Expr, name: &str) -> bool {
    matches!(expression, syn::Expr::Path(path) if path.attrs.is_empty()
        && path.qself.is_none() && path.path.is_ident(name))
}

pub(super) fn statement_expression(statement: &syn::Stmt) -> Result<&syn::Expr, String> {
    match statement {
        syn::Stmt::Expr(expression, _) => Ok(expression),
        _ => Err("Owned flow expected an original expression statement".into()),
    }
}

fn scalar_value(expression: &syn::Expr) -> Result<(), String> {
    match expression {
        syn::Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none() => Ok(()),
        syn::Expr::Lit(lit) if lit.attrs.is_empty() => Ok(()),
        syn::Expr::Call(call)
            if call.attrs.is_empty()
                && call.args.len() == 1
                && matches!(call.func.as_ref(), syn::Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none()) =>
        {
            match &call.args[0] {
                syn::Expr::Path(_) | syn::Expr::Lit(_) => scalar_value(&call.args[0]),
                _ => Err("Owned Option update requires one scalar leaf".into()),
            }
        }
        _ => Err("Owned update has unsupported hidden evaluation".into()),
    }
}

fn guard_source(statement: &syn::Stmt) -> Result<(), String> {
    let syn::Expr::If(guard) = statement_expression(statement)? else {
        return Err("Owned prefix permits only checked early-return guards".into());
    };
    if !guard.attrs.is_empty() || guard.else_branch.is_some() || guard.then_branch.stmts.len() != 1
    {
        return Err("Owned guard must have one return and no else".into());
    }

    condition(&guard.cond)?;
    let syn::Stmt::Expr(syn::Expr::Return(ret), Some(_)) = &guard.then_branch.stmts[0] else {
        return Err("Owned guard must return its exact error".into());
    };
    let Some(syn::Expr::Call(call)) = ret.expr.as_deref() else {
        return Err("Owned guard needs a direct error constructor".into());
    };

    if !ret.attrs.is_empty()
        || !call.attrs.is_empty()
        || call.args.len() != 1
        || !matches!(call.func.as_ref(), syn::Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none())
        || !matches!(&call.args[0], syn::Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none())
    {
        return Err("Owned guard error contains unsupported evaluation".into());
    }
    Ok(())
}

fn condition(expression: &syn::Expr) -> Result<(), String> {
    match expression {
        syn::Expr::Path(path) if path.attrs.is_empty() && path.qself.is_none() => Ok(()),
        syn::Expr::Lit(lit) if lit.attrs.is_empty() => Ok(()),
        syn::Expr::Binary(binary)
            if binary.attrs.is_empty()
                && matches!(
                    binary.op,
                    syn::BinOp::Lt(_)
                        | syn::BinOp::Le(_)
                        | syn::BinOp::Gt(_)
                        | syn::BinOp::Ge(_)
                        | syn::BinOp::Eq(_)
                        | syn::BinOp::Ne(_)
                        | syn::BinOp::And(_)
                        | syn::BinOp::Or(_)
                ) =>
        {
            condition(&binary.left)?;
            condition(&binary.right)
        }
        syn::Expr::Paren(paren) if paren.attrs.is_empty() => condition(&paren.expr),
        _ => Err("Owned guard condition has unsupported evaluation".into()),
    }
}
