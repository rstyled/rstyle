//! Whole original widening-reader syntax; selected definitions require separate proof.
use crate::source_evidence::{associated_owner, plain_type_path, plain_value_path};

#[cfg(test)]
mod tests;

pub(crate) struct Syntax<'a> {
    pub width: crate::native_reader_width::Width,
    pub formals: [&'a syn::PatIdent; 2],
    pub and_then: &'a syn::ExprMethodCall,
    pub reader: &'a syn::ExprCall,
    pub continuation: &'a syn::ExprClosure,
    pub decoded: &'a syn::PatIdent,
    pub count: &'a syn::PatIdent,
    pub map: &'a syn::ExprMethodCall,
    pub success: &'a syn::ExprClosure,
    pub converted: &'a syn::PatIdent,
    pub conversion: &'a syn::ExprCall,
    pub tuple: &'a syn::ExprTuple,
    pub map_err: &'a syn::ExprMethodCall,
    pub failure: &'a syn::ExprClosure,
    pub constructor: &'a syn::ExprCall,
}

pub(crate) fn syntax(function: &syn::ItemFn) -> Result<Syntax<'_>, String> {
    let formals = interface(function)?;

    let [syn::Stmt::Expr(tail, None)] = function.block.stmts.as_slice() else {
        return Err("Widening reader requires exactly one original tail".into());
    };

    let and_then = method(tail, "and_then")?;

    let reader = call(&and_then.receiver)?;
    reader_arguments(reader, formals)?;
    let width = if plain_value_path(&reader.func, "try_read_u16") {
        crate::native_reader_width::Width::U16
    } else {
        crate::native_reader_width::Width::U32
    };

    let continuation = closure(&and_then.args[0])?;

    let (decoded, count) = payload(continuation)?;

    if decoded.ident == formals[1].ident || count.ident == formals[1].ident {
        return Err("Widening payload shadows its captured static message".into());
    }

    let map_err = method(continuation_tail(continuation)?, "map_err")?;

    let failure = closure(&map_err.args[0])?;

    let constructor = error_constructor(failure, formals[1])?;

    let syn::Expr::Path(path) = constructor.func.as_ref() else {
        return Err("Widening reader error requires an original associated call".into());
    };

    let error_owner = associated_owner(path, "invalid_usize", "Invalid widening error owner")?;
    output(function, &error_owner)?;

    let map = method(&map_err.receiver, "map")?;

    let success = closure(&map.args[0])?;

    let converted = binding(&success.inputs[0])?;

    let tuple = success_tuple(success, converted, count)?;

    let conversion = conversion(&map.receiver, decoded)?;

    Ok(Syntax {
        width,
        formals,
        and_then,
        reader,
        continuation,
        decoded,
        count,
        map,
        success,
        converted,
        conversion,
        tuple,
        map_err,
        failure,
        constructor,
    })
}

fn interface(function: &syn::ItemFn) -> Result<[&syn::PatIdent; 2], String> {
    let signature = &function.sig;

    if !function.attrs.is_empty()
        || crate::source_evidence::has_body_attributes(&function.block)
        || signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
    {
        return Err("Widening reader requires a plain concrete interface".into());
    }

    let arguments: Vec<_> = signature.inputs.iter().collect();

    let [slice, message] = arguments.as_slice() else {
        return Err("Widening reader requires exactly two formals".into());
    };

    let (slice, slice_type) = formal(slice)?;

    let (message, message_type) = formal(message)?;

    if slice.ident == message.ident || !slice_reference(slice_type) || !static_message(message_type)
    {
        return Err(
            "Widening reader requires distinct byte-slice and static-message formals".into(),
        );
    }

    Ok([slice, message])
}

fn formal(argument: &syn::FnArg) -> Result<(&syn::PatIdent, &syn::Type), String> {
    let syn::FnArg::Typed(argument) = argument else {
        return Err("Widening reader cannot have a receiver".into());
    };

    if !argument.attrs.is_empty() {
        return Err("Widening reader cannot have attributed formals".into());
    }

    Ok((binding(&argument.pat)?, &argument.ty))
}

fn binding(pattern: &syn::Pat) -> Result<&syn::PatIdent, String> {
    let syn::Pat::Ident(pattern) = pattern else {
        return Err("Widening reader requires a plain binding".into());
    };

    if !pattern.attrs.is_empty()
        || pattern.by_ref.is_some()
        || pattern.mutability.is_some()
        || pattern.subpat.is_some()
    {
        return Err("Widening reader binding has unsupported work".into());
    }

    Ok(pattern)
}

fn slice_reference(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Reference(reference)
        if reference.lifetime.is_none() && reference.mutability.is_none()
        && matches!(reference.elem.as_ref(), syn::Type::Slice(slice)
            if plain_type_path(&slice.elem, "u8")))
}

fn static_message(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Reference(reference)
        if reference.mutability.is_none()
        && reference.lifetime.as_ref().is_some_and(|value| value.ident == "static")
        && plain_type_path(&reference.elem, "str"))
}

fn method<'a>(expression: &'a syn::Expr, name: &str) -> Result<&'a syn::ExprMethodCall, String> {
    let syn::Expr::MethodCall(method) = expression else {
        return Err("Widening reader requires an original method call".into());
    };

    if !method.attrs.is_empty()
        || method.method != name
        || method.turbofish.is_some()
        || method.args.len() != 1
    {
        return Err("Widening reader changes method, arguments or generic qualification".into());
    }

    Ok(method)
}

fn call(expression: &syn::Expr) -> Result<&syn::ExprCall, String> {
    match expression {
        syn::Expr::Call(call) if call.attrs.is_empty() => Ok(call),
        _ => Err("Widening reader requires a plain original call".into()),
    }
}

fn closure(expression: &syn::Expr) -> Result<&syn::ExprClosure, String> {
    let syn::Expr::Closure(closure) = expression else {
        return Err("Widening reader requires an immediate closure".into());
    };

    if !closure.attrs.is_empty()
        || closure.lifetimes.is_some()
        || closure.constness.is_some()
        || closure.movability.is_some()
        || closure.asyncness.is_some()
        || closure.capture.is_some()
        || closure.inputs.len() != 1
        || !matches!(closure.output, syn::ReturnType::Default)
    {
        return Err("Widening reader requires a plain inferred shared-capture callback".into());
    }

    Ok(closure)
}

fn payload(closure: &syn::ExprClosure) -> Result<(&syn::PatIdent, &syn::PatIdent), String> {
    let syn::Pat::Tuple(tuple) = &closure.inputs[0] else {
        return Err("Widening continuation requires the complete decoded/count tuple".into());
    };

    if !tuple.attrs.is_empty() || tuple.elems.len() != 2 {
        return Err("Widening continuation changes its payload shape".into());
    }

    let decoded = binding(&tuple.elems[0])?;

    let count = binding(&tuple.elems[1])?;

    if decoded.ident == count.ident {
        return Err("Widening continuation reuses its payload binding".into());
    }

    Ok((decoded, count))
}

fn continuation_tail(closure: &syn::ExprClosure) -> Result<&syn::Expr, String> {
    let syn::Expr::Block(block) = closure.body.as_ref() else {
        return Err("Widening continuation requires its complete original block".into());
    };

    let [syn::Stmt::Expr(tail, None)] = block.block.stmts.as_slice() else {
        return Err("Widening continuation has additional or missing work".into());
    };

    if !block.attrs.is_empty() || block.label.is_some() {
        return Err("Widening continuation has attributed or labeled work".into());
    }

    Ok(tail)
}

fn reader_arguments(reader: &syn::ExprCall, formals: [&syn::PatIdent; 2]) -> Result<(), String> {
    if !(plain_value_path(&reader.func, "try_read_u32")
        || plain_value_path(&reader.func, "try_read_u16"))
        || reader.args.len() != 2
        || !read(&reader.args[0], formals[0])
        || !read(&reader.args[1], formals[1])
    {
        return Err("Widening reader changes its complete checked-reader inputs".into());
    }

    Ok(())
}

fn read(expression: &syn::Expr, binding: &syn::PatIdent) -> bool {
    plain_value_path(expression, &binding.ident.to_string())
}

fn conversion<'a>(
    expression: &'a syn::Expr,
    decoded: &syn::PatIdent,
) -> Result<&'a syn::ExprCall, String> {
    let conversion = call(expression)?;

    let syn::Expr::Path(path) = conversion.func.as_ref() else {
        return Err("Widening conversion requires an associated call".into());
    };

    let owner = associated_owner(path, "try_from", "Invalid widening conversion path")?;

    if !owner.path.is_ident("usize")
        || conversion.args.len() != 1
        || !read(&conversion.args[0], decoded)
    {
        return Err("Widening conversion changes its type or decoded input".into());
    }

    Ok(conversion)
}

fn success_tuple<'a>(
    closure: &'a syn::ExprClosure,
    converted: &syn::PatIdent,
    count: &syn::PatIdent,
) -> Result<&'a syn::ExprTuple, String> {
    let syn::Expr::Tuple(tuple) = closure.body.as_ref() else {
        return Err("Widening success requires the complete converted/count tuple".into());
    };

    if !tuple.attrs.is_empty()
        || tuple.elems.len() != 2
        || converted.ident == count.ident
        || !read(&tuple.elems[0], converted)
        || !read(&tuple.elems[1], count)
    {
        return Err("Widening success changes its converted value or saved count".into());
    }

    Ok(tuple)
}

fn error_constructor<'a>(
    closure: &'a syn::ExprClosure,
    message: &syn::PatIdent,
) -> Result<&'a syn::ExprCall, String> {
    if !matches!(&closure.inputs[0], syn::Pat::Wild(wild) if wild.attrs.is_empty()) {
        return Err("Widening error callback must discard exactly its conversion error".into());
    }

    let constructor = call(&closure.body)?;

    if constructor.args.len() != 1 || !read(&constructor.args[0], message) {
        return Err("Widening error callback changes its original static message".into());
    }

    Ok(constructor)
}

fn output(function: &syn::ItemFn, owner: &syn::TypePath) -> Result<(), String> {
    let syn::ReturnType::Type(_, ty) = &function.sig.output else {
        return Err("Widening reader requires an explicit Result".into());
    };

    let syn::Type::Path(path) = ty.as_ref() else {
        return Err("Widening reader output is not a plain Result".into());
    };

    let segments = &path.path.segments;

    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || segments.len() != 1
        || segments[0].ident != "Result"
    {
        return Err("Widening reader output changes Result qualification".into());
    }

    let syn::PathArguments::AngleBracketed(arguments) = &segments[0].arguments else {
        return Err("Widening reader Result lacks complete arguments".into());
    };

    let values: Vec<_> = arguments.args.iter().collect();

    let [
        syn::GenericArgument::Type(syn::Type::Tuple(tuple)),
        syn::GenericArgument::Type(syn::Type::Path(error)),
    ] = values.as_slice()
    else {
        return Err("Widening reader requires its complete success and error types".into());
    };

    if arguments.colon2_token.is_some()
        || tuple.elems.len() != 2
        || !tuple.elems.iter().all(|ty| plain_type_path(ty, "usize"))
        || error.qself.is_some()
        || crate::source_evidence::plain_relative_path(&error.path)
            != crate::source_evidence::plain_relative_path(&owner.path)
    {
        return Err("Widening reader output changes its count or error owner".into());
    }

    Ok(())
}
