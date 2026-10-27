//! Bounded interpretation of original single-rule local declarative macros.
mod ownership;

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use std::collections::BTreeMap;

/// This expansion supplies no callable or cleanup permission by itself.
pub(super) fn expand_ownership(
    definition: &syn::ItemMacro,
    invocation: &syn::Macro,
    edition: &str,
) -> Result<TokenStream, String> {
    ownership::expand(definition, invocation, ownership::Context { edition })
}

pub(crate) fn expand(
    definition: &syn::ItemMacro,
    invocation: &syn::Macro,
) -> Result<TokenStream, String> {
    expand_mode(definition, invocation, false)
}

/// A source-view expansion: coordinates refer to template occurrences, not
/// compiler hygiene. Callers must independently check every observed mark.
pub(crate) fn expand_source(
    definition: &syn::ItemMacro,
    invocation: &syn::Macro,
) -> Result<TokenStream, String> {
    expand_mode(definition, invocation, true)
}

fn expand_mode(
    definition: &syn::ItemMacro,
    invocation: &syn::Macro,
    source_coordinates: bool,
) -> Result<TokenStream, String> {
    let rule: Vec<_> = definition.mac.tokens.clone().into_iter().collect();

    let [
        TokenTree::Group(pattern),
        equal,
        arrow,
        TokenTree::Group(body),
        rest @ ..,
    ] = rule.as_slice()
    else {
        return Err("Cleanup macro requires one complete declarative rule".into());
    };
    if equal.to_string() != "="
        || arrow.to_string() != ">"
        || rest.iter().any(|token| token.to_string() != ";")
    {
        return Err("Cleanup macro has an unsupported rule inventory".into());
    }
    let captures = capture(pattern.stream(), invocation.tokens.clone())?;
    substitute(body.stream(), &captures, source_coordinates)
}

fn capture(
    pattern: TokenStream,
    input: TokenStream,
) -> Result<BTreeMap<String, TokenStream>, String> {
    let pattern = pattern.to_string();
    let mut captures = BTreeMap::new();
    if pattern.contains("$ ($") || pattern.contains("$ ( #") || pattern.starts_with("$ (") {
        let mode = if pattern.contains(": tt") {
            "tt"
        } else if pattern.contains(": meta") {
            "meta"
        } else {
            return Err("Cleanup macro repetition requires whole tokens or attributes".into());
        };
        let expected: TokenStream = if mode == "tt" {
            "$($tt:tt)*"
        } else {
            "$(#[$doc:meta])*"
        }
        .parse()
        .unwrap();
        if pattern != expected.to_string() {
            return Err("Unsupported cleanup repetition matcher".into());
        }
        if mode == "meta" {
            let parsed: syn::ItemStruct =
                syn::parse_str(&format!("{} struct CleanupAttributes;", input))
                    .map_err(|error| error.to_string())?;
            if parsed.ident != "CleanupAttributes" {
                return Err("Cleanup macro loses attribute-only arguments".into());
            }
        }
        captures.insert(
            if mode == "tt" { "tt" } else { "doc" }.into(),
            input.clone(),
        );
    } else {
        simple(
            pattern.parse().map_err(|_| "Invalid cleanup matcher")?,
            input.clone(),
            &mut captures,
        )?;
    }
    Ok(captures)
}

fn simple(
    pattern: TokenStream,
    input: TokenStream,
    captures: &mut BTreeMap<String, TokenStream>,
) -> Result<(), String> {
    let patterns = split(pattern);
    let inputs = split(input);

    if patterns.len() != inputs.len() {
        return Err("Cleanup macro argument count disagrees".into());
    }
    for (pattern, input) in patterns.into_iter().zip(inputs) {
        let parts: Vec<_> = pattern.into_iter().collect();
        let [
            dollar,
            TokenTree::Ident(name),
            colon,
            TokenTree::Ident(fragment),
        ] = parts.as_slice()
        else {
            return Err("Cleanup macro requires simple typed captures".into());
        };
        if dollar.to_string() != "$" || colon.to_string() != ":" {
            return Err("Invalid cleanup capture".into());
        }
        let value = match fragment.to_string().as_str() {
            "ident" => {
                syn::parse2::<syn::Ident>(input.clone()).map_err(|error| error.to_string())?;
                input
            }
            "expr" => {
                syn::parse2::<syn::Expr>(input.clone()).map_err(|error| error.to_string())?;
                TokenTree::Group(Group::new(Delimiter::Parenthesis, input)).into()
            }
            _ => return Err("Unsupported cleanup macro fragment".into()),
        };
        if captures.insert(name.to_string(), value).is_some() {
            return Err("Duplicate cleanup capture".into());
        }
    }
    Ok(())
}

fn split(tokens: TokenStream) -> Vec<TokenStream> {
    let mut parts = vec![TokenStream::new()];
    for token in tokens {
        if matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',') {
            parts.push(TokenStream::new());
        } else {
            parts.last_mut().unwrap().extend([token]);
        }
    }
    if parts.last().is_some_and(TokenStream::is_empty) {
        parts.pop();
    }
    parts
}

fn substitute(
    tokens: TokenStream,
    captures: &BTreeMap<String, TokenStream>,
    source_coordinates: bool,
) -> Result<TokenStream, String> {
    let mut input = tokens.into_iter();
    let mut output = TokenStream::new();
    while let Some(token) = input.next() {
        output.extend(substitution_piece(
            token,
            &mut input,
            captures,
            source_coordinates,
        )?);
    }
    Ok(output)
}

fn substitution_piece(
    token: TokenTree,
    input: &mut impl Iterator<Item = TokenTree>,
    captures: &BTreeMap<String, TokenStream>,
    source_coordinates: bool,
) -> Result<TokenStream, String> {
    if token.to_string() == "$" {
        match input.next() {
            Some(TokenTree::Ident(name)) => {
                let value = captures
                    .get(&name.to_string())
                    .ok_or("Unbound cleanup macro capture")?
                    .clone();
                if !source_coordinates {
                    return Ok(value);
                }
                let mut value: syn::Ident = syn::parse2(value)
                    .map_err(|_| "Source coordinates require identifier captures")?;
                value.set_span(
                    token
                        .span()
                        .join(name.span())
                        .ok_or("Missing contiguous original capture occurrence")?,
                );
                Ok(TokenTree::Ident(value).into())
            }
            Some(TokenTree::Group(group)) => {
                if input.next().is_none_or(|token| token.to_string() != "*") {
                    return Err("Unsupported cleanup repetition operator".into());
                }
                repeated_capture(group, captures)
            }
            _ => Err("Unsupported cleanup macro substitution".into()),
        }
    } else if let TokenTree::Group(group) = token {
        let mut output = Group::new(
            group.delimiter(),
            substitute(group.stream(), captures, source_coordinates)?,
        );
        if source_coordinates {
            output.set_span(group.span());
        }
        Ok(TokenTree::Group(output).into())
    } else {
        Ok(token.into())
    }
}

fn repeated_capture(
    group: Group,
    captures: &BTreeMap<String, TokenStream>,
) -> Result<TokenStream, String> {
    let repeated = group.stream().to_string();
    let expected_tt: TokenStream = "$tt".parse().unwrap();
    let expected_doc: TokenStream = "#[$doc]".parse().unwrap();
    let name = if repeated == expected_tt.to_string() {
        "tt"
    } else if repeated == expected_doc.to_string() {
        "doc"
    } else {
        return Err("Unsupported cleanup repetition output".into());
    };

    captures
        .get(name)
        .cloned()
        .ok_or_else(|| "Unbound cleanup repetition".into())
}

pub(super) fn builtin(mac: &syn::Macro) -> Result<Vec<syn::Expr>, String> {
    use syn::parse::Parser;
    let name = mac
        .path
        .get_ident()
        .ok_or("Cleanup macro requires a resolved plain name")?
        .to_string();
    match name.as_str() {
        "cfg" => {
            syn::parse2::<syn::Meta>(mac.tokens.clone()).map_err(|error| error.to_string())?;
            Ok(Vec::new())
        }
        "stringify" => Ok(Vec::new()),
        "vec" => Ok(vec![
            syn::parse_str(&format!("[{}]", mac.tokens)).map_err(|error| error.to_string())?,
        ]),
        "matches" => {
            let parts = split(mac.tokens.clone());
            if parts.len() != 2 {
                return Err("Cleanup matches requires expression and pattern".into());
            }
            if pattern_tokens_have_items(parts[1].clone()) {
                return Err("Cleanup matches pattern contains opaque item-capable syntax".into());
            }
            let expression = syn::parse2(parts[0].clone()).map_err(|error| error.to_string())?;
            let arm: syn::Arm = syn::parse_str(&format!("{} => (),", parts[1]))
                .map_err(|error| error.to_string())?;
            let mut values = vec![expression];
            if let Some((_, guard)) = arm.guard {
                values.push(*guard);
            }
            Ok(values)
        }
        "assert" | "assert_eq" | "assert_ne" | "debug_assert" | "debug_assert_eq"
        | "debug_assert_ne" | "panic" | "unreachable" | "unimplemented" | "todo" | "write"
        | "writeln" | "format" | "format_args" | "concat" => {
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                .parse2(mac.tokens.clone())
                .map(|values| values.into_iter().collect())
                .map_err(|error| error.to_string())
        }
        _ => Err(format!("Cleanup closure lacks expansion proof for {name}")),
    }
}

fn pattern_tokens_have_items(tokens: TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Group(group) => {
            group.delimiter() == Delimiter::Brace || pattern_tokens_have_items(group.stream())
        }
        TokenTree::Punct(punct) => ['!', '#'].contains(&punct.as_char()),
        _ => false,
    })
}

#[cfg(test)]
mod source_coordinate_tests {
    use super::*;

    #[test]
    fn source_view_keeps_template_capture_coordinates_and_legacy_keeps_argument_span() {
        let bytes = "macro_rules! m { ($name:ident) => { pub struct $name; }; } m!(ActualName);";
        let file = syn::parse_file(bytes).unwrap();
        let syn::Item::Macro(definition) = &file.items[0] else {
            panic!("Expected definition")
        };
        let syn::Item::Macro(invocation) = &file.items[1] else {
            panic!("Expected invocation")
        };
        let ordinary = expand(definition, &invocation.mac).unwrap();
        let source = expand_source(definition, &invocation.mac).unwrap();
        assert_eq!(ordinary.to_string(), source.to_string());
        let ordinary: syn::ItemStruct = syn::parse2(ordinary).unwrap();
        let source: syn::ItemStruct = syn::parse2(source).unwrap();
        let argument = bytes.find("ActualName").unwrap();
        let occurrence = bytes.find("struct $name").unwrap() + "struct ".len();
        assert_eq!(
            ordinary.ident.span().byte_range(),
            argument..argument + "ActualName".len()
        );
        assert_eq!(
            source.ident.span().byte_range(),
            occurrence..occurrence + "$name".len()
        );
    }

    #[test]
    fn historical_expression_capture_remains_available_only_in_legacy_mode() {
        let definition =
            syn::parse_str("macro_rules! m { ($value:expr) => { fn f() { $value; } }; }").unwrap();
        let invocation: syn::ItemMacro = syn::parse_str("m!(1 + 2);").unwrap();
        assert!(expand(&definition, &invocation.mac).is_ok());
        assert!(expand_source(&definition, &invocation.mac).is_err());
    }
}
