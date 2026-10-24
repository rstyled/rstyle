//! Preserve all nested constructs, including unsupported neighboring patterns.
use super::*;
use crate::hir_extraction::hir::reference;
use rustc_ast::LitKind;

impl<'tcx> Context<'tcx> {
    pub(super) fn slice_pattern_children(
        &self,
        pattern: &hir::Pat<'tcx>,
        rest: bool,
    ) -> Vec<HirNode> {
        match pattern.kind {
            PatKind::Slice(prefix, rest, suffix) => prefix
                .iter()
                .map(|pat| (pat, false))
                .chain(rest.map(|pat| (pat, true)))
                .chain(suffix.iter().map(|pat| (pat, false)))
                .map(|(pat, rest)| self.record_slice_pattern(pat, rest))
                .collect(),
            PatKind::Binding(_, _, _, subpattern) => subpattern
                .into_iter()
                .map(|pat| self.record_slice_pattern(pat, rest))
                .collect(),
            PatKind::TupleStruct(_, children, _)
            | PatKind::Tuple(children, _)
            | PatKind::Or(children) => children
                .iter()
                .map(|pat| self.record_slice_pattern(pat, false))
                .collect(),
            PatKind::Range(start, end, _) => start
                .into_iter()
                .chain(end)
                .map(|bound| self.slice_pattern_bound(bound))
                .collect(),
            PatKind::Expr(bound) => vec![self.slice_pattern_bound(bound)],
            _ => self.other_slice_children(pattern),
        }
    }

    fn other_slice_children(&self, pattern: &hir::Pat<'tcx>) -> Vec<HirNode> {
        match pattern.kind {
            PatKind::Struct(_, fields, _) => fields
                .iter()
                .map(|field| HirNode {
                    children: vec![self.record_slice_pattern(field.pat, false)],
                    ..self.node(field.hir_id, field.span, "pat_slice_field_unsupported")
                })
                .collect(),
            PatKind::Deref(pat) | PatKind::Ref(pat, _, _) => {
                vec![self.record_slice_pattern(pat, false)]
            }
            PatKind::Guard(pat, guard) => vec![
                self.record_slice_pattern(pat, false),
                self.expression(guard),
            ],
            _ => Vec::new(),
        }
    }

    fn slice_pattern_bound(&self, bound: &hir::PatExpr<'tcx>) -> HirNode {
        let mut node = self.node(bound.hir_id, bound.span, "pat_slice_bound");
        node.ty = self
            .typeck
            .node_type_opt(bound.hir_id)
            .map(|ty| resolved(ty, String::new()));
        node.expansion = bound.span.from_expansion().then(|| ExpansionSite {
            callsite: self.span(bound.span.source_callsite()),
            snippet: self.snippet(bound.span.source_callsite()),
        });

        let (details, status) = self.slice_bound_details(bound);
        let status = if node.ty.is_none() {
            CaptureStatus::Unknown("Missing compiler range endpoint type".into())
        } else if node.expansion.is_some() {
            CaptureStatus::Unsupported("Expanded range endpoint".into())
        } else {
            status
        };
        let observation = Observation {
            identity: IDENTITY.into(),
            hir: reference(bound.hir_id),
            span: node.span,
            ty: super::observe::shape(self.typeck.node_type_opt(bound.hir_id)),
            adjustments: Vec::new(),
            details,
            status,
        };

        node.res = observation.resolution().ok();
        node
    }
    fn slice_bound_details(&self, bound: &hir::PatExpr<'tcx>) -> (Details, CaptureStatus) {
        match bound.kind {
            hir::PatExprKind::Lit { lit, negated } => match lit.node {
                LitKind::Byte(value) => (
                    Details::ByteLiteral { value, negated },
                    CaptureStatus::Captured,
                ),
                _ => unsupported_bound("Non-byte literal"),
            },
            hir::PatExprKind::Path(ref path) => {
                let resolution = self.typeck.qpath_res(path, bound.hir_id);
                (
                    Details::Path {
                        resolution: PathRes {
                            kind: "unsupported_pattern_path".into(),
                            target: match resolution {
                                hir::def::Res::Local(id) => Some(reference(id)),
                                _ => None,
                            },
                            description: format!("{resolution:?}"),
                        },
                    },
                    CaptureStatus::Unsupported("Path range endpoint".into()),
                )
            }
        }
    }
}

fn unsupported_bound(reason: &str) -> (Details, CaptureStatus) {
    (
        Details::Unsupported {
            compiler_kind: reason.into(),
        },
        CaptureStatus::Unsupported(reason.into()),
    )
}
