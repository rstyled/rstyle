//! A derive that writes its own tokens at one of its input's, as `quote_spanned!` does.
use proc_macro::{Ident, Punct, Spacing, Span, TokenStream, TokenTree};

#[proc_macro_derive(Spanning)]
pub fn spanning(input: TokenStream) -> TokenStream {
    let name = input
        .into_iter()
        .skip_while(|token| token.to_string() != "struct")
        .nth(1)
        .expect("the derived type's name");
    let written = name.span();
    let local = TokenTree::from(Ident::new("spanned", written));
    let mut operator = Punct::new('+', Spacing::Alone);
    operator.set_span(written);

    let generated: TokenStream = format!(
        "impl {name} {{ pub fn generated(left: u32, right: u32) -> u32 {{ \
         let __LOCAL__ = left __OPERATOR__ right; __LOCAL__ }} }}"
    )
    .parse()
    .expect("the generated implementation");

    substitute(generated, &local, &TokenTree::from(operator))
}

fn substitute(stream: TokenStream, local: &TokenTree, operator: &TokenTree) -> TokenStream {
    stream
        .into_iter()
        .map(|token| match &token {
            TokenTree::Group(group) => {
                let inner = substitute(group.stream(), local, operator);
                let mut rebuilt = proc_macro::Group::new(group.delimiter(), inner);
                rebuilt.set_span(Span::call_site());
                TokenTree::from(rebuilt)
            }
            TokenTree::Ident(ident) if ident.to_string() == "__LOCAL__" => local.clone(),
            TokenTree::Ident(ident) if ident.to_string() == "__OPERATOR__" => operator.clone(),
            _ => token,
        })
        .collect()
}
