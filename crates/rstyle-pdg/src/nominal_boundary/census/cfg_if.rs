//! One exact local cfg-if body; callers independently prove definition linkage.
use crate::conditional_source::Configuration;
use proc_macro2::{TokenStream, TokenTree};
use syn::{
    Attribute, Meta, Token, braced,
    parse::{Parse, ParseStream},
};

const DEFINITION: &str = include_str!("cfg_if/definition.rs");

/// This proves the complete expansion rule, never the invocation's binding.
pub(super) fn verify_definition(definition: &syn::ItemMacro) -> Result<(), String> {
    let expected: syn::ItemMacro = syn::parse_str(DEFINITION).map_err(|error| error.to_string())?;
    if !definition.mac.path.is_ident("macro_rules")
        || definition.ident != expected.ident
        || !matches!(definition.mac.delimiter, syn::MacroDelimiter::Brace(_))
        || definition
            .attrs
            .iter()
            .any(|attribute| !attribute.path().is_ident("doc"))
        || !tokens_equal(definition.mac.tokens.clone(), expected.mac.tokens)
    {
        return Err(
            "Module expansion requires the complete checked local cfg-if definition".into(),
        );
    }
    Ok(())
}

fn tokens_equal(left: TokenStream, right: TokenStream) -> bool {
    let left: Vec<_> = left.into_iter().collect();
    let right: Vec<_> = right.into_iter().collect();
    left.len() == right.len()
        && left
            .into_iter()
            .zip(right)
            .all(|(left, right)| token_equal(left, right))
}

fn token_equal(left: TokenTree, right: TokenTree) -> bool {
    match (left, right) {
        (TokenTree::Group(left), TokenTree::Group(right)) => {
            left.delimiter() == right.delimiter() && tokens_equal(left.stream(), right.stream())
        }
        (TokenTree::Ident(left), TokenTree::Ident(right)) => left == right,
        (TokenTree::Punct(left), TokenTree::Punct(right)) => {
            left.as_char() == right.as_char() && left.spacing() == right.spacing()
        }
        (TokenTree::Literal(left), TokenTree::Literal(right)) => {
            left.to_string() == right.to_string()
        }
        _ => false,
    }
}

/// Whole selected branch and its zero-based public-branch position. An inactive
/// invocation has no selection; has_else is meaningful for active invocations.
pub(super) struct Selection {
    pub items: Vec<syn::Item>,
    pub index: Option<usize>,
    pub has_else: bool,
}

pub(super) fn select_with_index(
    invocation: &syn::ItemMacro,
    configuration: &Configuration<'_>,
) -> Result<Selection, String> {
    if invocation.ident.is_some()
        || !invocation.mac.path.is_ident("cfg_if")
        || !matches!(invocation.mac.delimiter, syn::MacroDelimiter::Brace(_))
    {
        return Err("Module cfg-if requires ordinary original invocation syntax".into());
    }

    if !configuration.active(&invocation.attrs)? {
        return Ok(Selection {
            items: Vec::new(),
            index: None,
            has_else: false,
        });
    }

    invocation_attributes(invocation, configuration)?;
    let branches: Branches =
        syn::parse2(invocation.mac.tokens.clone()).map_err(|error| error.to_string())?;
    let has_else = branches
        .0
        .last()
        .is_some_and(|branch| branch.condition.is_none());

    let selected = select_branch(branches, configuration)?;
    let Some((index, tokens)) = selected else {
        return Ok(Selection {
            items: Vec::new(),
            index: None,
            has_else,
        });
    };

    let items: Items = syn::parse2(tokens).map_err(|error| error.to_string())?;
    if items
        .0
        .iter()
        .any(|item| !matches!(item, syn::Item::Mod(_) | syn::Item::Use(_)))
    {
        return Err("Selected cfg-if output requires only complete module and use items".into());
    }

    Ok(Selection {
        items: items.0,
        index: Some(index),
        has_else,
    })
}

#[cfg(test)]
pub(super) fn select(
    invocation: &syn::ItemMacro,
    configuration: &Configuration<'_>,
) -> Result<Vec<syn::Item>, String> {
    select_with_index(invocation, configuration).map(|selection| selection.items)
}

fn invocation_attributes(
    invocation: &syn::ItemMacro,
    configuration: &Configuration<'_>,
) -> Result<(), String> {
    for attribute in configuration.effective_attributes(&invocation.attrs)? {
        if !["cfg", "doc", "allow", "warn", "deny", "forbid"]
            .iter()
            .any(|name| attribute.path().is_ident(name))
        {
            return Err("Module cfg-if invocation has unsupported active attributes".into());
        }
    }
    Ok(())
}

fn select_branch(
    branches: Branches,
    configuration: &Configuration<'_>,
) -> Result<Option<(usize, TokenStream)>, String> {
    let mut selected = None;
    for (index, branch) in branches.0.into_iter().enumerate() {
        let active = match &branch.condition {
            Some(condition) => configuration.predicate(condition)?,
            None => true,
        };
        if active && selected.is_none() {
            selected = Some((index, branch.tokens));
        }
    }
    Ok(selected)
}

struct Branch {
    condition: Option<Meta>,
    tokens: TokenStream,
}
struct Branches(Vec<Branch>);

impl Parse for Branches {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut branches = Vec::new();
        loop {
            input.parse::<Token![if]>()?;
            let attributes = input.call(Attribute::parse_outer)?;
            let [attribute] = attributes.as_slice() else {
                return Err(input.error("cfg-if branch requires exactly one cfg predicate"));
            };
            if !attribute.path().is_ident("cfg") {
                return Err(input.error("cfg-if branch requires cfg syntax"));
            }
            let condition = attribute.parse_args::<Meta>()?;
            branches.push(Branch {
                condition: Some(condition),
                tokens: body(input)?,
            });
            if branches.len() > super::MAX_FILES {
                return Err(input.error("cfg-if branch count exceeds source census budget"));
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![else]>()?;
            if input.peek(Token![if]) {
                continue;
            }
            branches.push(Branch {
                condition: None,
                tokens: body(input)?,
            });
            if !input.is_empty() {
                return Err(input.error("Unexpected tokens after cfg-if else"));
            }
            break;
        }
        Ok(Self(branches))
    }
}

fn body(input: ParseStream<'_>) -> syn::Result<TokenStream> {
    let content;
    braced!(content in input);
    content.parse()
}

struct Items(Vec<syn::Item>);
impl Parse for Items {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse()?);
        }
        Ok(Self(items))
    }
}

#[cfg(test)]
mod tests;
