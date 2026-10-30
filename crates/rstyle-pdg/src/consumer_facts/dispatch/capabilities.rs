//! Original two-method wrappers provide advisory capability evidence only.
use super::{MethodOrigin, index::CheckedDeclaration, shapes};
use crate::ByteSpan;
use serde::Serialize;
use syn::{Expr, FnArg, Pat, ReturnType, Stmt, Type};

pub type Assessment = super::Assessment<Wrapper>;

/// Exact identity inside one immutable source index; never a durable report ID.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct OwnerKey {
    pub source_owner: usize,
    pub module: usize,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct Wrapper {
    pub owner: OwnerKey,
    pub owner_name: String,
    pub caller_header: ByteSpan,
    pub methods: Vec<MethodWitness>,
}

#[derive(Debug, Serialize)]
pub struct MethodWitness {
    pub name: String,
    pub call_span: ByteSpan,
    pub callee_header: ByteSpan,
    pub origin: MethodOrigin,
}

pub(super) fn evaluate(declaration: CheckedDeclaration<'_>) -> Assessment {
    match wrapper(declaration) {
        Ok(wrapper) => Assessment {
            availability: super::super::Availability::Known(Some(wrapper)),
            reason: None,
            budget_exhausted: false,
        },
        Err(shapes::Decision::Negative(reason)) => Assessment::negative(reason),
        Err(shapes::Decision::Unsupported(reason)) => Assessment::unsupported(reason),
    }
}

fn wrapper(declaration: CheckedDeclaration<'_>) -> shapes::DispatchResult<Wrapper> {
    let function = declaration.function;
    if function.sig.receiver().is_some()
        || !matches!(&function.sig.output, ReturnType::Type(_, ty) if matches!(&**ty, Type::Tuple(_)))
    {
        return Err(shapes::Decision::Negative(
            "Only original free functions with tuple results are wrapper candidates",
        ));
    }

    let tuple = tuple(function)?;
    let (binder, owner) = parameter(function)?;
    shapes::capability_owner(declaration.module, &owner)?;
    let request = shapes::MethodRequest {
        module: declaration.module,
        binder: &binder,
        owner: &owner,
        source: declaration.source,
    };
    let methods = methods(&request, tuple)?;
    if methods[0].callee_header == methods[1].callee_header {
        return Err(shapes::Decision::Negative(
            "Wrapper repeats the same selected method",
        ));
    }

    Ok(Wrapper {
        owner: OwnerKey {
            source_owner: declaration.owner_index,
            module: declaration.module_index,
            name: owner.clone(),
        },
        owner_name: owner,
        caller_header: shapes::function_header(declaration.source, function),
        methods,
    })
}

fn parameter(function: &syn::ItemFn) -> shapes::DispatchResult<(String, String)> {
    if !function.attrs.is_empty()
        || !shapes::bounded_signature(&function.sig)
        || !tuple_return(&function.sig.output)
    {
        return Err(shapes::Decision::Unsupported(
            "Wrapper requires a plain free signature returning (u32, u32)",
        ));
    }

    let Some(FnArg::Typed(parameter)) = function
        .sig
        .inputs
        .first()
        .filter(|_| function.sig.inputs.len() == 1)
    else {
        return Err(shapes::Decision::Unsupported(
            "Wrapper requires one shared nominal parameter",
        ));
    };
    if !parameter.attrs.is_empty() {
        return Err(shapes::Decision::Unsupported(
            "Attributed wrapper parameter is unavailable",
        ));
    }

    let (Pat::Ident(binding), Type::Reference(reference)) = (&*parameter.pat, &*parameter.ty)
    else {
        return Err(shapes::Decision::Unsupported(
            "Wrapper parameter requires a plain shared nominal binding",
        ));
    };
    if !binding.attrs.is_empty()
        || binding.by_ref.is_some()
        || binding.mutability.is_some()
        || binding.subpat.is_some()
        || reference.mutability.is_some()
        || reference.lifetime.is_some()
    {
        return Err(shapes::Decision::Unsupported(
            "Wrapper binding or reference exceeds the bounded contract",
        ));
    }

    let owner = shapes::plain_type(&reference.elem).ok_or(shapes::Decision::Unsupported(
        "Wrapper nominal parameter must be an unqualified local type",
    ))?;
    Ok((binding.ident.to_string(), owner))
}

fn tuple_return(output: &ReturnType) -> bool {
    matches!(output, ReturnType::Type(_, ty) if matches!(&**ty, Type::Tuple(tuple)
        if tuple.elems.len() == 2 && tuple.elems.iter().all(|ty| shapes::plain_type(ty).as_deref() == Some("u32"))))
}

fn plain_free_call(expression: &Expr) -> bool {
    let Expr::Call(call) = expression else {
        return false;
    };
    call.attrs.is_empty()
        && unqualified_value(&call.func)
        && call.args.iter().all(unqualified_value)
}

fn unqualified_value(expression: &Expr) -> bool {
    let Expr::Path(path) = expression else {
        return false;
    };
    path.path
        .get_ident()
        .is_some_and(|name| crate::source_evidence::plain_value_path(expression, &name.to_string()))
}

pub(super) fn plain_argument(expression: &Expr) -> bool {
    unqualified_value(expression)
        || matches!(expression, Expr::Lit(literal) if literal.attrs.is_empty())
}

fn tuple(function: &syn::ItemFn) -> shapes::DispatchResult<&syn::ExprTuple> {
    let [Stmt::Expr(Expr::Tuple(tuple), None)] = function.block.stmts.as_slice() else {
        return Err(
            if function.block.stmts.len() > 1
                || matches!(
                    function.block.stmts.first(),
                    Some(Stmt::Expr(Expr::Block(_), _))
                )
            {
                shapes::Decision::Unsupported("Wrapper has additional or nested body work")
            } else {
                shapes::Decision::Negative("No exact two-method tuple wrapper")
            },
        );
    };
    if !tuple.attrs.is_empty() {
        return Err(shapes::Decision::Unsupported(
            "Attributed tuple is unavailable",
        ));
    }
    if tuple.elems.len() != 2 {
        return Err(shapes::Decision::Negative(
            "Wrapper requires exactly two original tuple components",
        ));
    }
    Ok(tuple)
}

fn methods(
    request: &shapes::MethodRequest<'_>,
    tuple: &syn::ExprTuple,
) -> shapes::DispatchResult<Vec<MethodWitness>> {
    for expression in &tuple.elems {
        match expression {
            Expr::MethodCall(call) => shapes::capability_call_syntax(call)?,
            expression if plain_free_call(expression) => {}
            _ => {
                return Err(shapes::Decision::Unsupported(
                    "Wrapper tuple component has unsupported or additional work",
                ));
            }
        }
    }
    let methods = tuple
        .elems
        .iter()
        .map(|expression| {
            let Expr::MethodCall(call) = expression else {
                return Err(shapes::Decision::Negative(
                    "Wrapper component is a plain free call rather than a method capability",
                ));
            };
            shapes::capability_method(request, call)
        })
        .collect::<shapes::DispatchResult<Vec<_>>>()?;
    Ok(methods)
}
