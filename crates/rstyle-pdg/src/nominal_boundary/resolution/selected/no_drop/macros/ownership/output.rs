//! Preserve every selected template token and every captured nested item.
use super::*;
#[cfg(test)]
mod tests;

pub(super) fn expand(
    nodes: &[Node],
    captures: &Captures,
    budget: &mut Budget,
) -> Result<TokenStream, String> {
    let output = Renderer { captures, budget }.render(nodes, &[], 0)?;
    // A captured subtree may become deeper when inserted into the template.
    // Check the assembled depth once, without recounting emitted output.
    budget.tokens(output.clone(), 0)?;
    Ok(output)
}

struct Renderer<'a> {
    captures: &'a Captures,
    budget: &'a mut Budget,
}
struct Repetition<'a> {
    body: &'a [Node],
    separator: Option<&'a TokenTree>,
    operator: char,
}

impl Renderer<'_> {
    fn render(
        &mut self,
        nodes: &[Node],
        indices: &[usize],
        depth: usize,
    ) -> Result<TokenStream, String> {
        self.budget.depth(depth)?;
        let mut output = TokenStream::new();
        for node in nodes {
            self.budget.charge(1)?;
            let piece = self.piece(node, indices, depth)?;
            output.extend(piece);
        }
        Ok(output)
    }

    fn piece(
        &mut self,
        node: &Node,
        indices: &[usize],
        depth: usize,
    ) -> Result<TokenStream, String> {
        match node {
            Node::Token(_) | Node::Capture(_, None) => self.atom(node, indices),
            Node::Punctuation(_) => Err("Matcher punctuation appeared in ownership output".into()),
            Node::Group(_, _) => self.group(node, indices, depth),
            Node::Repeat {
                body,
                separator,
                operator,
            } => self.repeated(
                Repetition {
                    body,
                    separator: separator.as_ref(),
                    operator: *operator,
                },
                indices,
                depth + 1,
            ),
            _ => Err("Typed binding appeared in ownership macro output".into()),
        }
    }

    fn group(
        &mut self,
        node: &Node,
        indices: &[usize],
        depth: usize,
    ) -> Result<TokenStream, String> {
        let Node::Group(delimiter, body) = node else {
            return Err("Non-group ownership output".into());
        };
        self.budget.output_tokens(1)?;
        Ok(TokenTree::Group(Group::new(
            *delimiter,
            self.render(body, indices, depth + 1)?,
        ))
        .into())
    }

    fn atom(&mut self, node: &Node, indices: &[usize]) -> Result<TokenStream, String> {
        let tokens = match node {
            Node::Token(token) => token.clone().into(),
            Node::Capture(name, None) if name == "crate" => TokenTree::Ident(
                proc_macro2::Ident::new("crate", proc_macro2::Span::call_site()),
            )
            .into(),
            Node::Capture(name, None) => match selected(self.captures, name, indices)? {
                Value::Atom(tokens) => tokens.clone(),
                Value::Many(_) => {
                    return Err("Ownership capture remains repeated at this template depth".into());
                }
            },
            _ => return Err("Non-atomic ownership output".into()),
        };

        self.budget.emit(tokens)
    }

    fn repeated(
        &mut self,
        repetition: Repetition<'_>,
        indices: &[usize],
        depth: usize,
    ) -> Result<TokenStream, String> {
        let length = repeated_length(repetition.body, self.captures, indices)?;
        if repetition.operator == '+' && length == 0 || repetition.operator == '?' && length > 1 {
            return Err("Ownership output repetition cardinality disagrees".into());
        }

        let mut output = TokenStream::new();
        for index in 0..length {
            self.budget.charge(1)?;
            if index > 0
                && let Some(separator) = repetition.separator
            {
                output.extend(self.budget.emit(separator.clone().into())?);
            }
            let nested: Vec<_> = indices
                .iter()
                .copied()
                .chain(std::iter::once(index))
                .collect();
            output.extend(self.render(repetition.body, &nested, depth + 1)?);
        }
        Ok(output)
    }
}

fn selected<'a>(
    captures: &'a Captures,
    name: &str,
    indices: &[usize],
) -> Result<&'a Value, String> {
    let mut value = captures
        .get(name)
        .ok_or("Unbound ownership macro output capture")?;
    for index in indices {
        if let Value::Many(values) = value {
            value = values
                .get(*index)
                .ok_or("Ownership repetition index differs from captured shape")?;
        }
    }
    Ok(value)
}

fn repeated_length(body: &[Node], captures: &Captures, indices: &[usize]) -> Result<usize, String> {
    let mut variables = Vec::new();
    names(body, &mut variables);

    let mut length = None;
    for name in variables {
        if let Value::Many(values) = selected(captures, &name, indices)? {
            if length.is_some_and(|length| length != values.len()) {
                return Err("Ownership macro repetition capture lengths disagree".into());
            }
            length = Some(values.len());
        }
    }

    length.ok_or_else(|| "Ownership output repetition lacks a captured driver".into())
}
