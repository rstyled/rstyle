//! Complete bounded source grammar; no supported-prefix or name-only acceptance.
use syn::spanned::Spanned;

const HEADER: &str = "impl<'a, T> ParsedItem<'a, T>";
const METHOD: &str = "pub(crate) fn flat_map<U>(self, f: impl FnOnce(T) -> Option<U>) -> Option<ParsedItem<'a, U>> { Some(ParsedItem(self.0, f(self.1)?)) }";

pub(super) fn verify(source: &str, facts: &crate::HirFnFacts) -> Result<(), String> {
    if source.len() > 64 * 1024 {
        return Err("Callback owner exceeds bounded source budget".into());
    }
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    let mut candidates = Vec::new();
    for item in &file.items {
        if let syn::Item::Impl(owner) = item {
            for member in &owner.items {
                if let syn::ImplItem::Fn(method) = member {
                    let range = method_range(source, method);
                    if range == facts.fn_span {
                        candidates.push((owner, method));
                    }
                }
            }
        }
    }

    let [(owner, method)] = candidates.as_slice() else {
        return Err("Callback requires one original generic member at its captured span".into());
    };

    owner_header(source, owner)?;
    attributes(&method.attrs)?;
    let range = method_range(source, method);
    let original = source
        .get(range.start as usize..range.end as usize)
        .ok_or("Missing callback source range")?;
    if tokens(original)? != tokens(METHOD)? {
        return Err("Callback requires the complete ordered saved-suffix, once-only Option propagation body".into());
    }

    if original != facts.source.bytes.as_str() {
        return Err("Callback original member does not match captured source facts".into());
    }
    Ok(())
}

fn owner_header(source: &str, owner: &syn::ItemImpl) -> Result<(), String> {
    let range = crate::source_evidence::original_span(source, owner.span());
    let original = source
        .get(range.start as usize..range.end as usize)
        .ok_or("Missing callback owner source")?;
    let stream: proc_macro2::TokenStream = original
        .parse()
        .map_err(|error: proc_macro2::LexError| error.to_string())?;
    let mut header = stream.into_iter().collect::<Vec<_>>();
    header.pop().ok_or("Missing callback owner body")?;

    if header
        .into_iter()
        .collect::<proc_macro2::TokenStream>()
        .to_string()
        != tokens(HEADER)?
    {
        return Err("Callback requires the complete original generic inherent owner".into());
    }
    Ok(())
}

fn tokens(source: &str) -> Result<String, String> {
    source
        .parse::<proc_macro2::TokenStream>()
        .map(|stream| stream.to_string())
        .map_err(|error| error.to_string())
}

fn attributes(attributes: &[syn::Attribute]) -> Result<(), String> {
    if !crate::source_evidence::literal_metadata(attributes, "inline") {
        return Err("Callback member has an unproved conditional or generated attribute".into());
    }

    Ok(())
}

fn method_range(source: &str, method: &syn::ImplItemFn) -> crate::ByteSpan {
    let start = if matches!(method.vis, syn::Visibility::Inherited) {
        method.sig.span()
    } else {
        method.vis.span()
    };
    crate::ByteSpan {
        start: crate::source_evidence::original_span(source, start).start,
        end: crate::source_evidence::original_span(source, method.block.span()).end,
    }
}
