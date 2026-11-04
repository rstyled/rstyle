//! Per-node observations of Rust-only contracts, recorded on a function's document by
//! the node they resolve: scalar product constructors, id-reader constants and their
//! annotations, and scalar instance calls.

use super::hir::Context;
use rustc_hir::intravisit::{self, Visitor};
use rustc_hir::{Expr, ExprKind, LetStmt};
use rustc_middle::hir::nested_filter::OnlyBodies;
use rustc_middle::ty::TyCtxt;
use serde_json::{Map, Value, json};

pub(super) fn observe<'tcx>(
    context: &Context<'tcx>,
    body: &'tcx rustc_hir::Body<'tcx>,
) -> Map<String, Value> {
    let mut scan = Scan {
        context,
        observations: Map::new(),
    };
    scan.visit_body(body);
    scan.observations
}

struct Scan<'a, 'tcx> {
    context: &'a Context<'tcx>,
    observations: Map<String, Value>,
}

impl Scan<'_, '_> {
    fn record(&mut self, id: rustc_hir::HirId, key: &str, value: Value) {
        let node = format!(
            "f{}.{}",
            id.owner.def_id.local_def_index.as_u32(),
            id.local_id.as_u32()
        );
        let entry = self.observations.entry(node).or_insert_with(|| json!({}));
        entry[key] = value;
    }
}

impl<'tcx> Visitor<'tcx> for Scan<'_, 'tcx> {
    type NestedFilter = OnlyBodies;

    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.context.tcx
    }

    // Constant bodies have their own typeck owner, which this function's never covers.
    fn visit_anon_const(&mut self, _: &'tcx rustc_hir::AnonConst) {}

    fn visit_inline_const(&mut self, _: &'tcx rustc_hir::ConstBlock) {}

    // rstyle-allow(mutation_per_target): the scan records each node's observations as it walks
    fn visit_expr(&mut self, expression: &'tcx Expr<'tcx>) {
        let context = self.context;
        if let Some((resolution, dispatch)) = context.scalar_instance_call(expression) {
            self.record(
                expression.hir_id,
                "scalar_instance",
                json!({"res": resolution, "dispatch": dispatch}),
            );
        }
        if let Some(resolution) = context.scalar_product_constructor(expression) {
            self.record(
                expression.hir_id,
                "scalar_product_constructor",
                json!(resolution),
            );
        }
        match super::concrete_types::product_constant(context, expression) {
            Ok(Some(constant)) => self.record(expression.hir_id, "product_constant", constant),
            Ok(None) => {}
            Err(reason) => self.record(
                expression.hir_id,
                "product_constant",
                json!({"error": reason}),
            ),
        }
        if let ExprKind::Path(ref path) = expression.kind {
            let resolution = context.typeck.qpath_res(path, expression.hir_id);
            if let rustc_hir::def::Res::Def(rustc_hir::def::DefKind::AssocFn, callee) = resolution
                && !callee.is_local()
                && let Some(call) = super::trait_members::default_call(context, expression, callee)
            {
                self.record(expression.hir_id, "default_call", json!(call));
            }
            if let Some(constant) = context.id_reader_constant_resolution(resolution) {
                self.record(expression.hir_id, "id_reader_constant", json!(constant));
            }
        }

        intravisit::walk_expr(self, expression);
    }

    fn visit_local(&mut self, local: &'tcx LetStmt<'tcx>) {
        if let Some(annotation) = local.ty
            && let Some(resolution) = self.context.id_reader_annotation(annotation)
        {
            self.record(annotation.hir_id, "id_reader_annotation", json!(resolution));
        }
        intravisit::walk_local(self, local);
    }
}
