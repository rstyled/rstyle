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

/// Wraps a function's body under a root of its own, keeping its signature and its body's
/// tokens as written, as `#[tracing::instrument]` does for an async method.
#[proc_macro_attribute]
pub fn wrapping(_arguments: TokenStream, item: TokenStream) -> TokenStream {
    let mut tokens: Vec<TokenTree> = item.into_iter().collect();
    let body = tokens.pop().expect("the wrapped function's body");
    let result = tokens.last().expect("the wrapped function's result").span();
    // A branch of the macro's own, written at the result type's span.
    let own: TokenStream = "if false { return __wrapped; }".parse().expect("the branch");
    let wrapper: TokenStream = "let __wrapped = __BODY__; __OWN__ __wrapped"
        .parse()
        .expect("the wrapper");
    let wrapper = wrapper
        .into_iter()
        .flat_map(|token| match &token {
            TokenTree::Ident(ident) if ident.to_string() == "__BODY__" => vec![body.clone()],
            TokenTree::Ident(ident) if ident.to_string() == "__OWN__" => {
                respanned(own.clone(), result).into_iter().collect()
            }
            _ => vec![token],
        })
        .collect();
    let mut root = proc_macro::Group::new(proc_macro::Delimiter::Brace, wrapper);
    root.set_span(Span::call_site());
    tokens.push(TokenTree::from(root));

    tokens.into_iter().collect()
}

/// Every token of `stream` at `span`, as `quote_spanned!` writes them.
fn respanned(stream: TokenStream, span: Span) -> TokenStream {
    stream
        .into_iter()
        .map(|mut token| {
            if let TokenTree::Group(group) = &token {
                let inner = respanned(group.stream(), span);
                token = TokenTree::from(proc_macro::Group::new(group.delimiter(), inner));
            }
            token.set_span(span);
            token
        })
        .collect()
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
