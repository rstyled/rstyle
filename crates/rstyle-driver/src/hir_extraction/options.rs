//! Observational core Option identities and patterns; no construction admission.
use super::hir::Context;
use rstyle_pdg::frontend::{HirNode, PathRes};
use rustc_hir::{
    self as hir, PatKind,
    def::{DefKind, Res},
    def_id::DefId,
};
use rustc_middle::ty::{self, Ty, TyCtxt};

pub(super) fn payload<'tcx>(tcx: TyCtxt<'tcx>, value: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let ty::Adt(definition, arguments) = value.kind() else {
        return None;
    };
    if !tcx.is_diagnostic_item(rustc_span::sym::Option, definition.did()) {
        return None;
    }
    let payload = arguments.type_at(0);
    (scalar(payload) || shared_scalar(payload)).then_some(payload)
}

fn scalar(value: Ty<'_>) -> bool {
    matches!(value.kind(), ty::Bool | ty::Int(_) | ty::Uint(_))
}

fn shared_scalar(value: Ty<'_>) -> bool {
    matches!(value.kind(), ty::Ref(_, inner, hir::Mutability::Not) if scalar(*inner))
}

fn candidate(pattern: &hir::Pat<'_>) -> bool {
    match pattern.kind {
        PatKind::Ref(..) | PatKind::TupleStruct(..) => true,
        PatKind::Expr(expression) => matches!(expression.kind, hir::PatExprKind::Path(_)),
        _ => false,
    }
}

impl<'tcx> Context<'tcx> {
    pub(super) fn option_expression(&self, expression: &hir::Expr<'tcx>) -> Option<PathRes> {
        if self.profile != rstyle_pdg::extraction_profile::ExtractionProfile::DataV3 {
            return None;
        }
        let (path, id, expected) = match expression.kind {
            hir::ExprKind::Call(callee, arguments) if arguments.len() == 1 => {
                let hir::ExprKind::Path(ref path) = callee.kind else {
                    return None;
                };
                (path, callee.hir_id, "Some")
            }
            hir::ExprKind::Path(ref path) => (path, expression.hir_id, "None"),
            _ => return None,
        };
        let resolution = self.option_variant(path, id, expected)?;
        payload(self.tcx, self.typeck.expr_ty(expression))?;

        Some(resolution)
    }

    pub(super) fn option_pattern(&self, pattern: &hir::Pat<'tcx>) -> Option<HirNode> {
        if self.profile != rstyle_pdg::extraction_profile::ExtractionProfile::DataV3
            || !candidate(pattern)
        {
            return None;
        }

        let value = self.typeck.node_type_opt(pattern.hir_id)?;
        let (kind, resolution, children) = match pattern.kind {
            PatKind::Ref(inner, hir::Pinnedness::Not, hir::Mutability::Not)
                if shared_scalar(value) =>
            {
                ("pat_shared", None, vec![self.pattern(inner)])
            }
            PatKind::TupleStruct(ref path, children, rest)
                if children.len() == 1 && rest.as_opt_usize().is_none() =>
            {
                payload(self.tcx, value)?;
                let resolution = self.option_variant(path, pattern.hir_id, "Some")?;
                (
                    "pat_option_some",
                    Some(resolution),
                    children.iter().map(|child| self.pattern(child)).collect(),
                )
            }
            PatKind::Expr(expression) => {
                let hir::PatExprKind::Path(ref path) = expression.kind else {
                    return None;
                };
                payload(self.tcx, value)?;
                let resolution = self.option_variant(path, pattern.hir_id, "None")?;
                ("pat_option_none", Some(resolution), Vec::new())
            }
            _ => return None,
        };

        let mut node = self.node(pattern.hir_id, pattern.span, kind);
        node.ty = Some(super::hir::resolved(value, String::new()));
        node.res = resolution;
        node.children = children;
        Some(node)
    }

    fn option_variant(
        &self,
        path: &hir::QPath<'tcx>,
        id: hir::HirId,
        expected: &str,
    ) -> Option<PathRes> {
        let Res::Def(kind, definition) = self.typeck.qpath_res(path, id) else {
            return None;
        };
        let variant = if matches!(kind, DefKind::Ctor(..)) {
            self.tcx.parent(definition)
        } else {
            definition
        };
        if !self
            .tcx
            .is_diagnostic_item(rustc_span::sym::Option, self.tcx.parent(variant))
            || self.tcx.item_name(variant).as_str() != expected
        {
            return None;
        }
        Some(self.variant_resolution(definition))
    }

    fn variant_resolution(&self, definition: DefId) -> PathRes {
        let callee = rstyle_pdg::data_library::Callee {
            path: self.tcx.def_path_str(definition),
            crate_name: self.tcx.crate_name(definition.krate).to_string(),
            crate_hash: self.tcx.crate_hash(definition.krate).to_string(),
            definition_hash: format!("{:?}", self.tcx.def_path_hash(definition)),
            implementation: None,
        };
        PathRes {
            kind: "resolved_option_variant".into(),
            target: None,
            description: serde_json::to_string(&callee).expect("resolved Option variant"),
        }
    }
}
