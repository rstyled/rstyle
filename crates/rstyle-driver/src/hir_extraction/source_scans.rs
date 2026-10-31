//! Source scan lookup preserves the first match in original scan order.
use super::source;
use rustc_span::Span;
use std::collections::HashMap;

pub(super) struct SourceScans<'a> {
    functions: &'a [source::Function],
    indexed: Option<Indexed>,
}

struct Indexed {
    signatures: HashMap<Span, usize>,
    bodies: HashMap<Span, usize>,
}

impl<'a> SourceScans<'a> {
    pub(super) fn new(functions: &'a [source::Function]) -> Self {
        let mut signatures = HashMap::new();
        let mut bodies = HashMap::new();
        for (index, function) in functions.iter().enumerate() {
            signatures
                .entry(function.span.source_callsite())
                .or_insert(index);
            bodies
                .entry(function.body_span.source_callsite())
                .or_insert(index);
        }
        Self {
            functions,
            indexed: Some(Indexed { signatures, bodies }),
        }
    }

    pub(super) fn linear(functions: &'a [source::Function]) -> Self {
        Self {
            functions,
            indexed: None,
        }
    }

    pub(super) fn find(
        &self,
        signature: Span,
        body: Span,
        conditional: bool,
    ) -> Option<&'a source::Function> {
        let Some(indexed) = &self.indexed else {
            return self.functions.iter().find(|scan| {
                scan.span.source_callsite() == signature
                    || (conditional && scan.body_span.source_callsite() == body)
            });
        };
        let signature = indexed.signatures.get(&signature).copied();
        let body = conditional
            .then(|| indexed.bodies.get(&body).copied())
            .flatten();
        signature
            .into_iter()
            .chain(body)
            .min()
            .map(|index| &self.functions[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_span::BytePos;

    fn span(start: u32) -> Span {
        Span::with_root_ctxt(BytePos(start), BytePos(start + 1))
    }

    fn function(signature: u32, body: u32) -> source::Function {
        source::Function {
            span: span(signature),
            body_span: span(body),
            findings: Vec::new(),
            conditional_source: false,
        }
    }

    #[test]
    fn indexed_lookup_preserves_earliest_or_match_and_profile_gating() {
        rustc_span::create_default_session_globals_then(|| {
            let functions = [function(1, 20), function(2, 21), function(2, 20)];
            let indexed = SourceScans::new(&functions);
            let linear = SourceScans::linear(&functions);
            for conditional in [false, true] {
                for signature in [1, 2, 9] {
                    for body in [20, 21, 29] {
                        assert_eq!(
                            indexed
                                .find(span(signature), span(body), conditional)
                                .map(std::ptr::from_ref),
                            linear
                                .find(span(signature), span(body), conditional)
                                .map(std::ptr::from_ref)
                        );
                    }
                }
            }
            assert!(std::ptr::eq(
                indexed.find(span(2), span(20), true).unwrap(),
                &functions[0]
            ));
            assert!(std::ptr::eq(
                indexed.find(span(2), span(20), false).unwrap(),
                &functions[1]
            ));
            assert!(indexed.find(span(9), span(20), false).is_none());
        });
    }
}
