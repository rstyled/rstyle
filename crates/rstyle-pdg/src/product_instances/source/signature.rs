//! Original binders and lexical nominal joins, independent of displayed type names.
use super::*;
use crate::concrete_types::Nominal;

type Paths = Vec<(syn::TypePath, Nominal)>;

pub(super) fn verify(facts: &HirFnFacts, syntax: &syn::ItemFn, result: &TypeTree) -> Result<()> {
    metadata(facts, syntax)?;

    let generic = generic(facts, syntax)?;
    let (product, pattern, converted) = products(result)?;
    let mut paths = formals(facts, syntax, pattern)?;
    result_paths(facts, syntax, product, &mut paths)?;
    converted_paths(syntax, generic, converted, &mut paths)?;
    crate::nominal_boundary::resolution::verify_reference_paths(facts, &paths)
}

fn metadata(facts: &HirFnFacts, syntax: &syn::ItemFn) -> Result<()> {
    let sig = &facts.signature;

    if !syntax.attrs.is_empty()
        || syntax.sig.ident != "new"
        || syntax.sig.constness.is_some()
        || syntax.sig.asyncness.is_some()
        || syntax.sig.unsafety.is_some()
        || syntax.sig.abi.is_some()
        || syntax.sig.variadic.is_some()
        || syntax.sig.generics.where_clause.is_some()
        || syntax.sig.inputs.len() != 2
        || sig.params.len() != 2
        || sig.has_self
        || sig.receiver.is_some()
        || sig.is_async
        || sig.is_const
        || sig.is_unsafe
        || sig.is_variadic
        || sig.abi.known_value() != Some("Rust")
        || sig.sig_span
            != (crate::ByteSpan {
                start: 0,
                end: span(facts, &syntax.sig).end,
            })
    {
        return Err("Product original constructor signature differs".into());
    }
    Ok(())
}

fn generic<'a>(facts: &HirFnFacts, syntax: &'a syn::ItemFn) -> Result<&'a syn::TypeParam> {
    if syntax.sig.generics.params.len() != 1 {
        return Err("Product constructor requires exactly one original generic".into());
    }
    let generic = &syntax.sig.generics.params[0];

    let syn::GenericParam::Type(generic) = generic else {
        return Err("Product constructor generic is not a type".into());
    };
    generic_bound(facts, generic)?;
    Ok(generic)
}

fn formals(facts: &HirFnFacts, syntax: &syn::ItemFn, pattern: &Nominal) -> Result<Paths> {
    let sig = &facts.signature;

    let mut paths = Vec::new();
    for (index, (input, name)) in syntax
        .sig
        .inputs
        .iter()
        .zip(["pattern", "span"])
        .enumerate()
    {
        let syn::FnArg::Typed(input) = input else {
            return Err("Product formal cannot be self".into());
        };
        let syn::Pat::Ident(binding) = &*input.pat else {
            return Err("Product formal requires one binding".into());
        };
        binding_source(binding, name)?;
        let param = &sig.params[index];
        binding_node(facts, &param.pat, binding, Some(index as u32))?;
        if !input.attrs.is_empty()
            || param.position != index as u32
            || param.symbol != param.pat.binding.as_deref().unwrap_or("")
            || param.name_span != span(facts, &binding.ident)
            || param.ty_span != span(facts, &input.ty)
            || param.ty.spelling
                != crate::source_evidence::byte_range(&facts.source.bytes, param.ty_span)?
        {
            return Err("Product formal metadata changes its original declaration".into());
        }
        let path = type_path(&input.ty)?;
        if index == 0 {
            paths.push((path.clone(), pattern.clone()));
        } else if !path.path.is_ident("S") {
            return Err("Product input must retain original S".into());
        }
    }
    Ok(paths)
}

fn result_paths(
    facts: &HirFnFacts,
    syntax: &syn::ItemFn,
    product: &Nominal,
    paths: &mut Paths,
) -> Result<()> {
    let sig = &facts.signature;

    let syn::ReturnType::Type(_, ty) = &syntax.sig.output else {
        return Err("Product result annotation missing".into());
    };
    if sig.return_ty_span != span(facts, ty)
        || sig.return_ty.spelling
            != crate::source_evidence::byte_range(&facts.source.bytes, sig.return_ty_span)?
    {
        return Err("Product result metadata differs from source".into());
    }
    paths.push((type_path(ty)?.clone(), product.clone()));
    let Some(syn::Stmt::Expr(syn::Expr::Struct(value), None)) = syntax.block.stmts.last() else {
        return Err("Product result structure absent".into());
    };
    paths.push((
        syn::TypePath {
            qself: None,
            path: value.path.clone(),
        },
        product.clone(),
    ));
    Ok(())
}

fn converted_paths(
    syntax: &syn::ItemFn,
    generic: &syn::TypeParam,
    converted: &Nominal,
    paths: &mut Paths,
) -> Result<()> {
    let Some(syn::Stmt::Local(local)) = syntax.block.stmts.first() else {
        return Err("Product converted binding absent".into());
    };

    let syn::Pat::Type(local) = &local.pat else {
        return Err("Product converted type missing".into());
    };
    paths.push((type_path(&local.ty)?.clone(), converted.clone()));
    let syn::TypeParamBound::Trait(bound) = &generic.bounds[0] else {
        unreachable!()
    };
    let syn::PathArguments::AngleBracketed(arguments) = &bound.path.segments[0].arguments else {
        unreachable!()
    };
    let syn::GenericArgument::Type(ty) = &arguments.args[0] else {
        unreachable!()
    };
    paths.push((type_path(ty)?.clone(), converted.clone()));
    Ok(())
}

fn generic_bound(facts: &HirFnFacts, generic: &syn::TypeParam) -> Result<()> {
    let [captured] = facts.signature.generics.as_slice() else {
        return Err("Product generic roster differs".into());
    };

    if generic.ident != "S"
        || !generic.attrs.is_empty()
        || generic.default.is_some()
        || generic.bounds.len() != 1
        || captured.name != "S"
        || captured.span != span(facts, &generic.ident)
    {
        return Err("Product original generic declaration differs".into());
    }
    let syn::TypeParamBound::Trait(bound) = &generic.bounds[0] else {
        return Err("Product Into bound absent".into());
    };
    if bound.paren_token.is_some()
        || bound.lifetimes.is_some()
        || !matches!(bound.modifier, syn::TraitBoundModifier::None)
        || bound.path.leading_colon.is_some()
        || bound.path.segments.len() != 1
        || bound.path.segments[0].ident != "Into"
    {
        return Err("Product original Into bound differs".into());
    }
    let syn::PathArguments::AngleBracketed(args) = &bound.path.segments[0].arguments else {
        return Err("Product Into arguments absent".into());
    };
    if args.colon2_token.is_some()
        || args.args.len() != 1
        || !matches!(&args.args[0], syn::GenericArgument::Type(_))
    {
        return Err("Product Into arguments differ".into());
    }
    Ok(())
}

fn products(result: &TypeTree) -> Result<(&Nominal, &Nominal, &Nominal)> {
    let TypeTree::Struct {
        nominal, fields, ..
    } = result
    else {
        return Err("Product result is not a concrete struct".into());
    };

    let [pattern, span] = fields.as_slice() else {
        return Err("Product result field roster differs".into());
    };
    if pattern.position != 0
        || pattern.name != "pattern"
        || span.position != 1
        || span.name != "span"
    {
        return Err("Product result fields reorder original values".into());
    }
    let (
        TypeTree::Struct {
            nominal: pattern, ..
        },
        TypeTree::Struct { nominal: span, .. },
    ) = (&pattern.ty, &span.ty)
    else {
        return Err("Product components lack captured nominal structure".into());
    };
    Ok((nominal, pattern, span))
}

fn type_path(ty: &syn::Type) -> Result<&syn::TypePath> {
    match ty {
        syn::Type::Path(path) if path.qself.is_none() => Ok(path),
        _ => Err("Product nominal source path is unsupported".into()),
    }
}

pub(super) fn binding_source(binding: &syn::PatIdent, name: &str) -> Result<()> {
    if binding.ident != name
        || !binding.attrs.is_empty()
        || binding.by_ref.is_some()
        || binding.mutability.is_some()
        || binding.subpat.is_some()
    {
        return Err("Product binding source changes ownership".into());
    }
    Ok(())
}

pub(super) fn binding_node(
    facts: &HirFnFacts,
    binding: &HirNode,
    syntax: &syn::PatIdent,
    parameter: Option<u32>,
) -> Result<()> {
    node(
        facts,
        binding,
        syntax,
        Shape {
            kind: "pat_binding",
            children: 0,
            typed: true,
        },
    )?;

    if binding.param_index != parameter
        || binding.binding.as_ref().is_none_or(String::is_empty)
        || binding.binding_mut != Some(false)
        || binding.res.is_some()
        || binding.dispatch.is_some()
        || binding.expansion.is_some()
        || binding.lit.is_some()
    {
        return Err("Product binding HIR changes original binding semantics".into());
    }
    Ok(())
}
