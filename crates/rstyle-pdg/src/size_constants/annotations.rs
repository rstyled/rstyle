//! Bind named local array lengths to the independently verified declaration.
use super::*;
use crate::data_types::TypeToken;

pub(crate) fn verify_annotation(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if node.kind != "ty_annotation" {
        return Ok(());
    }
    let constants: Vec<_> = facts
        .body
        .children
        .iter()
        .filter(|child| child.kind == "stmt_size_const")
        .collect();
    if constants.is_empty() {
        return Ok(());
    }
    let source = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing named-length source")?;
    let syntax: syn::Type = syn::parse_str(source).map_err(|error| error.to_string())?;

    let mut names = Vec::new();
    for constant in constants {
        verify(constant, facts)?;
        let record = crate::scalar_constants::decode_record::<NativeSize>(
            constant.res.as_ref().ok_or("Missing size declaration")?,
            RESOLUTION,
        )?;
        let name = declaration(&record.constant.declaration)?.ident.to_string();
        if names.contains(&name) {
            return Err("Ambiguous local size declaration".into());
        }
        names.push(name);
    }
    // Only this bounded source shape gains named-length authority. Other local
    // annotations keep their existing independently checked contracts.
    if !referenced(&syntax, &names) {
        return Ok(());
    }

    if token(&syntax, &names)? != crate::data_types::parse(crate::data_aggregates::token(node)?)? {
        return Err("Named byte-array annotation contradicts the resolved type".into());
    }
    Ok(())
}

fn token(ty: &syn::Type, names: &[String]) -> Result<TypeToken, String> {
    match ty {
        syn::Type::Reference(ty) if ty.mutability.is_none() => {
            Ok(TypeToken::Shared(Box::new(token(&ty.elem, names)?)))
        }
        syn::Type::Array(ty) => {
            let length = match &ty.len {
                syn::Expr::Path(path)
                    if path.qself.is_none()
                        && names.iter().any(|name| path.path.is_ident(name)) =>
                {
                    8
                }
                syn::Expr::Lit(value) => match &value.lit {
                    syn::Lit::Int(value) => {
                        value.base10_parse().map_err(|error| error.to_string())?
                    }
                    _ => return Err("Unsupported array length".into()),
                },
                _ => return Err("Unresolved local array length".into()),
            };
            Ok(TypeToken::Array {
                element: Box::new(token(&ty.elem, names)?),
                length,
            })
        }
        syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("u8") => {
            Ok(TypeToken::Scalar("u8".into()))
        }
        syn::Type::Path(path) => result(path, names),
        _ => Err("Unsupported named-length annotation shape".into()),
    }
}

fn result(path: &syn::TypePath, names: &[String]) -> Result<TypeToken, String> {
    let segment = path.path.segments.first().ok_or("Missing Result name")?;
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err("Missing Result arguments".into());
    };
    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || path.path.segments.len() != 1
        || segment.ident != "Result"
        || arguments.args.len() != 2
        || !matches!(
            &arguments.args[1],
            syn::GenericArgument::Type(syn::Type::Infer(_))
        )
    {
        return Err("Named length requires the exact concrete Result annotation".into());
    }
    let syn::GenericArgument::Type(payload) = &arguments.args[0] else {
        return Err("Missing Result payload type".into());
    };

    let payload = token(payload, names)?;
    let expected = TypeToken::Shared(Box::new(TypeToken::Array {
        element: Box::new(TypeToken::Scalar("u8".into())),
        length: 8,
    }));
    if payload != expected {
        return Err("Named Result payload requires &[u8;8]".into());
    }
    Ok(TypeToken::SliceArrayResult {
        length: 8,
        borrowed: true,
    })
}

fn referenced(syntax: &syn::Type, names: &[String]) -> bool {
    use syn::visit::Visit;
    struct Names<'a> {
        names: &'a [String],
        found: bool,
    }
    impl<'ast> Visit<'ast> for Names<'_> {
        fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
            self.found |= self.names.iter().any(|name| path.path.is_ident(name));
            syn::visit::visit_expr_path(self, path);
        }
    }

    let mut visitor = Names {
        names,
        found: false,
    };
    visitor.visit_type(syntax);
    visitor.found
}
