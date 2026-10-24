//! Bounded ownership-only expansion of already source-proved local definitions.
//! Tokens retain opaque typed-fragment groups; callers must not stringify them.
mod grammar;
mod matching;
mod output;
#[cfg(test)]
mod tests;

use proc_macro2::{Delimiter, Group, TokenStream, TokenTree};
use std::collections::BTreeMap;

pub(super) struct Context<'a> {
    pub edition: &'a str,
}

pub(super) fn expand(
    definition: &syn::ItemMacro,
    invocation: &syn::Macro,
    context: Context<'_>,
) -> Result<TokenStream, String> {
    if !matches!(context.edition, "2021" | "2024")
        || !definition.mac.path.is_ident("macro_rules")
        || definition.ident.is_none()
    {
        return Err(
            "Ownership macro requires a local declarative definition and supported edition".into(),
        );
    }

    let mut budget = Budget::inputs(definition, invocation)?;
    let rules = grammar::rules(definition.mac.tokens.clone(), &mut budget)?;
    let input = invocation.tokens.clone().into_iter().collect::<Vec<_>>();

    select(&rules, &input, &context, &mut budget)
}

fn select(
    rules: &[grammar::Rule],
    input: &[TokenTree],
    context: &Context<'_>,
    budget: &mut Budget,
) -> Result<TokenStream, String> {
    for rule in rules {
        match matching::complete(&rule.pattern, input, context, budget) {
            Ok(captures) => return output::expand(&rule.body, &captures, budget),
            Err(Failure::Mismatch) => {}
            Err(Failure::Unsupported(message)) => return Err(message),
        }
    }

    Err("Ownership macro has no matching complete rule".into())
}

#[derive(Clone)]
enum Node {
    Token(TokenTree),
    Punctuation(String),
    Group(Delimiter, Vec<Node>),
    Capture(String, Option<Fragment>),
    Repeat {
        body: Vec<Node>,
        separator: Option<TokenTree>,
        operator: char,
    },
}

fn compound(value: &str) -> bool {
    [
        "=>", "->", "::", "..", "..=", "...", "<<", ">>", "<<=", ">>=", "+=", "-=", "*=", "/=",
        "%=", "^=", "&=", "|=", "&&", "||", "<=", ">=", "==", "!=",
    ]
    .contains(&value)
}

fn operator(tokens: &[TokenTree]) -> Option<(String, usize)> {
    let TokenTree::Punct(first) = tokens.first()? else {
        return None;
    };
    let mut spelling = first.as_char().to_string();
    let mut count = 1;
    let mut previous = first;

    while previous.spacing() == proc_macro2::Spacing::Joint && count < 3 {
        let Some(TokenTree::Punct(next)) = tokens.get(count) else {
            break;
        };
        let candidate = format!("{spelling}{}", next.as_char());
        if !compound(&candidate) {
            break;
        }

        spelling = candidate;
        count += 1;
        previous = next;
    }

    Some((spelling, count))
}

#[derive(Clone, Copy)]
enum Fragment {
    Ident,
    Literal,
    Expr,
    Expr2021,
    Ty,
    Tt,
    Vis,
    Meta,
}

#[derive(Clone)]
enum Value {
    Atom(TokenStream),
    Many(Vec<Value>),
}
type Captures = BTreeMap<String, Value>;

enum Failure {
    Mismatch,
    Unsupported(String),
}
impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::Unsupported(message)
    }
}

struct Budget {
    remaining: usize,
    emitted: usize,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            remaining: 131_072,
            emitted: 0,
        }
    }
}
impl Budget {
    fn inputs(definition: &syn::ItemMacro, invocation: &syn::Macro) -> Result<Self, String> {
        let mut budget = Self::default();
        budget.tokens(definition.mac.tokens.clone(), 0)?;
        budget.tokens(invocation.tokens.clone(), 0)?;

        Ok(budget)
    }

    fn charge(&mut self, work: usize) -> Result<(), String> {
        self.remaining = self
            .remaining
            .checked_sub(work)
            .ok_or("Ownership macro work budget exceeded")?;
        Ok(())
    }
    fn tokens(&mut self, tokens: TokenStream, depth: usize) -> Result<(), String> {
        self.depth(depth)?;
        for token in tokens {
            self.charge(1)?;
            if let TokenTree::Group(group) = token {
                self.tokens(group.stream(), depth + 1)?;
            }
        }
        Ok(())
    }
    fn depth(&self, depth: usize) -> Result<(), String> {
        if depth > 48 {
            Err("Ownership macro nesting budget exceeded".into())
        } else {
            Ok(())
        }
    }
    fn emit(&mut self, tokens: TokenStream) -> Result<TokenStream, String> {
        let before = self.remaining;
        self.tokens(tokens.clone(), 0)?;

        self.output_tokens(before - self.remaining)?;
        Ok(tokens)
    }

    // Count each output token once, when created or copied from a capture.
    // Reassembling a group or repetition must not recount its descendants.
    fn output_tokens(&mut self, count: usize) -> Result<(), String> {
        self.emitted = self
            .emitted
            .checked_add(count)
            .ok_or("Ownership macro output overflow")?;
        if self.emitted > 32_768 {
            return Err("Ownership macro output budget exceeded".into());
        }

        self.charge(1)?;
        Ok(())
    }
}

fn punctuation(token: &TokenTree, expected: char) -> bool {
    matches!(token, TokenTree::Punct(value) if value.as_char() == expected)
}

fn names(nodes: &[Node], result: &mut Vec<String>) {
    for node in nodes {
        match node {
            Node::Capture(name, _) if name != "crate" => result.push(name.clone()),
            Node::Group(_, body) | Node::Repeat { body, .. } => names(body, result),
            _ => {}
        }
    }
}
