//! Source-complete one-field const constructor; selection identity is checked externally.
use super::{BuildContext, ProductConstant, SourceSelection};
use syn::spanned::Spanned;

pub(super) fn verify(
    record: &ProductConstant,
    source: &SourceSelection<'_>,
    build: &BuildContext<'_>,
) -> Result<(), String> {
    let unit = &source.constructor_unit;
    let method = source.constructor;
    super::source::implementation(unit, &record.constructor.implementation_source, build)?;
    super::source::attributes(&method.attrs, build, true)?;
    let start = if matches!(method.vis, syn::Visibility::Inherited) {
        method.sig.span()
    } else {
        method.vis.span()
    };
    let span = crate::ByteSpan {
        start: super::source::range(unit.bytes, start).start,
        end: super::source::range(unit.bytes, method.block.span()).end,
    };
    super::source::site(unit, &record.constructor.declaration, span)?;
    let parameter = signature(method, unit.implementation)?;
    if super::source::range(unit.bytes, parameter.span()) != record.constructor.parameter_span {
        return Err("Product constant constructor parameter source disagrees".into());
    }
    if !unit.implementation.items.iter().any(|item| {
        matches!(item, syn::ImplItem::Fn(item)
            if super::source::range(unit.bytes, item.span())
                == super::source::range(unit.bytes, method.span()))
    }) {
        return Err("Product constant constructor is absent from original impl".into());
    }

    unique_active(source, build)?;
    body(source, parameter)
}

fn signature<'a>(
    method: &'a syn::ImplItemFn,
    implementation: &syn::ItemImpl,
) -> Result<&'a syn::Ident, String> {
    let signature = &method.sig;
    if signature.constness.is_none()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || method.defaultness.is_some()
        || signature.inputs.len() != 1
    {
        return Err("Product constant requires a safe nongeneric const constructor".into());
    }
    let syn::ReturnType::Type(_, result) = &signature.output else {
        return Err("Product constant constructor lacks its original product return type".into());
    };
    if !super::source::same_owner_type(result, &implementation.self_ty) {
        return Err("Product constant constructor returns a different original owner".into());
    }

    parameter(&signature.inputs[0])
}

fn parameter(argument: &syn::FnArg) -> Result<&syn::Ident, String> {
    let syn::FnArg::Typed(argument) = argument else {
        return Err("Product constant constructor cannot have a receiver".into());
    };
    let syn::Pat::Ident(pattern) = &*argument.pat else {
        return Err("Product constant constructor requires an unadorned parameter".into());
    };
    if !argument.attrs.is_empty()
        || !pattern.attrs.is_empty()
        || pattern.by_ref.is_some()
        || pattern.mutability.is_some()
        || pattern.subpat.is_some()
        || !crate::source_evidence::plain_type_path(&argument.ty, "usize")
    {
        return Err("Product constant constructor requires one immutable usize parameter".into());
    }
    Ok(&pattern.ident)
}

fn body(source: &SourceSelection<'_>, parameter: &syn::Ident) -> Result<(), String> {
    let [syn::Stmt::Expr(syn::Expr::Call(call), None)] = source.constructor.block.stmts.as_slice()
    else {
        return Err("Product constant constructor must be one complete tuple construction".into());
    };
    let syn::Expr::Path(path) = &*call.func else {
        return Err("Product constant constructor requires the selected tuple constructor".into());
    };
    let owner = super::source::owner_name(source.constructor_unit.implementation)
        .ok_or("Missing original product constructor owner")?;
    if !call.attrs.is_empty()
        || call.args.len() != 1
        || super::initializer::path_names(path)? != [owner.to_string()]
    {
        return Err("Product constant constructor has a different original product".into());
    }

    super::initializer::checked_primitive_cast(
        &call.args[0],
        super::initializer::PrimitiveCast {
            target: "u32",
            path: &[&parameter.to_string()],
            cast_error: "Product constant constructor requires its scalar field cast",
            path_error: "Product constant constructor field must consume its parameter",
            mismatch_error: "Product constant constructor field differs from its original parameter cast",
        },
    )
}

fn unique_active(source: &SourceSelection<'_>, build: &BuildContext<'_>) -> Result<(), String> {
    let configuration = crate::conditional_source::Configuration::new(build.cfgs);
    crate::source_evidence::unique_active(
        &source.constructor_unit.implementation.items,
        |item| {
            if let syn::ImplItem::Fn(method) = item
                && method.sig.ident == source.constructor.sig.ident
            {
                configuration.active(&method.attrs)
            } else {
                Ok(false)
            }
        },
        "Product constant constructor lacks unique active original source",
    )
}
