use super::*;
use crate::{HirNode, concrete_types::Field};
use syn::spanned::Spanned;

fn static_str(ty: &syn::Type) -> bool {
    let syn::Type::Reference(reference) = ty else {
        return false;
    };
    super::super::copy_components::source::static_str(reference).is_ok()
}

fn fields(tree: &TypeTree) -> Result<&[Field], String> {
    let TypeTree::Struct { fields, .. } = tree else {
        return Err("Constructor result is not a concrete struct".into());
    };
    if fields.len() != 2
        || fields
            .iter()
            .filter(|field| {
                matches!(&field.ty,
            TypeTree::Shared(inner) if inner.as_ref() == &TypeTree::Str)
            })
            .count()
            != 1
        || fields
            .iter()
            .filter(|field| field.ty == TypeTree::Scalar("bool".into()))
            .count()
            != 1
    {
        return Err("Constructor requires exactly one static string and one bool field".into());
    }
    Ok(fields)
}

pub(super) fn declaration(item: &syn::ItemStruct, tree: &TypeTree) -> Result<(), String> {
    let observed = fields(tree)?;
    if !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || !matches!(item.fields, syn::Fields::Named(_))
        || item.fields.len() != observed.len()
    {
        return Err("Constructor requires an exact nongeneric named product".into());
    }
    for (position, (source, field)) in item.fields.iter().zip(observed).enumerate() {
        let type_matches = match &field.ty {
            TypeTree::Shared(_) => static_str(&source.ty),
            TypeTree::Scalar(value) => matches!(&source.ty, syn::Type::Path(path)
                if path.qself.is_none() && path.path.is_ident(value)),
            _ => false,
        };
        if field.position != position
            || source.ident.as_ref().is_none_or(|name| name != &field.name)
            || !type_matches
        {
            return Err("Constructor original field type/name/order differs".into());
        }
    }
    Ok(())
}

pub(super) fn function(
    function: &syn::ItemFn,
    facts: &HirFnFacts,
    tree: &TypeTree,
) -> Result<(), String> {
    let (parameter, value) = syntax(function)?;
    let observed = fields(tree)?;
    let formal = formal(function, facts, parameter, tree)?;

    let [literal] = facts.body.children.as_slice() else {
        return Err("Constructor body has additional recorded operations".into());
    };

    if facts.body.kind != "expr_block"
        || !plain(&facts.body)
        || !typed(&facts.body, tree)?
        || literal.kind != "expr_struct"
        || !plain(literal)
        || !typed(literal, tree)?
        || value.fields.len() != observed.len()
        || literal.children.len() != observed.len()
        || facts.body.span
            != crate::source_evidence::original_span(&facts.source.bytes, function.block.span())
        || literal.span != crate::source_evidence::original_span(&facts.source.bytes, value.span())
    {
        return Err("Constructor source/HIR body and complete field roster disagree".into());
    }

    let context = FieldCheck {
        parameter,
        formal,
        facts,
    };

    for ((source, field), child) in value.fields.iter().zip(observed).zip(&literal.children) {
        context.verify(source, field, child)?;
    }

    Ok(())
}

fn formal<'a>(
    function: &syn::ItemFn,
    facts: &'a HirFnFacts,
    parameter: &syn::Ident,
    tree: &TypeTree,
) -> Result<&'a crate::ParamFact, String> {
    let [formal] = facts.signature.params.as_slice() else {
        return Err("Constructor requires one supplied static string formal".into());
    };

    if facts.signature.is_async
        || facts.signature.is_unsafe
        || facts.signature.is_variadic
        || facts.signature.receiver.is_some()
        || !facts.signature.generics.is_empty()
        || facts.signature.is_const != function.sig.constness.is_some()
        || facts.signature.abi.known_value() != Some("Rust")
    {
        return Err(
            "Constructor signature observations contradict original safe Rust source".into(),
        );
    }

    if formal.position != 0
        || formal.pat.kind != "pat_binding"
        || formal.pat.binding.as_deref() != Some(&formal.symbol)
        || formal.pat.binding_mut != Some(false)
        || facts.signature.has_self
        || !typed(&formal.pat, &TypeTree::Shared(Box::new(TypeTree::Str)))?
        || formal.name_span
            != crate::source_evidence::original_span(&facts.source.bytes, parameter.span())
        || formal.pat.span != formal.name_span
        || formal.ty.normalized != formal.pat.ty.as_ref().and_then(|ty| ty.normalized.clone())
        || facts.signature.return_ty.normalized.as_deref()
            != Some(crate::data_types::token(tree)?.as_str())
    {
        return Err("Constructor formal binding/type evidence disagrees".into());
    }

    Ok(formal)
}

pub(super) fn syntax(function: &syn::ItemFn) -> Result<(&syn::Ident, &syn::ExprStruct), String> {
    let signature = &function.sig;
    if !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !matches!(&signature.output, syn::ReturnType::Type(_, ty)
            if matches!(ty.as_ref(), syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("Self")))
    {
        return Err("Constructor requires a safe concrete Rust signature returning Self".into());
    }

    let Some(syn::FnArg::Typed(argument)) = signature.inputs.first() else {
        return Err("Constructor requires its supplied static string".into());
    };
    let syn::Pat::Ident(parameter) = argument.pat.as_ref() else {
        return Err("Constructor formal must be one plain binding".into());
    };

    if signature.inputs.len() != 1
        || !argument.attrs.is_empty()
        || !parameter.attrs.is_empty()
        || parameter.by_ref.is_some()
        || parameter.mutability.is_some()
        || parameter.subpat.is_some()
        || !static_str(&argument.ty)
    {
        return Err("Constructor requires one explicitly static immutable string binding".into());
    }

    let [syn::Stmt::Expr(syn::Expr::Struct(value), None)] = function.block.stmts.as_slice() else {
        return Err("Constructor body must be its complete original struct literal".into());
    };

    if !value.attrs.is_empty()
        || value.qself.is_some()
        || !value.path.is_ident("Self")
        || value.rest.is_some()
        || value.dot2_token.is_some()
    {
        return Err("Constructor cannot use updates or unresolved constructor paths".into());
    }

    initializers(value, &parameter.ident)?;

    Ok((&parameter.ident, value))
}

fn initializers(value: &syn::ExprStruct, parameter: &syn::Ident) -> Result<(), String> {
    let mut references = 0;
    let mut flags = 0;

    for field in &value.fields {
        if !field.attrs.is_empty() {
            return Err(
                "Constructor fields cannot carry conditional or transforming attributes".into(),
            );
        }
        match &field.expr {
            syn::Expr::Path(path)
                if path.attrs.is_empty()
                    && path.qself.is_none()
                    && path.path.is_ident(parameter) =>
            {
                references += 1
            }
            syn::Expr::Lit(literal)
                if literal.attrs.is_empty() && matches!(literal.lit, syn::Lit::Bool(_)) =>
            {
                flags += 1
            }
            _ => return Err("Constructor initializers require its formal and literal flag".into()),
        }
    }

    if references != 1 || flags != 1 {
        return Err("Constructor must initialize exactly one supplied string and one flag".into());
    }

    Ok(())
}

struct FieldCheck<'a> {
    parameter: &'a syn::Ident,
    formal: &'a crate::ParamFact,
    facts: &'a HirFnFacts,
}

impl FieldCheck<'_> {
    fn verify(
        &self,
        source: &syn::FieldValue,
        field: &Field,
        child: &HirNode,
    ) -> Result<(), String> {
        if !source.attrs.is_empty()
            || !matches!(&source.member, syn::Member::Named(name) if name == &field.name)
            || !plain(child)
            || !child.children.is_empty()
            || !typed(child, &field.ty)?
            || child.span
                != crate::source_evidence::original_span(
                    &self.facts.source.bytes,
                    source.expr.span(),
                )
        {
            return Err("Constructor field source/order/type/evaluation evidence disagrees".into());
        }
        let valid = match (&field.ty, &source.expr) {
            (TypeTree::Shared(_), syn::Expr::Path(path)) => {
                path.attrs.is_empty()
                    && path.qself.is_none()
                    && path.path.is_ident(self.parameter)
                    && crate::source_evidence::resolved_local_leaf(child)
                        == Some(&self.formal.pat.hir)
            }
            (TypeTree::Scalar(token), syn::Expr::Lit(literal)) if token == "bool" => {
                matches!(&literal.lit, syn::Lit::Bool(value)
                if literal.attrs.is_empty() && child.kind == "expr_lit"
                    && child.lit.as_deref() == Some(if value.value { "true" } else { "false" }))
            }
            _ => false,
        };
        if !valid {
            return Err("Constructor field must preserve its formal or exact bool literal".into());
        }
        Ok(())
    }
}

fn plain(node: &HirNode) -> bool {
    node.expansion.is_none() && node.dispatch.is_none() && !node.is_unsafe_block
}

fn typed(node: &HirNode, tree: &TypeTree) -> Result<bool, String> {
    Ok(node.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
        == Some(crate::data_types::token(tree)?.as_str()))
}
