use syn::{
    Expr, Ident, Token,
    parse::{Parse, ParseStream},
};

pub(super) struct Arguments(pub Vec<Expr>);

impl Parse for Arguments {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut expressions = Vec::new();
        while !input.is_empty() {
            expressions.push(input.parse()?);
            if input.peek(Token![=>]) {
                input.parse::<Token![=>]>()?;
                options(input, &mut expressions)?;
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(Self(expressions))
    }
}

fn options(input: ParseStream<'_>, expressions: &mut Vec<Expr>) -> syn::Result<()> {
    let first = expressions.len();
    while input.peek(Ident) {
        let name: Ident = input.parse()?;
        if ![
            "fill",
            "sign_plus",
            "sign_minus",
            "align",
            "width",
            "precision",
            "alternate",
            "sign_aware_zero_pad",
        ]
        .contains(&name.to_string().as_str())
        {
            return Err(syn::Error::new(
                name.span(),
                "Unknown source-proved formatter option",
            ));
        }
        let content;
        syn::parenthesized!(content in input);
        expressions.push(content.parse()?);
        if !content.is_empty() {
            return Err(content.error("Option requires exactly one complete expression"));
        }
    }
    if expressions.len() == first {
        return Err(input.error("Formatter options cannot be empty"));
    }
    Ok(())
}
