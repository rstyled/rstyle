//! Parse the entire ordered rule roster before any arm can be selected.
use super::*;
#[cfg(test)]
#[path = "grammar_tests.rs"]
mod tests;

pub(super) struct Rule {
    pub pattern: Vec<Node>,
    pub body: Vec<Node>,
}

pub(super) fn rules(tokens: TokenStream, budget: &mut Budget) -> Result<Vec<Rule>, String> {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    let mut position = 0;
    let mut rules = Vec::new();

    while position < tokens.len() {
        budget.charge(1)?;
        rules.push(rule(tokens.get(position..position + 4), budget)?);
        position += 4;
        if position < tokens.len() {
            if !punctuation(&tokens[position], ';') {
                return Err("Ownership macro rules require complete separators".into());
            }
            position += 1;
        }
    }

    if rules.is_empty() {
        return Err("Ownership macro has no rules".into());
    }
    Ok(rules)
}

fn rule(tokens: Option<&[TokenTree]>, budget: &mut Budget) -> Result<Rule, String> {
    let Some(
        [
            TokenTree::Group(pattern),
            equal,
            arrow,
            TokenTree::Group(body),
        ],
    ) = tokens
    else {
        return Err("Ownership macro has malformed or missing declarative rules".into());
    };
    if !punctuation(equal, '=') || !punctuation(arrow, '>') {
        return Err("Ownership macro rule lacks its arrow".into());
    }

    let pattern = Grammar {
        pattern: true,
        budget,
    }
    .nodes(pattern.stream(), 0)?;
    let body = Grammar {
        pattern: false,
        budget,
    }
    .nodes(body.stream(), 0)?;

    let mut bindings = Vec::new();
    names(&pattern, &mut bindings);
    let unique = bindings.iter().collect::<std::collections::BTreeSet<_>>();
    if unique.len() != bindings.len() {
        return Err("Ownership macro duplicates a matcher binding".into());
    }

    Ok(Rule { pattern, body })
}

struct Grammar<'a> {
    pattern: bool,
    budget: &'a mut Budget,
}
impl Grammar<'_> {
    fn nodes(&mut self, tokens: TokenStream, depth: usize) -> Result<Vec<Node>, String> {
        self.budget.depth(depth)?;
        let tokens = tokens.into_iter().collect::<Vec<_>>();
        let mut output = Vec::new();
        let mut position = 0;

        while position < tokens.len() {
            self.budget.charge(1)?;
            let token = &tokens[position];
            if punctuation(token, '$') {
                output.push(self.variable(&tokens, &mut position, depth)?);
            } else if self.pattern
                && let Some((spelling, count)) = operator(&tokens[position..])
            {
                if spelling == "'" {
                    return Err("Ownership macro lifetime tokens require separate proof".into());
                }
                output.push(Node::Punctuation(spelling));
                position += count;
            } else {
                output.push(match token {
                    TokenTree::Group(group) => {
                        if group.delimiter() == Delimiter::None {
                            return Err("Ownership macro definition contains an unproved invisible fragment".into());
                        }
                        Node::Group(group.delimiter(), self.nodes(group.stream(), depth + 1)?)
                    }
                    token => Node::Token(token.clone()),
                });
                position += 1;
            }
        }

        if self.pattern {
            followers(&output)?;
        }
        Ok(output)
    }

    fn variable(
        &mut self,
        tokens: &[TokenTree],
        position: &mut usize,
        depth: usize,
    ) -> Result<Node, String> {
        match tokens.get(*position + 1) {
            Some(TokenTree::Ident(name)) => {
                let name = name.to_string();
                let (node, end) = self.capture(tokens, *position + 2, name)?;
                *position = end;
                Ok(node)
            }
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
                let body = self.nodes(group.stream(), depth + 1)?;
                let mut end = *position + 2;
                let (separator, operator) = repetition(tokens, &mut end)?;
                *position = end;
                Ok(Node::Repeat {
                    body,
                    separator,
                    operator,
                })
            }
            _ => Err("Unsupported ownership macro dollar syntax".into()),
        }
    }

    fn capture(
        &self,
        tokens: &[TokenTree],
        position: usize,
        name: String,
    ) -> Result<(Node, usize), String> {
        if !self.pattern {
            return Ok((Node::Capture(name, None), position));
        }
        if name == "crate"
            || tokens
                .get(position)
                .is_none_or(|token| !punctuation(token, ':'))
        {
            return Err("Ownership macro requires an ordinary typed matcher binding".into());
        }

        let Some(TokenTree::Ident(kind)) = tokens.get(position + 1) else {
            return Err("Ownership macro lacks a fragment kind".into());
        };
        let fragment = fragment(&kind.to_string())?;

        Ok((Node::Capture(name, Some(fragment)), position + 2))
    }
}

fn followers(nodes: &[Node]) -> Result<(), String> {
    for (index, node) in nodes.iter().enumerate() {
        if let Node::Capture(
            _,
            Some(fragment @ (Fragment::Expr | Fragment::Expr2021 | Fragment::Ty)),
        ) = node
            && !follows(*fragment, &nodes[index + 1..])
        {
            return Err("Unsupported ownership macro typed-fragment follower".into());
        }
    }
    Ok(())
}

fn follows(fragment: Fragment, tail: &[Node]) -> bool {
    match tail.first() {
        None => true,
        Some(Node::Punctuation(value)) if [",", ";", "=>"].contains(&value.as_str()) => true,
        Some(Node::Punctuation(value)) if matches!(fragment, Fragment::Ty) => {
            ["=", ":", ">", "|"].contains(&value.as_str())
        }
        Some(Node::Token(token)) if punctuation(token, ',') || punctuation(token, ';') => true,
        Some(Node::Token(token)) if punctuation(token, '=') => {
            matches!(tail.get(1), Some(Node::Token(next)) if punctuation(next, '>'))
                || matches!(fragment, Fragment::Ty)
        }
        Some(Node::Token(token)) if matches!(fragment, Fragment::Ty) => {
            matches!(token, TokenTree::Punct(punct) if matches!(punct.as_char(), ':' | '>' | '|'))
                || matches!(token, TokenTree::Ident(name) if name == "as" || name == "where")
        }
        Some(Node::Group(Delimiter::Bracket | Delimiter::Brace, _)) => {
            matches!(fragment, Fragment::Ty)
        }
        Some(Node::Repeat { body, operator, .. }) => {
            follows(fragment, body) && (*operator == '+' || follows(fragment, &tail[1..]))
        }
        _ => false,
    }
}

fn fragment(name: &str) -> Result<Fragment, String> {
    Ok(match name {
        "ident" => Fragment::Ident,
        "literal" => Fragment::Literal,
        "expr" => Fragment::Expr,
        "expr_2021" => Fragment::Expr2021,
        "ty" => Fragment::Ty,
        "tt" => Fragment::Tt,
        "vis" => Fragment::Vis,
        "meta" => Fragment::Meta,
        _ => return Err(format!("Unsupported ownership macro fragment {name}")),
    })
}

fn repetition(
    tokens: &[TokenTree],
    position: &mut usize,
) -> Result<(Option<TokenTree>, char), String> {
    let mut separator = None;
    loop {
        let token = tokens
            .get(*position)
            .ok_or("Missing ownership macro repetition operator")?;
        *position += 1;
        if let TokenTree::Punct(punct) = token
            && matches!(punct.as_char(), '*' | '+' | '?')
        {
            if punct.as_char() == '?' && separator.is_some() {
                return Err("Optional ownership repetition cannot have a separator".into());
            }
            return Ok((separator, punct.as_char()));
        }
        if separator.is_some() || matches!(token, TokenTree::Group(_)) || punctuation(token, '$') {
            return Err("Unsupported ownership macro repetition separator".into());
        }
        separator = Some(token.clone());
    }
}
