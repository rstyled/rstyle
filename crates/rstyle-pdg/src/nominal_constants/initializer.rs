//! Independently specified initializer syntax and integer arithmetic.
use super::{ProductConstant, SourceSelection};

pub(super) fn verify(record: &ProductConstant, source: &SourceSelection<'_>) -> Result<(), String> {
    let syn::Expr::Call(call) = &source.constant.expr else {
        return Err("Product constant requires its original bounded constructor call".into());
    };
    constructor_path(&call.func, record, source)?;
    if !call.attrs.is_empty() || call.args.len() != 1 {
        return Err("Product constant initializer requires one complete argument".into());
    }

    let syn::Expr::Binary(binary) = &call.args[0] else {
        return Err("Product constant initializer requires checked source subtraction".into());
    };
    if !binary.attrs.is_empty() || !matches!(binary.op, syn::BinOp::Sub(_)) {
        return Err("Product constant initializer requires source subtraction".into());
    }

    checked_primitive_cast(
        &binary.left,
        PrimitiveCast {
            target: "usize",
            path: &["core", "i32", "MAX"],
            cast_error: "Product constant requires the original i32-to-usize cast",
            path_error: "Product constant requires the original core scalar dependency",
            mismatch_error: "Product constant initializer has a different core dependency or cast",
        },
    )?;
    let offset = offset(&binary.right)?;
    scalar_observations(record, offset)?;

    let expected = 2_147_483_647u32
        .checked_sub(offset)
        .ok_or("Product constant offset underflows")?;
    if record.field.value != expected.to_string() {
        return Err("Evaluated product constant differs from independent source arithmetic".into());
    }
    Ok(())
}

fn constructor_path(
    expression: &syn::Expr,
    record: &ProductConstant,
    source: &SourceSelection<'_>,
) -> Result<(), String> {
    let syn::Expr::Path(path) = expression else {
        return Err("Product constant requires an original constructor path".into());
    };
    let owner = super::source::owner_name(source.constant_unit.implementation)
        .ok_or("Missing original product constant owner")?;

    let names = path_names(path)?;
    let method = source.constructor.sig.ident.to_string();
    if names != [owner.to_string(), method.clone()]
        || record.constructor.definition.path.rsplit("::").next() != Some(&method)
    {
        return Err("Product constant initializer selects a different original constructor".into());
    }

    Ok(())
}

pub(super) struct PrimitiveCast<'a> {
    pub target: &'a str,
    pub path: &'a [&'a str],
    pub cast_error: &'a str,
    pub path_error: &'a str,
    pub mismatch_error: &'a str,
}

pub(super) fn checked_primitive_cast(
    expression: &syn::Expr,
    expected: PrimitiveCast<'_>,
) -> Result<(), String> {
    let syn::Expr::Cast(cast) = expression else {
        return Err(expected.cast_error.into());
    };
    let syn::Expr::Path(path) = &*cast.expr else {
        return Err(expected.path_error.into());
    };
    if !cast.attrs.is_empty()
        || !crate::source_evidence::plain_type_path(&cast.ty, expected.target)
        || path_names(path)? != expected.path
    {
        return Err(expected.mismatch_error.into());
    }
    Ok(())
}

fn scalar_observations(record: &ProductConstant, offset: u32) -> Result<(), String> {
    let initializer = &record.initializer;
    let dependency = &initializer.core_constant;
    if initializer.offset != offset.to_string()
        || initializer.cast_from != "i32"
        || initializer.cast_to != "usize"
        || initializer.subtraction_type != "usize"
        || dependency.scalar.ty != "i32"
        || dependency.scalar.value != "2147483647"
        || dependency.definition.crate_name != "core"
        || dependency.definition.crate_hash.is_empty()
        || dependency.definition.crate_hash == "local-unencoded"
        || dependency.definition.path.is_empty()
        || dependency.definition.definition_hash.is_empty()
        || dependency.definition.implementation.is_some()
    {
        return Err(
            "Product constant scalar observations disagree with its source contract".into(),
        );
    }
    Ok(())
}

fn offset(expression: &syn::Expr) -> Result<u32, String> {
    let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Int(literal),
        attrs,
        ..
    }) = expression
    else {
        return Err("Product constant offset requires a complete integer literal".into());
    };
    if !attrs.is_empty() || !matches!(literal.suffix(), "" | "usize") {
        return Err("Product constant offset has unsupported attributes or suffix".into());
    }

    let value = literal
        .base10_parse::<u32>()
        .map_err(|error| error.to_string())?;
    if literal.to_string() != format!("{value}{}", literal.suffix()) {
        return Err("Product constant offset requires canonical decimal source".into());
    }
    Ok(value)
}

pub(super) fn path_names(path: &syn::ExprPath) -> Result<Vec<String>, String> {
    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || !path.attrs.is_empty()
        || path
            .path
            .segments
            .iter()
            .any(|segment| !segment.arguments.is_empty())
    {
        return Err("Product constant requires complete ordinary nongeneric paths".into());
    }
    Ok(path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect())
}
