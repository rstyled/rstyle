//! Exhaustive source proof for call-free primitive float leaves.
//!
//! This proves the dependency boundary only. Callers must still verify the exact
//! owner's items/imports/attributes and run every ordinary HIR/float verifier.
use super::*;
use crate::float_scalars::FloatValue;

pub(super) fn verify(
    facts: &HirFnFacts,
    environment: &Environment,
    namespace: &[String],
) -> Result<bool, String> {
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing scalar leaf source file")?;
    let mut owners = Vec::new();
    for module in environment
        .modules
        .iter()
        .filter(|module| module.namespace == namespace && module.path.to_string_lossy() == file)
    {
        let source = environment.source(&module.path)?;
        for item in &module.items {
            if let syn::Item::Fn(item) = item
                && (FunctionSource {
                    vis: &item.vis,
                    sig: &item.sig,
                    body: &item.block,
                })
                .exact(source.bytes(), facts)
            {
                owners.push((module, item));
            }
        }
    }
    if owners.is_empty() {
        return Ok(false);
    }
    let [(module, function)] = owners.as_slice() else {
        return Err("Missing or ambiguous exact original scalar leaf ownership".into());
    };

    Ok(source_leaf(
        function,
        &module.items,
        supports(facts, crate::float_scalars::SUBTRACTION)?,
    )
    .is_ok())
}

fn source_leaf(
    function: &syn::ItemFn,
    items: &[syn::Item],
    subtraction: bool,
) -> Result<(), String> {
    let signature = &function.sig;
    if signature.constness.is_some()
        || signature.asyncness.is_some()
        || signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
    {
        return Err("Scalar leaf requires an ordinary nongeneric safe function".into());
    }
    let syn::ReturnType::Type(_, result) = &signature.output else {
        return Err("Scalar leaf requires an explicit primitive float result".into());
    };
    let result = primitive(result)?;
    let mut scope = Scope {
        values: BTreeMap::new(),
        items,
        subtraction,
    };
    for input in &signature.inputs {
        let syn::FnArg::Typed(input) = input else {
            return Err("Scalar leaf cannot have a receiver".into());
        };
        let name = binding(&input.pat)?;
        if !input.attrs.is_empty() || scope.values.insert(name, primitive(&input.ty)?).is_some() {
            return Err("Scalar leaf parameter attributes or duplicate binding".into());
        }
    }

    if scope.block(&function.block)? != result {
        return Err("Scalar leaf result disagrees with its primitive source type".into());
    }
    Ok(())
}

fn primitive(ty: &syn::Type) -> Result<&'static str, String> {
    match ty {
        syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("f32") => Ok("f32"),
        syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("f64") => Ok("f64"),
        _ => Err("Scalar leaf dependency is not a bare primitive float".into()),
    }
}

fn binding(pattern: &syn::Pat) -> Result<String, String> {
    let syn::Pat::Ident(pattern) = pattern else {
        return Err("Scalar leaf requires a plain initialized value binding".into());
    };
    if !pattern.attrs.is_empty()
        || pattern.by_ref.is_some()
        || pattern.mutability.is_some()
        || pattern.subpat.is_some()
    {
        return Err("Scalar leaf binding has unsupported modifiers".into());
    }
    Ok(pattern.ident.to_string())
}

#[derive(Clone)]
struct Scope<'a> {
    values: BTreeMap<String, &'static str>,
    items: &'a [syn::Item],
    subtraction: bool,
}

impl Scope<'_> {
    fn block(&self, block: &syn::Block) -> Result<&'static str, String> {
        let mut scope = self.clone();
        let Some((tail, statements)) = block.stmts.split_last() else {
            return Err("Scalar leaf requires a float block result".into());
        };
        for statement in statements {
            let syn::Stmt::Local(local) = statement else {
                return Err("Scalar leaf permits only initialized lets before its result".into());
            };
            scope.local(local)?;
        }

        let syn::Stmt::Expr(expression, None) = tail else {
            return Err("Scalar leaf requires a complete tail expression".into());
        };
        scope.expression(expression)
    }

    fn local(&mut self, local: &syn::Local) -> Result<(), String> {
        let name = binding(&local.pat)?;
        let initial = local
            .init
            .as_ref()
            .ok_or("Scalar leaf local is uninitialized")?;
        if !local.attrs.is_empty() || initial.diverge.is_some() {
            return Err("Scalar leaf let attributes or diverging initializer".into());
        }

        let ty = self.expression(&initial.expr)?;
        self.values.insert(name, ty);
        Ok(())
    }

    fn expression(&self, expression: &syn::Expr) -> Result<&'static str, String> {
        match expression {
            syn::Expr::Lit(value) if value.attrs.is_empty() => literal(&value.lit),
            syn::Expr::Path(value) if value.attrs.is_empty() && value.qself.is_none() => {
                let name = value
                    .path
                    .get_ident()
                    .ok_or("Scalar leaf value path is not bare")?;
                self.value(&name.to_string())
            }
            syn::Expr::Paren(value) if value.attrs.is_empty() => self.expression(&value.expr),
            syn::Expr::Group(value) if value.attrs.is_empty() => self.expression(&value.expr),
            syn::Expr::Block(value) if value.attrs.is_empty() && value.label.is_none() => {
                self.block(&value.block)
            }
            syn::Expr::Unary(value)
                if value.attrs.is_empty() && matches!(value.op, syn::UnOp::Neg(_)) =>
            {
                self.expression(&value.expr)
            }
            syn::Expr::Binary(value) if value.attrs.is_empty() => self.binary(value),
            syn::Expr::Cast(value) if value.attrs.is_empty() => {
                if self.expression(&value.expr)? != "f64" || primitive(&value.ty)? != "f32" {
                    return Err("Scalar leaf supports only binary64 to binary32 casts".into());
                }
                Ok("f32")
            }
            _ => Err("Scalar leaf has an uncovered source expression or effect".into()),
        }
    }

    fn binary(&self, value: &syn::ExprBinary) -> Result<&'static str, String> {
        if !(matches!(value.op, syn::BinOp::Add(_) | syn::BinOp::Mul(_))
            || self.subtraction && matches!(value.op, syn::BinOp::Sub(_)))
            || self.expression(&value.left)? != "f64"
            || self.expression(&value.right)? != "f64"
        {
            return Err("Scalar leaf requires registered binary64 arithmetic".into());
        }
        Ok("f64")
    }

    fn value(&self, name: &str) -> Result<&'static str, String> {
        if let Some(ty) = self.values.get(name) {
            return Ok(ty);
        }
        let mut candidates = self.items.iter().filter_map(|item| match item {
            syn::Item::Const(item) if item.ident == name => Some(item),
            _ => None,
        });
        let constant = candidates
            .next()
            .ok_or("Scalar leaf value lacks a local declaration")?;
        if candidates.next().is_some()
            || !constant.attrs.is_empty()
            || !constant.generics.params.is_empty()
            || constant.generics.where_clause.is_some()
        {
            return Err("Scalar leaf constant is ambiguous, attributed or generic".into());
        }

        let ty = primitive(&constant.ty)?;
        FloatValue::parse(&constant_spelling(&constant.expr)?, ty)?;
        Ok(ty)
    }
}

fn literal(value: &syn::Lit) -> Result<&'static str, String> {
    let syn::Lit::Float(value) = value else {
        return Err("Scalar leaf requires original decimal float literals".into());
    };
    let ty = match value.suffix() {
        "" | "f64" => "f64",
        "f32" => "f32",
        _ => return Err("Scalar leaf float literal has an unknown width".into()),
    };
    FloatValue::parse(&value.to_string(), ty)?;
    Ok(ty)
}

fn constant_spelling(expression: &syn::Expr) -> Result<String, String> {
    match expression {
        syn::Expr::Lit(value) if value.attrs.is_empty() => match &value.lit {
            syn::Lit::Float(value) => Ok(value.to_string()),
            _ => Err("Scalar leaf constant requires a decimal float literal".into()),
        },
        syn::Expr::Unary(value)
            if value.attrs.is_empty() && matches!(value.op, syn::UnOp::Neg(_)) =>
        {
            Ok(format!("-{}", constant_spelling(&value.expr)?))
        }
        _ => Err("Scalar leaf constant has an uncovered source dependency".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::source_leaf;

    fn checked(source: &str) -> bool {
        checked_with_subtraction(source, false)
    }

    fn checked_with_subtraction(source: &str, subtraction: bool) -> bool {
        let file = syn::parse_file(source).unwrap();
        let function = file
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Fn(item) if item.sig.ident == "kernel" => Some(item),
                _ => None,
            })
            .unwrap();
        source_leaf(function, &file.items, subtraction).is_ok()
    }

    #[test]
    fn subtraction_needs_its_own_capability_and_complete_primitive_dependencies() {
        let source = "fn kernel(x:f64,y:f64)->f64 {let w=x*y; let w=1.0-w; w+((1.0-w)-x*y)}";
        assert!(!checked(source));
        assert!(checked_with_subtraction(source, true));
        for body in ["x / y", "helper(x)-y", "{let _drop=Noise; x-y}"] {
            assert!(!checked_with_subtraction(
                &format!("fn kernel(x:f64,y:f64)->f64 {{{body}}}"),
                true
            ));
        }
        assert!(!checked_with_subtraction(
            "fn kernel(x:f32,y:f32)->f32 {x-y}",
            true
        ));
    }

    #[test]
    fn source_family_covers_complete_float_dependency_composition() {
        for source in [
            "const C:f64=-0.5; fn kernel(x:f64)->f32 { let z=x*x; let w={let r=z*C; r+1.0}; (x+w) as f32 }",
            "fn kernel(x:f32)->f32 {-x}",
            "fn kernel(x:f64)->f64 {let x=x*x; let y=x+1.0; x*y}",
        ] {
            assert!(checked(source), "{source}");
        }
    }

    #[test]
    fn source_proof_rejects_uncovered_dependencies_and_effectful_suffixes() {
        for body in [
            "let x:f64; x",
            "let x; 1.0",
            "struct Hidden; 1.0",
            "fn nested()->f64{1.0} 1.0",
            "let c=||1.0; 1.0",
            "helper(x)",
            "{helper(x); x}",
            "let r=&x; x",
            "unknown!()",
            "x+unknown!()",
            "let y=1.0; helper(y); x",
            "let y=1.0; unsafe {x}",
            "let y=1.0; return y",
            "if true{x}else{x}",
            "x/1.0",
            "x-1.0",
            "let mut y=x; y",
            "let (a,b)=(x,x); a",
            "x as u64",
            "x.frob()",
        ] {
            let source = format!("fn kernel(x:f64)->f64 {{{body}}}");
            assert!(!checked(&source), "{source}");
        }
    }

    #[test]
    fn constants_cannot_hide_calls_aliases_or_imported_payloads() {
        for declaration in [
            "const C:f64=helper();",
            "const C:f64=1.0+2.0;",
            "const C:Alias=1.0;",
            "const C:f64=OTHER;",
            "#[allow(dead_code)] const C:f64=1.0;",
            "const C:f64=1e999;",
            "use other::C;",
        ] {
            let source = format!("{declaration} fn kernel(x:f64)->f64 {{x*C}}");
            assert!(!checked(&source), "{source}");
        }
    }
}
