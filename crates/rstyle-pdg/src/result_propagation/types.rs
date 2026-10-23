//! Structural type operations shared after each propagation proves its payload.
pub(crate) fn scalar_path(ty: &syn::Type, name: &str) -> bool {
    matches!(ty, syn::Type::Path(path) if path.qself.is_none()
        && path.path.leading_colon.is_none() && path.path.segments.len() == 1
        && path.path.segments[0].ident == name
        && path.path.segments[0].arguments.is_none())
}

/// Each implementation proves its dedicated payload before exposing output parts.
pub(crate) trait CheckedOutput {
    const GENERATED_PAYLOAD: &'static str;

    fn checked_output(&self) -> Result<DeclaredOutput<'_>, String>;
}

pub(crate) struct DeclaredOutput<'a> {
    pub declared: &'a str,
    pub payload: &'a str,
    pub error: &'a str,
}

/// Preserve the exact declared Result path while replacing a proved payload.
pub(crate) fn generated_output<T: CheckedOutput>(shape: &T) -> Result<String, String> {
    let DeclaredOutput {
        declared,
        payload,
        error,
    } = shape.checked_output()?;
    let path = ["core::result::Result", "std::result::Result"]
        .into_iter()
        .find(|path| declared == format!("{path}<{payload}, {error}>"))
        .ok_or("Result output lacks its exact declared core type")?;

    Ok(format!("{path}<{}, {error}>", T::GENERATED_PAYLOAD))
}
