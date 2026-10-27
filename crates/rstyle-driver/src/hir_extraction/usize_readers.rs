//! Observations for immediate Result reader callbacks; no callable or effect authority.
use super::hir::{Context, hir_id};
use rstyle_pdg::{HirNode, PathRes, concrete_types::Observation};
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{self as hir, Expr};
use rustc_middle::ty;
use rustc_middle::ty::Ty;

pub(super) const CAPABILITY: &str = "rust.usize-reader-callback-observations";
const KIND: &str = "compiler_usize_reader_callback_environment";
const IDENTITY: &str = "rstyle.rust.usize-reader-callback-environment/1";

impl<'tcx> Context<'tcx> {
    pub(super) fn usize_reader_callback(&self, expression: &Expr<'tcx>) -> bool {
        self.profile.supports(CAPABILITY)
            && self.usize_reader_result()
            && self.immediate_core_callback(
                expression,
                &[
                    "core::result::Result::<T, E>::and_then",
                    "std::result::Result::<T, E>::and_then",
                    "core::result::Result::<T, E>::map",
                    "std::result::Result::<T, E>::map",
                    "core::result::Result::<T, E>::map_err",
                    "std::result::Result::<T, E>::map_err",
                ],
            )
            && self.usize_reader_return_free(expression)
    }

    fn usize_reader_return_free(&self, expression: &Expr<'tcx>) -> bool {
        let hir::ExprKind::Closure(closure) = expression.kind else {
            return false;
        };
        let mut scan = ReturnScan {
            tcx: self.tcx,
            found: false,
            remaining: rstyle_pdg::concrete_types::MAX_ENTRIES,
            depth: 0,
        };
        scan.visit_expr(self.tcx.hir_body(closure.body).value);

        !scan.found
    }

    pub(super) fn usize_reader_result(&self) -> bool {
        let ty::Adt(definition, arguments) = self.result.kind() else {
            return false;
        };
        if !self
            .tcx
            .is_diagnostic_item(rustc_span::sym::Result, definition.did())
        {
            return false;
        }

        matches!(arguments.type_at(0).kind(), ty::Tuple(fields)
            if fields.len() == 2 && fields[0] == self.tcx.types.usize
                && fields[1] == self.tcx.types.usize)
    }

    pub(super) fn usize_reader_children(&self, expression: &Expr<'tcx>) -> Option<Vec<HirNode>> {
        self.closure_children(expression, |pattern| self.local_pattern(pattern))
    }

    pub(super) fn capture_usize_reader_callbacks(
        &self,
        node: &mut HirNode,
        observe: &mut impl FnMut(Ty<'tcx>) -> Result<Observation, String>,
    ) -> Result<(), String> {
        if !self.profile.supports(CAPABILITY) {
            return Ok(());
        }
        if node.kind == "expr_closure"
            && !node.children.is_empty()
            && let hir::Node::Expr(expression) = self.tcx.hir_node(hir_id(&node.hir))
            && self.usize_reader_callback(expression)
        {
            let environment = self.callback_environment_record(expression, observe, IDENTITY)?;
            node.res = Some(PathRes {
                kind: KIND.into(),
                target: None,
                description: serde_json::to_string(&environment)
                    .map_err(|error| error.to_string())?,
            });
        }

        node.children
            .iter_mut()
            .try_for_each(|child| self.capture_usize_reader_callbacks(child, observe))
    }
}

struct ReturnScan<'tcx> {
    tcx: rustc_middle::ty::TyCtxt<'tcx>,
    found: bool,
    remaining: usize,
    depth: usize,
}

impl<'tcx> Visitor<'tcx> for ReturnScan<'tcx> {
    fn visit_expr(&mut self, expression: &'tcx Expr<'tcx>) {
        if !self.enter(expression) {
            return;
        }

        self.walk(expression);
        self.depth -= 1;
    }
}

impl<'tcx> ReturnScan<'tcx> {
    fn enter(&mut self, expression: &Expr<'_>) -> bool {
        if self.found
            || self.remaining == 0
            || self.depth == rstyle_pdg::concrete_types::MAX_DEPTH
            || matches!(expression.kind, hir::ExprKind::Ret(_))
        {
            self.found = true;
            return false;
        }
        self.remaining -= 1;
        self.depth += 1;

        true
    }

    fn walk(&mut self, expression: &'tcx Expr<'tcx>) {
        match expression.kind {
            hir::ExprKind::Closure(closure) => {
                self.visit_expr(self.tcx.hir_body(closure.body).value);
            }
            _ => intravisit::walk_expr(self, expression),
        }
    }
}
