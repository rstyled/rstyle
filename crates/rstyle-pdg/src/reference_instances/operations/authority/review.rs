//! Source reviewed with rustc 5c543b0b8c73c7b72bc8284ced4fb22ead15734d.
//! These exact method excerpts define this bounded contract, not arbitrary AsRef purity.
const SLICE: &str = include_str!("source/slice-as-ref.rs");
const STR: &str = include_str!("source/str-as-ref.rs");
const BYTES: &str = include_str!("source/str-as-bytes.rs");

pub(super) fn verify() -> Result<(), String> {
    let slice: syn::ImplItemFn = syn::parse_str(SLICE).map_err(|e| e.to_string())?;
    let text: syn::ImplItemFn = syn::parse_str(STR).map_err(|e| e.to_string())?;
    let bytes: syn::ImplItemFn = syn::parse_str(BYTES).map_err(|e| e.to_string())?;

    signature(&slice, "as_ref", "T")?;
    signature(&text, "as_ref", "u8")?;
    signature(&bytes, "as_bytes", "u8")?;

    let [syn::Stmt::Expr(slice_value, None)] = slice.block.stmts.as_slice() else {
        return Err("Reviewed slice AsRef body changed".into());
    };
    let [syn::Stmt::Expr(syn::Expr::MethodCall(call), None)] = text.block.stmts.as_slice() else {
        return Err("Reviewed str AsRef body changed".into());
    };
    if !is_self(slice_value)
        || !is_self(&call.receiver)
        || call.method != "as_bytes"
        || !call.args.is_empty()
        || call.turbofish.is_some()
    {
        return Err("Reviewed AsRef source no longer has its exact identity/byte-view body".into());
    }
    byte_intrinsic(&bytes)
}

fn byte_intrinsic(bytes: &syn::ImplItemFn) -> Result<(), String> {
    let [syn::Stmt::Expr(syn::Expr::Unsafe(block), None)] = bytes.block.stmts.as_slice() else {
        return Err("Reviewed str as_bytes body changed".into());
    };
    let [syn::Stmt::Expr(syn::Expr::Call(call), None)] = block.block.stmts.as_slice() else {
        return Err("Reviewed str byte-view intrinsic changed".into());
    };
    let syn::Expr::Path(path) = &*call.func else {
        return Err("Reviewed byte-view intrinsic is not a fixed path".into());
    };

    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || path.path.segments.len() != 2
        || path.path.segments[0].ident != "mem"
        || path.path.segments[1].ident != "transmute"
        || path
            .path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
        || call.args.len() != 1
        || !is_self(&call.args[0])
    {
        return Err("Reviewed str byte-view representation contract changed".into());
    }
    Ok(())
}

fn is_self(value: &syn::Expr) -> bool {
    let syn::Expr::Path(path) = value else {
        return false;
    };
    path.qself.is_none()
        && crate::source_evidence::plain_relative_path(&path.path).as_deref()
            == Some(&["self".into()])
}

fn signature(method: &syn::ImplItemFn, name: &str, element: &str) -> Result<(), String> {
    let signature = &method.sig;
    let mut inputs = signature.inputs.iter();
    let Some(syn::FnArg::Receiver(receiver)) = inputs.next() else {
        return Err("Reviewed byte-view method lost its sole receiver".into());
    };
    if inputs.next().is_some() {
        return Err("Reviewed byte-view method has extra inputs".into());
    }

    let (result, path) = returned_slice(signature)?;

    if signature.ident != name
        || !method.attrs.is_empty()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || signature.constness.is_some() != (name == "as_bytes")
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
        || receiver
            .reference
            .as_ref()
            .is_none_or(|(_, lifetime)| lifetime.is_some())
        || receiver.mutability.is_some()
        || receiver.colon_token.is_some()
        || result.lifetime.is_some()
        || result.mutability.is_some()
        || path.qself.is_some()
        || !path.path.is_ident(element)
    {
        return Err("Reviewed byte-view method signature changed".into());
    }
    Ok(())
}

fn returned_slice(
    signature: &syn::Signature,
) -> Result<(&syn::TypeReference, &syn::TypePath), String> {
    let syn::ReturnType::Type(_, result) = &signature.output else {
        return Err("Reviewed byte-view method lost its result".into());
    };
    let syn::Type::Reference(result) = &**result else {
        return Err("Reviewed byte-view result is not borrowed".into());
    };

    let syn::Type::Slice(slice) = &*result.elem else {
        return Err("Reviewed byte-view result is not a slice".into());
    };
    let syn::Type::Path(path) = &*slice.elem else {
        return Err("Reviewed byte-view element is not fixed".into());
    };
    Ok((result, path))
}
