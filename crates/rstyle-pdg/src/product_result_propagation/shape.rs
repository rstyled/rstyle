//! Observed nominal spellings corroborate exact captured tokens, never lifetimes.
use super::*;
use capture::Shape;

/// Generated signatures use the input's anonymous regions. Actual backing
/// lifetimes remain obligations of source/owner and guarded helper origins.
pub(super) fn generated_output(shape: &Shape) -> Result<String, String> {
    let input = nominal_path(&shape.payload, true)?;
    let output = nominal_path(&shape.output_payload, false)?;
    if input != output {
        return Err("Product propagation changes its nominal success or lifetime arity".into());
    }

    let path = ["core::result::Result", "std::result::Result"]
        .into_iter()
        .find(|path| {
            shape.input == format!("{path}<{}, {}>", shape.payload, shape.error)
                && shape.output == format!("{path}<{}, {}>", shape.output_payload, shape.error)
        })
        .ok_or("Product propagation lacks exact core input/output Result shapes")?;

    let residual = format!("{path}<!, {}>", shape.error);
    if shape.residual != residual
        || !["core::ops::ControlFlow", "std::ops::ControlFlow"]
            .iter()
            .any(|path| shape.control_flow == format!("{path}<{residual}, {}>", shape.payload))
    {
        return Err("Product propagation lacks complete residual/continuation shapes".into());
    }

    Ok(shape.input.clone())
}

pub(super) fn nominal(shape: &Shape, ok: &TypeToken, error: &TypeToken) -> Result<(), String> {
    let TypeToken::Struct {
        nominal: product, ..
    } = ok
    else {
        return Err("Product propagation success is not a checked named product".into());
    };
    let TypeToken::FieldlessEnum {
        nominal: failure, ..
    } = error
    else {
        return Err("Product propagation error is not the actual checked Copy enum".into());
    };
    if nominal_path(&shape.payload, true)?.0 != product.definition_path
        || nominal_path(&shape.error, true)? != (failure.definition_path.clone(), 0)
    {
        return Err(
            "Product propagation observed nominal identities disagree with captured tokens".into(),
        );
    }
    Ok(())
}

fn nominal_path(text: &str, anonymous: bool) -> Result<(String, usize), String> {
    let syn::Type::Path(path) = syn::parse_str::<syn::Type>(text).map_err(|e| e.to_string())?
    else {
        return Err("Product propagation requires a direct nominal type path".into());
    };

    if path.qself.is_some() || path.path.leading_colon.is_some() || path.path.segments.is_empty() {
        return Err("Product propagation nominal path is not canonical".into());
    }

    let last = path.path.segments.len() - 1;
    let mut lifetimes = 0;

    for (position, segment) in path.path.segments.iter().enumerate() {
        match &segment.arguments {
            syn::PathArguments::None => {}
            syn::PathArguments::AngleBracketed(arguments) if position == last => {
                for argument in &arguments.args {
                    let syn::GenericArgument::Lifetime(lifetime) = argument else {
                        return Err("Product propagation permits only lifetime arguments".into());
                    };
                    if anonymous && lifetime.ident != "_" {
                        return Err(
                            "Generated product regions must be anonymous observations".into()
                        );
                    }
                    lifetimes += 1;
                }
            }
            _ => return Err("Product propagation nominal arguments are not canonical".into()),
        }
    }

    Ok((
        path.path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        lifetimes,
    ))
}
