//! Narrow declaration contract, independently parsed before and after capture.
use syn::visit::{self, Visit};

pub fn array_declaration(source: &str) -> Result<syn::ItemConst, String> {
    let item: syn::ItemConst = syn::parse_str(source).map_err(|e| e.to_string())?;
    if !item.attrs.is_empty()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || !matches!(item.vis, syn::Visibility::Inherited)
    {
        return Err("Table requires an unadorned nongeneric local const".into());
    }

    array_length(&item.ty)?;
    initializer(&item)?;
    Ok(item)
}

pub fn function_declaration(source: &str) -> Result<syn::ItemFn, String> {
    let item: syn::ItemFn = syn::parse_str(source).map_err(|e| e.to_string())?;
    let sig = &item.sig;
    if !item.attrs.is_empty()
        || !matches!(item.vis, syn::Visibility::Inherited)
        || sig.constness.is_none()
        || sig.asyncness.is_some()
        || sig.unsafety.is_some()
        || sig.abi.is_some()
        || sig.variadic.is_some()
        || !sig.inputs.is_empty()
        || !sig.generics.params.is_empty()
        || sig.generics.where_clause.is_some()
    {
        return Err(
            "Table initializer requires a safe unadorned zero-argument local const fn".into(),
        );
    }

    return_token(sig)?;
    let mut restrictions = Restrictions { valid: true };
    restrictions.visit_block(&item.block);
    if !restrictions.valid {
        return Err(
            "Compile-time table body contains attributes, macros, unsafe or nested declarations"
                .into(),
        );
    }
    Ok(item)
}

pub(super) fn return_token(sig: &syn::Signature) -> Result<String, String> {
    let syn::ReturnType::Type(_, ty) = &sig.output else {
        return Err("Missing bool table result".into());
    };
    array_token(ty)
}

pub(super) fn array_token(ty: &syn::Type) -> Result<String, String> {
    let length = array_length(ty)?;
    crate::data_types::TypeToken::Array {
        element: Box::new(crate::data_types::TypeToken::Scalar("bool".into())),
        length,
    }
    .text()
}

pub(super) fn array_length(ty: &syn::Type) -> Result<u64, String> {
    let syn::Type::Array(array) = ty else {
        return Err("Expected concrete bool array".into());
    };
    let syn::Expr::Lit(length) = &array.len else {
        return Err("Table length must be a literal".into());
    };
    let syn::Lit::Int(length) = &length.lit else {
        return Err("Table length must be an integer".into());
    };

    let length = length.base10_parse::<u64>().map_err(|e| e.to_string())?;
    if !matches!(&*array.elem, syn::Type::Path(ty) if ty.qself.is_none() && ty.path.is_ident("bool"))
        || !(1..=super::MAX_LENGTH).contains(&length)
    {
        return Err("Only bounded nonempty bool tables are supported".into());
    }
    Ok(length)
}

pub(super) fn initializer(item: &syn::ItemConst) -> Result<String, String> {
    let syn::Expr::Call(call) = &*item.expr else {
        return Err("Table requires a local const function call".into());
    };
    let syn::Expr::Path(path) = &*call.func else {
        return Err("Table initializer must resolve a function item".into());
    };
    if !call.attrs.is_empty()
        || !call.args.is_empty()
        || !path.attrs.is_empty()
        || path.qself.is_some()
        || path.path.leading_colon.is_some()
        || path.path.segments.len() != 1
        || !path.path.segments[0].arguments.is_empty()
    {
        return Err("Table requires an exact zero-argument lexical initializer".into());
    }
    Ok(path.path.segments[0].ident.to_string())
}

struct Restrictions {
    valid: bool,
}
impl<'ast> Visit<'ast> for Restrictions {
    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {
        self.valid = false;
    }
    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        self.valid = false;
    }
    fn visit_item(&mut self, _: &'ast syn::Item) {
        self.valid = false;
    }
    fn visit_expr_unsafe(&mut self, _: &'ast syn::ExprUnsafe) {
        self.valid = false;
    }
    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        if matches!(
            expression,
            syn::Expr::Const(_) | syn::Expr::Closure(_) | syn::Expr::Async(_)
        ) {
            self.valid = false;
        }
        visit::visit_expr(self, expression);
    }
}
