//! Bind lexical arm scope to both original syntax and the owning recorded arm.
use super::*;
use crate::frontend::ByteSpan;
use syn::visit::{self, Visit};

pub(super) fn verify(root: &HirNode, facts: &dyn EvidenceFacts) -> Result<String, String> {
    let syntax: syn::ItemFn = syn::parse_str(&facts.source.bytes)
        .map_err(|error| format!("Unknown original pattern owner source: {error}"))?;
    let mut original = Arms {
        source: &facts.source.bytes,
        pattern: root.span,
        spans: Vec::new(),
    };
    original.visit_block(&syntax.block);
    let [span] = original.spans.as_slice() else {
        return Err(unsupported("Pattern has no unique original match arm"));
    };

    let mut recorded = Vec::new();
    recorded_arms(&facts.body, root, &mut recorded);
    let [arm] = recorded.as_slice() else {
        return Err(unsupported("Pattern has no unique recorded match arm"));
    };
    if arm.span != *span || arm.children.first() != Some(root) || arm.expansion.is_some() {
        return Err(unsupported(
            "Original match arm and recorded scope disagree",
        ));
    }

    Ok(format!(
        "scope:source:slice-arm:{}:{}",
        span.start, span.end
    ))
}

struct Arms<'a> {
    source: &'a str,
    pattern: ByteSpan,
    spans: Vec<ByteSpan>,
}

impl<'ast> Visit<'ast> for Arms<'_> {
    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        let span = crate::source_evidence::original_span(self.source, arm.pat.span());
        if span == self.pattern {
            let body = crate::source_evidence::original_span(self.source, arm.body.span());
            if arm.attrs.is_empty() {
                self.spans.push(ByteSpan {
                    start: span.start,
                    end: body.end,
                });
            }
        }
        visit::visit_arm(self, arm);
    }
}

fn recorded_arms<'a>(node: &'a HirNode, root: &HirNode, found: &mut Vec<&'a HirNode>) {
    if node.kind == "match_arm"
        && node
            .children
            .first()
            .is_some_and(|child| child.hir == root.hir)
    {
        found.push(node);
    }
    for child in &node.children {
        recorded_arms(child, root, found);
    }
}
