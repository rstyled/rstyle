//! Source shape only: no concrete substitution, nominal, call or purity authority.
use crate::ByteSpan;
use crate::source_evidence::syntax_span as span;
use syn::ext::IdentExt;

#[cfg(test)]
mod tests;

type Result<T> = std::result::Result<T, String>;

/// Original immutable formal spelling and complete source positions.
pub struct Parameter {
    pub name: String,
    pub name_span: ByteSpan,
    pub ty_span: ByteSpan,
}

/// Original length comparison roles; no resolved operator authority.
pub struct Condition {
    pub expression: ByteSpan,
    pub length: ByteSpan,
    pub length_method: ByteSpan,
    pub operator: ByteSpan,
    pub receiver: ByteSpan,
    pub minimum: ByteSpan,
}

/// Complete original short-input return expression and constructor roles.
pub struct ErrorBranch {
    pub block: ByteSpan,
    pub returned: ByteSpan,
    pub err: ByteSpan,
    pub err_callee: ByteSpan,
    pub constructor: ByteSpan,
    pub constructor_callee: ByteSpan,
    pub what: ByteSpan,
}

/// Complete original unit-success construction roles.
pub struct Success {
    pub expression: ByteSpan,
    pub callee: ByteSpan,
    pub unit: ByteSpan,
}

/// A mutable source-shape description, never trusted instance or effect evidence.
/// Consumers must independently verify HIR correspondence and all call/type
/// authorities; possession or mutation of this record grants no admission.
pub struct Checked {
    pub function: String,
    pub function_name: ByteSpan,
    pub generic: String,
    pub generic_span: ByteSpan,
    pub signature: ByteSpan,
    pub body: ByteSpan,
    pub output: ByteSpan,
    pub parameters: [Parameter; 3],
    pub result_path: syn::Path,
    pub error_path: syn::Path,
    pub constructor_path: syn::Path,
    pub selection: ByteSpan,
    pub condition: Condition,
    pub error: ErrorBranch,
    pub success: Success,
}

/// Parse the entire ordinary bounded family without granting semantic authority.
pub fn verify(bytes: &str) -> Result<Checked> {
    if bytes.len() > crate::concrete_types::MAX_SOURCE_BYTES {
        return Err("Slice length family exceeds the bounded source budget".into());
    }
    let syntax: syn::ItemFn = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let generic = signature(&syntax)?;
    let parameters = parameters(bytes, &syntax.sig, &generic)?;
    let (result_path, error_path) = result(&syntax.sig.output)?;

    let [
        syn::Stmt::Expr(syn::Expr::If(selection), None),
        syn::Stmt::Expr(success, None),
    ] = syntax.block.stmts.as_slice()
    else {
        return Err("Slice length family requires one if and one complete Ok tail".into());
    };
    body_attributes(&syntax.block)?;
    let condition = condition(bytes, selection, &parameters)?;
    let (error, constructor_path) = failure(bytes, selection, &parameters[2].name, &error_path)?;
    let success = success_value(bytes, success)?;

    Ok(Checked {
        function: syntax.sig.ident.unraw().to_string(),
        function_name: span(bytes, &syntax.sig.ident),
        generic,
        generic_span: span(bytes, &syntax.sig.generics.params[0]),
        signature: span(bytes, &syntax.sig),
        body: span(bytes, syntax.block.as_ref()),
        output: span(bytes, &syntax.sig.output),
        parameters,
        result_path,
        error_path,
        constructor_path,
        selection: span(bytes, selection),
        condition,
        error,
        success,
    })
}

fn signature(function: &syn::ItemFn) -> Result<String> {
    let signature = &function.sig;
    if !crate::source_evidence::literal_metadata(&function.attrs, "inline")
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || signature.generics.where_clause.is_some()
        || signature.inputs.len() != 3
    {
        return Err("Slice length family requires an ordinary free three-formal function".into());
    }

    let generic_parameters = signature.generics.params.iter().collect::<Vec<_>>();
    let [syn::GenericParam::Type(parameter)] = generic_parameters.as_slice() else {
        return Err("Slice length family requires one unconstrained type parameter".into());
    };
    if !parameter.attrs.is_empty()
        || parameter.colon_token.is_some()
        || !parameter.bounds.is_empty()
        || parameter.eq_token.is_some()
        || parameter.default.is_some()
    {
        return Err("Slice element parameter permits only implicit Sized".into());
    }
    Ok(parameter.ident.unraw().to_string())
}

fn parameters(bytes: &str, signature: &syn::Signature, generic: &str) -> Result<[Parameter; 3]> {
    let arguments: Vec<_> = signature
        .inputs
        .iter()
        .map(parameter)
        .collect::<Result<_>>()?;
    let [slice, minimum, what] = arguments.as_slice() else {
        return Err("Slice length family requires all three original formals".into());
    };
    slice_type(&slice.1.ty, generic)?;
    if !crate::source_evidence::plain_type_path(&minimum.1.ty, "usize") {
        return Err("Minimum length requires builtin usize source spelling".into());
    }
    static_string(&what.1.ty)?;

    let values = arguments
        .iter()
        .map(|(ident, argument)| Parameter {
            name: ident.unraw().to_string(),
            name_span: span(bytes, argument.pat.as_ref()),
            ty_span: span(bytes, argument.ty.as_ref()),
        })
        .collect::<Vec<_>>();
    if values
        .iter()
        .map(|value| &value.name)
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != 3
    {
        return Err("Slice length family cannot shadow a formal binding".into());
    }
    values
        .try_into()
        .map_err(|_| "Slice length formal inventory changed".into())
}

fn parameter(argument: &syn::FnArg) -> Result<(&syn::Ident, &syn::PatType)> {
    let syn::FnArg::Typed(argument) = argument else {
        return Err("Slice length family cannot have a receiver".into());
    };
    let syn::Pat::Ident(pattern) = argument.pat.as_ref() else {
        return Err("Slice length formals require plain identifier bindings".into());
    };
    if !argument.attrs.is_empty()
        || !pattern.attrs.is_empty()
        || pattern.by_ref.is_some()
        || pattern.mutability.is_some()
        || pattern.subpat.is_some()
    {
        return Err("Slice length formals must be immutable, unattributed and unprojected".into());
    }
    Ok((&pattern.ident, argument))
}

fn slice_type(ty: &syn::Type, generic: &str) -> Result<()> {
    let syn::Type::Reference(reference) = ty else {
        return Err("Slice formal requires an elided shared reference".into());
    };
    let syn::Type::Slice(slice) = reference.elem.as_ref() else {
        return Err("Slice formal requires the complete generic slice".into());
    };
    if reference.lifetime.is_some()
        || reference.mutability.is_some()
        || !crate::source_evidence::plain_type_path(&slice.elem, generic)
    {
        return Err("Slice formal changes its shared generic element".into());
    }
    Ok(())
}

fn static_string(ty: &syn::Type) -> Result<()> {
    if !matches!(ty, syn::Type::Reference(reference)
        if reference.mutability.is_none()
            && reference.lifetime.as_ref().is_some_and(|lifetime| lifetime.ident == "static")
            && crate::source_evidence::plain_type_path(&reference.elem, "str"))
    {
        return Err("Error message requires an explicit static shared str".into());
    }
    Ok(())
}

fn result(output: &syn::ReturnType) -> Result<(syn::Path, syn::Path)> {
    let syn::ReturnType::Type(_, ty) = output else {
        return Err("Missing complete Result output".into());
    };
    let syn::Type::Path(result) = ty.as_ref() else {
        return Err("Output must be a Result path".into());
    };
    let arguments = result_path(result)?;

    let result_arguments = arguments.args.iter().collect::<Vec<_>>();
    let [
        syn::GenericArgument::Type(syn::Type::Tuple(unit)),
        syn::GenericArgument::Type(syn::Type::Path(error)),
    ] = result_arguments.as_slice()
    else {
        return Err("Result requires unit success and a nominal error path".into());
    };
    if !unit.elems.is_empty() || error.qself.is_some() || arguments.colon2_token.is_some() {
        return Err("Slice length Result changes unit success or error qualification".into());
    }
    plain_path(&error.path)?;
    Ok((result.path.clone(), error.path.clone()))
}

fn result_path(result: &syn::TypePath) -> Result<&syn::AngleBracketedGenericArguments> {
    let segments: Vec<_> = result
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.unraw().to_string())
        .collect();
    if result.qself.is_some()
        || !matches!(
            segments.join("::").as_str(),
            "Result" | "core::result::Result" | "std::result::Result"
        )
        || result
            .path
            .segments
            .iter()
            .rev()
            .skip(1)
            .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
    {
        return Err("Output changes the direct Result source path".into());
    }

    let syn::PathArguments::AngleBracketed(arguments) = &result
        .path
        .segments
        .last()
        .ok_or("Missing Result path")?
        .arguments
    else {
        return Err("Result must specify complete success and error types".into());
    };
    Ok(arguments)
}

fn condition(
    bytes: &str,
    selection: &syn::ExprIf,
    parameters: &[Parameter; 3],
) -> Result<Condition> {
    let syn::Expr::Binary(condition) = selection.cond.as_ref() else {
        return Err("Slice length condition must compare exact length and minimum".into());
    };
    let syn::Expr::MethodCall(length) = condition.left.as_ref() else {
        return Err("Slice length condition must evaluate the original len receiver".into());
    };
    if selection.else_branch.is_some()
        || !matches!(condition.op, syn::BinOp::Lt(_))
        || length.method != "len"
        || !length.args.is_empty()
        || length.turbofish.is_some()
        || !crate::source_evidence::plain_value_path(&length.receiver, &parameters[0].name)
        || !crate::source_evidence::plain_value_path(&condition.right, &parameters[1].name)
    {
        return Err(
            "Slice length condition changes comparison, evaluation or binding roles".into(),
        );
    }
    Ok(Condition {
        expression: span(bytes, condition),
        length: span(bytes, length),
        length_method: span(bytes, &length.method),
        operator: span(bytes, &condition.op),
        receiver: span(bytes, length.receiver.as_ref()),
        minimum: span(bytes, condition.right.as_ref()),
    })
}

fn failure(
    bytes: &str,
    selection: &syn::ExprIf,
    what: &str,
    owner: &syn::Path,
) -> Result<(ErrorBranch, syn::Path)> {
    let [syn::Stmt::Expr(syn::Expr::Return(returned), Some(_))] =
        selection.then_branch.stmts.as_slice()
    else {
        return Err("Short branch must return exactly one fresh error without omitted work".into());
    };
    let err = call(
        returned
            .expr
            .as_deref()
            .ok_or("Missing original returned error")?,
        "Err",
    )?;
    let syn::Expr::Call(constructor) = &err.args[0] else {
        return Err("Error payload must be an original constructor call".into());
    };
    let syn::Expr::Path(callee) = constructor.func.as_ref() else {
        return Err("Error constructor requires a direct source path".into());
    };
    if callee.qself.is_some()
        || constructor.args.len() != 1
        || !crate::source_evidence::plain_value_path(&constructor.args[0], what)
    {
        return Err("Error constructor changes its sole original message argument".into());
    }
    plain_path(&callee.path)?;
    let mut actual_owner = callee.path.clone();
    actual_owner.segments.pop();
    if !same_path(&actual_owner, owner) {
        return Err("Error constructor source owner differs from output error".into());
    }

    Ok((
        ErrorBranch {
            block: span(bytes, &selection.then_branch),
            returned: span(bytes, returned),
            err: span(bytes, err),
            err_callee: span(bytes, err.func.as_ref()),
            constructor: span(bytes, constructor),
            constructor_callee: span(bytes, constructor.func.as_ref()),
            what: span(bytes, &constructor.args[0]),
        },
        callee.path.clone(),
    ))
}

fn success_value(bytes: &str, source: &syn::Expr) -> Result<Success> {
    let success = call(source, "Ok")?;
    if !matches!(&success.args[0], syn::Expr::Tuple(unit) if unit.elems.is_empty()) {
        return Err("Success must construct the complete empty unit value".into());
    }
    Ok(Success {
        expression: span(bytes, success),
        callee: span(bytes, success.func.as_ref()),
        unit: span(bytes, &success.args[0]),
    })
}

fn call<'a>(source: &'a syn::Expr, name: &str) -> Result<&'a syn::ExprCall> {
    match source {
        syn::Expr::Call(call)
            if call.args.len() == 1
                && crate::source_evidence::plain_value_path(&call.func, name) =>
        {
            Ok(call)
        }
        _ => Err("Slice length Result requires one original direct variant argument".into()),
    }
}

fn plain_path(path: &syn::Path) -> Result<()> {
    if path.segments.is_empty()
        || path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
    {
        return Err(
            "Slice length nominal paths cannot contain generic or associated substitution".into(),
        );
    }
    Ok(())
}

fn same_path(left: &syn::Path, right: &syn::Path) -> bool {
    left.leading_colon.is_some() == right.leading_colon.is_some()
        && left
            .segments
            .iter()
            .map(|segment| segment.ident.unraw().to_string())
            .eq(right
                .segments
                .iter()
                .map(|segment| segment.ident.unraw().to_string()))
}

fn body_attributes(block: &syn::Block) -> Result<()> {
    if crate::source_evidence::has_body_attributes(block) {
        return Err(
            "Slice length body cannot contain attributed or conditionally omitted operations"
                .into(),
        );
    }
    Ok(())
}
