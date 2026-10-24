//! One syn token buffer per arm; typed fragments consume only their own prefix.
use super::*;
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseStream, Parser, discouraged::Speculative},
};
#[cfg(test)]
mod tests;

pub(super) fn complete(
    pattern: &[Node],
    input: &[TokenTree],
    context: &Context<'_>,
    budget: &mut Budget,
) -> Result<Captures, Failure> {
    let input = input.iter().cloned().collect::<TokenStream>();
    budget.tokens(input.clone(), 0)?;
    Matcher { context, budget }.parse(pattern, input, 0)
}

struct Matcher<'a, 'b> {
    context: &'a Context<'b>,
    budget: &'a mut Budget,
}
enum Step {
    Empty,
    Binding(String, Value),
    Nested(Captures),
}

struct Repetition<'a> {
    body: &'a [Node],
    separator: Option<&'a TokenTree>,
    operator: char,
    tail: &'a [Node],
}

impl Matcher<'_, '_> {
    fn parse(
        &mut self,
        pattern: &[Node],
        input: TokenStream,
        depth: usize,
    ) -> Result<Captures, Failure> {
        self.budget.depth(depth)?;
        (|stream: ParseStream<'_>| {
            let result = self.sequence(pattern, stream, depth).and_then(|captures| {
                if stream.is_empty() {
                    Ok(captures)
                } else {
                    Err(Failure::Mismatch)
                }
            });
            let _: TokenStream = stream.parse()?;
            Ok(result)
        })
        .parse2(input)
        .map_err(|error| {
            Failure::Unsupported(format!("Ownership macro token parser failed: {error}"))
        })?
    }

    fn sequence(
        &mut self,
        pattern: &[Node],
        input: ParseStream<'_>,
        depth: usize,
    ) -> Result<Captures, Failure> {
        self.budget.depth(depth)?;
        let mut captures = Captures::new();
        for index in 0..pattern.len() {
            self.budget.charge(1)?;
            match self.step(&pattern[index..], input, depth)? {
                Step::Empty => {}
                Step::Binding(name, value) => {
                    captures.insert(name, value);
                }
                Step::Nested(nested) => captures.extend(nested),
            }
        }
        Ok(captures)
    }

    fn step(
        &mut self,
        pattern: &[Node],
        input: ParseStream<'_>,
        depth: usize,
    ) -> Result<Step, Failure> {
        match &pattern[0] {
            Node::Token(expected) => {
                if !token(expected, input)? {
                    return Err(Failure::Mismatch);
                }
            }
            Node::Punctuation(expected) => {
                if read_operator(input)? != *expected {
                    return Err(Failure::Mismatch);
                }
            }
            Node::Group(delimiter, body) => {
                return self
                    .parse(body, group_input(*delimiter, input)?, depth + 1)
                    .map(Step::Nested);
            }
            Node::Capture(name, Some(fragment)) => {
                return Ok(Step::Binding(
                    name.clone(),
                    Value::Atom(self.capture(*fragment, input)?),
                ));
            }
            Node::Repeat {
                body,
                separator,
                operator,
            } => {
                let repetition = Repetition {
                    body,
                    separator: separator.as_ref(),
                    operator: *operator,
                    tail: &pattern[1..],
                };
                return self.repeat(&repetition, input, depth).map(Step::Nested);
            }
            _ => {
                return Err(Failure::Unsupported(
                    "Untyped ownership matcher binding".into(),
                ));
            }
        }

        Ok(Step::Empty)
    }

    fn repeat(
        &mut self,
        repetition: &Repetition<'_>,
        input: ParseStream<'_>,
        depth: usize,
    ) -> Result<Captures, Failure> {
        let bindings = binding_names(repetition.body);
        let mut values: Captures = bindings
            .iter()
            .map(|name| (name.clone(), Value::Many(Vec::new())))
            .collect();
        let mut count = 0;

        loop {
            self.budget.charge(1)?;
            if repetition.operator == '?' && count == 1 {
                break;
            }
            let fork = input.fork();
            self.reject_ambiguity(repetition, input)?;
            if count > 0
                && let Some(separator) = repetition.separator
                && !token(separator, &fork)?
            {
                break;
            }
            let start = fork.cursor();
            let captures = match self.sequence(repetition.body, &fork, depth + 1) {
                Ok(value) => value,
                Err(Failure::Mismatch) => break,
                Err(error) => return Err(error),
            };
            if fork.cursor() == start {
                return Err(Failure::Unsupported(
                    "Ownership repetition matches empty input".into(),
                ));
            }
            count += 1;
            if count > 512 {
                return Err(Failure::Unsupported(
                    "Ownership macro repetition budget exceeded".into(),
                ));
            }
            append(&mut values, &bindings, &captures)?;
            input.advance_to(&fork);
        }

        if repetition.operator == '+' && count == 0 {
            return Err(Failure::Mismatch);
        }
        Ok(values)
    }

    fn reject_ambiguity(
        &mut self,
        repetition: &Repetition<'_>,
        input: ParseStream<'_>,
    ) -> Result<(), Failure> {
        if repetition.tail.is_empty() || trailing_separator(repetition) {
            return Ok(());
        }
        if !self.starts(repetition.tail, input)? {
            return Ok(());
        }

        let fork = input.fork();
        if let Some(separator) = repetition.separator
            && !token(separator, &fork)?
        {
            return Ok(());
        }

        if self.starts(repetition.body, &fork)? {
            return Err(Failure::Unsupported(
                "Ambiguous ownership macro repetition boundary".into(),
            ));
        }
        Ok(())
    }

    fn starts(&mut self, pattern: &[Node], input: ParseStream<'_>) -> Result<bool, Failure> {
        self.budget.charge(1)?;
        let Some(node) = pattern.first() else {
            return Ok(false);
        };
        let fork = input.fork();
        match node {
            Node::Token(expected) => token(expected, &fork),
            Node::Punctuation(expected) => match read_operator(&fork) {
                Ok(actual) => Ok(actual == *expected),
                Err(Failure::Mismatch) => Ok(false),
                Err(error) => Err(error),
            },
            Node::Group(delimiter, _) => Ok(
                matches!(fork.parse::<TokenTree>(), Ok(TokenTree::Group(group)) if group.delimiter() == *delimiter),
            ),
            Node::Capture(_, Some(_)) => self.capture_start(pattern, input),
            Node::Repeat { .. } => self.repeat_start(pattern, input),
            _ => Err(Failure::Unsupported(
                "Unsupported ownership matcher start".into(),
            )),
        }
    }

    fn capture_start(&mut self, pattern: &[Node], input: ParseStream<'_>) -> Result<bool, Failure> {
        let Node::Capture(_, Some(fragment)) = &pattern[0] else {
            unreachable!();
        };
        let fork = input.fork();
        match self.capture(*fragment, &fork) {
            Ok(_) if matches!(fragment, Fragment::Vis) && fork.cursor() == input.cursor() => {
                self.starts(&pattern[1..], input)
            }
            Ok(_) => Ok(true),
            Err(Failure::Mismatch) => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn repeat_start(&mut self, pattern: &[Node], input: ParseStream<'_>) -> Result<bool, Failure> {
        let Node::Repeat { body, operator, .. } = &pattern[0] else {
            unreachable!();
        };
        if self.starts(body, input)? {
            return Ok(true);
        }
        if *operator != '+' {
            return self.starts(&pattern[1..], input);
        }
        Ok(false)
    }

    fn capture(
        &mut self,
        fragment: Fragment,
        input: ParseStream<'_>,
    ) -> Result<TokenStream, Failure> {
        self.budget.charge(1)?;
        if input.is_empty() && !matches!(fragment, Fragment::Vis) {
            return Err(Failure::Mismatch);
        }
        // These punctuation tokens cannot begin a Rust type. This is a
        // fragment mismatch, including in ordered arms and repetition
        // lookahead. A possible type prefix still receives a complete parse;
        // unsupported or malformed syntax remains terminal.
        if matches!(fragment, Fragment::Ty)
            && (input.peek(syn::Token![#]) || input.peek(syn::Token![@]))
        {
            return Err(Failure::Mismatch);
        }
        if matches!(fragment, Fragment::Tt) {
            return input
                .parse::<TokenTree>()
                .map(Into::into)
                .map_err(|_| Failure::Mismatch);
        }

        if let Some((TokenTree::Group(group), _)) = input.cursor().token_tree()
            && group.delimiter() == Delimiter::None
        {
            if marker(&group) != kind(fragment) {
                return Err(Failure::Unsupported(
                    "Ownership macro opaque fragment kind transition requires independent proof"
                        .into(),
                ));
            }
            if matches!(fragment, Fragment::Literal | Fragment::Meta | Fragment::Vis) {
                let token: TokenTree = input.parse().map_err(fragment_error)?;
                return Ok(token.into());
            }
        }

        Ok(wrap_fragment(
            fragment,
            self.fragment_tokens(fragment, input)?,
        ))
    }

    fn fragment_tokens(
        &mut self,
        fragment: Fragment,
        input: ParseStream<'_>,
    ) -> Result<TokenStream, Failure> {
        let start = input.cursor();
        self.fragment(fragment, input)?;
        let mut tokens = TokenStream::new();
        let mut cursor = start;

        while cursor != input.cursor() {
            self.budget.charge(1)?;
            let (token, next) = cursor.token_tree().ok_or_else(|| {
                Failure::Unsupported("Ownership fragment boundary is not a token boundary".into())
            })?;
            tokens.extend([token]);
            cursor = next;
        }

        Ok(tokens)
    }

    fn fragment(&mut self, fragment: Fragment, input: ParseStream<'_>) -> Result<(), Failure> {
        match fragment {
            Fragment::Ident => {
                syn::Ident::parse_any(input).map_err(|_| Failure::Mismatch)?;
            }
            Fragment::Literal => {
                input.parse::<Literal>().map_err(|_| Failure::Mismatch)?;
            }
            Fragment::Expr | Fragment::Expr2021 => {
                let old = self.context.edition != "2024" || matches!(fragment, Fragment::Expr2021);
                if old && (input.peek(syn::Token![_]) || input.peek(syn::Token![const])) {
                    return Err(Failure::Mismatch);
                }
                let expression = input.parse::<syn::Expr>().map_err(fragment_error)?;
                if matches!(expression, syn::Expr::Let(_) | syn::Expr::Verbatim(_)) {
                    return Err(Failure::Unsupported(
                        "Ownership macro expression lies outside the checked Rust fragment grammar"
                            .into(),
                    ));
                }
            }
            Fragment::Ty => {
                input.parse::<syn::Type>().map_err(fragment_error)?;
            }
            Fragment::Vis => {
                input.parse::<syn::Visibility>().map_err(fragment_error)?;
            }
            Fragment::Meta => {
                input.parse::<syn::Meta>().map_err(fragment_error)?;
            }
            Fragment::Tt => unreachable!(),
        }
        Ok(())
    }
}

fn binding_names(body: &[Node]) -> Vec<String> {
    let mut bindings = Vec::new();
    names(body, &mut bindings);
    bindings
}

fn group_input(delimiter: Delimiter, input: ParseStream<'_>) -> Result<TokenStream, Failure> {
    let actual: TokenTree = input.parse().map_err(|_| Failure::Mismatch)?;
    let TokenTree::Group(group) = actual else {
        return Err(Failure::Mismatch);
    };
    if group.delimiter() != delimiter {
        return Err(Failure::Mismatch);
    }
    Ok(group.stream())
}

fn wrap_fragment(fragment: Fragment, mut tokens: TokenStream) -> TokenStream {
    if let [TokenTree::Group(group)] = tokens.clone().into_iter().collect::<Vec<_>>().as_slice()
        && group.delimiter() == Delimiter::None
        && marker(group) == kind(fragment)
    {
        return tokens;
    }

    for _ in 0..kind(fragment) {
        tokens = TokenTree::Group(Group::new(Delimiter::None, tokens)).into();
    }
    tokens
}

fn kind(fragment: Fragment) -> usize {
    match fragment {
        Fragment::Ident | Fragment::Tt => 0,
        Fragment::Expr | Fragment::Expr2021 => 1,
        Fragment::Ty => 2,
        Fragment::Literal => 3,
        Fragment::Meta => 4,
        Fragment::Vis => 5,
    }
}

fn marker(group: &Group) -> usize {
    let mut count = 1;
    let mut tokens = group.stream().into_iter().collect::<Vec<_>>();
    while let [TokenTree::Group(nested)] = tokens.as_slice() {
        if nested.delimiter() != Delimiter::None {
            break;
        }
        count += 1;
        if count > 5 {
            break;
        }
        tokens = nested.stream().into_iter().collect();
    }
    count
}

fn append(values: &mut Captures, bindings: &[String], captures: &Captures) -> Result<(), Failure> {
    for name in bindings {
        let value = captures
            .get(name)
            .ok_or_else(|| Failure::Unsupported("Missing ownership repetition binding".into()))?;
        let Some(Value::Many(items)) = values.get_mut(name) else {
            return Err(Failure::Unsupported(
                "Invalid ownership repetition capture".into(),
            ));
        };
        items.push(value.clone());
    }
    Ok(())
}

fn equal(expected: &TokenTree, actual: &TokenTree) -> Result<bool, Failure> {
    Ok(match (expected, actual) {
        (TokenTree::Ident(a), TokenTree::Ident(b)) => {
            if a != b
                && a.to_string().trim_start_matches("r#") == b.to_string().trim_start_matches("r#")
            {
                return Err(Failure::Unsupported(
                    "Ownership macro raw identifier matching requires exact spelling".into(),
                ));
            }
            a == b
        }
        (TokenTree::Punct(a), TokenTree::Punct(b)) => a.as_char() == b.as_char(),
        (TokenTree::Literal(a), TokenTree::Literal(b)) => a.to_string() == b.to_string(),
        _ => false,
    })
}

fn trailing_separator(repetition: &Repetition<'_>) -> bool {
    let Some(separator) = repetition.separator else {
        return false;
    };
    matches!(repetition.tail, [Node::Repeat { body, separator: None, operator: '?' }]
        if matches!(body.as_slice(), [Node::Punctuation(value)] if separator.to_string() == *value)
        || matches!(body.as_slice(), [Node::Token(token)] if matches!(equal(separator, token), Ok(true))))
}

fn token(expected: &TokenTree, input: ParseStream<'_>) -> Result<bool, Failure> {
    if let TokenTree::Punct(punct) = expected {
        return match read_operator(input) {
            Ok(actual) => Ok(actual == punct.as_char().to_string()),
            Err(Failure::Mismatch) => Ok(false),
            Err(error) => Err(error),
        };
    }
    let Ok(actual) = input.parse::<TokenTree>() else {
        return Ok(false);
    };
    equal(expected, &actual)
}

fn read_operator(input: ParseStream<'_>) -> Result<String, Failure> {
    let mut cursor = input.cursor();
    let mut tokens = Vec::new();
    for _ in 0..3 {
        let Some((token, next)) = cursor.token_tree() else {
            break;
        };
        tokens.push(token);
        cursor = next;
    }

    let (spelling, count) = operator(&tokens).ok_or(Failure::Mismatch)?;
    for _ in 0..count {
        input.parse::<TokenTree>().map_err(fragment_error)?;
    }

    Ok(spelling)
}

fn fragment_error(error: syn::Error) -> Failure {
    Failure::Unsupported(format!(
        "Ownership macro fragment requires unsupported or malformed syntax: {error}"
    ))
}

struct Literal;
impl Parse for Literal {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let negative = input.parse::<Option<syn::Token![-]>>()?.is_some();
        let literal: syn::Lit = input.parse()?;
        if negative && !matches!(literal, syn::Lit::Int(_) | syn::Lit::Float(_)) {
            return Err(input.error("Negative literal requires a numeric literal"));
        }
        Ok(Self)
    }
}
