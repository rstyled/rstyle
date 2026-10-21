//! Literal messages with implicit, distinct builtin scalar captures.

pub struct Capture {
    pub name: String,
    pub span: proc_macro2::Span,
}

pub struct Diagnostic {
    pub message: String,
    pub captures: Vec<Capture>,
}

pub fn parse(expression: &syn::Expr) -> Result<Diagnostic, String> {
    let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(literal),
        ..
    }) = expression
    else {
        return Err("Assertion diagnostic requires a literal format string".into());
    };

    let decoded = fields(&literal.value())?;
    let token = literal.token();
    let raw = token.to_string();
    let original = fields(&raw)?;
    if decoded.iter().map(|(name, _)| name).collect::<Vec<_>>()
        != original.iter().map(|(name, _)| name).collect::<Vec<_>>()
    {
        return Err("Escaped format captures lack direct source correspondence".into());
    }
    let mut captures = Vec::new();
    for (name, range) in original {
        let span = token
            .subspan(range)
            .ok_or("Missing literal capture subspan")?;
        captures.push(Capture { name, span });
    }
    Ok(Diagnostic {
        message: literal.value(),
        captures,
    })
}

fn fields(source: &str) -> Result<Vec<(String, std::ops::Range<usize>)>, String> {
    let mut fields = Vec::new();
    let mut index = 0;
    let bytes = source.as_bytes();

    while index < bytes.len() {
        match bytes[index] {
            b'{' | b'}' if bytes.get(index + 1) == bytes.get(index) => index += 2,
            b'{' => {
                let start = index + 1;
                let end = source[start..]
                    .find('}')
                    .map(|offset| start + offset)
                    .ok_or("Unclosed assertion format capture")?;
                let name = &source[start..end];
                if syn::parse_str::<syn::Ident>(name).is_err()
                    || !name.is_ascii()
                    || fields.iter().any(|(previous, _)| previous == name)
                {
                    return Err(
                        "Only distinct implicit scalar format captures are supported".into(),
                    );
                }
                fields.push((name.into(), start..end));
                index = end + 1;
            }
            b'}' => return Err("Unmatched assertion format brace".into()),
            _ => index += 1,
        }
    }
    Ok(fields)
}

pub fn span(source: &str, offset: u32, capture: &Capture) -> crate::ByteSpan {
    crate::ByteSpan {
        start: offset + crate::source_evidence::byte_offset(source, capture.span.start()) as u32,
        end: offset + crate::source_evidence::byte_offset(source, capture.span.end()) as u32,
    }
}

pub fn verify(node: &crate::HirNode, source: &str, diagnostic: &Diagnostic) -> Result<(), String> {
    if node.children.len() != diagnostic.captures.len() + 1 {
        return Err("Assertion diagnostic loses captured operands".into());
    }
    for (capture, operand) in diagnostic.captures.iter().zip(&node.children[1..]) {
        let scalar = operand.ty.as_ref().and_then(|ty| ty.normalized.as_deref());
        if operand.kind != "expr_path"
            || operand.span != span(source, node.span.start, capture)
            || operand.snippet != capture.name
            || !operand.children.is_empty()
            || operand.expansion.is_some()
            || !operand
                .res
                .as_ref()
                .is_some_and(|res| res.kind == "local" && res.target.is_some())
            || !scalar.is_some_and(|ty| {
                matches!(
                    ty,
                    "bool"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "usize"
                        | "i8"
                        | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "isize"
                )
            })
        {
            return Err("Assertion diagnostic lacks a direct builtin scalar capture".into());
        }
    }
    Ok(())
}
