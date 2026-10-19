//! Evaluate only referenced local scalar constants; rustc owns const execution.

use super::hir::{Context, resolved};
use rstyle_pdg::{PathRes, scalar_constants::ScalarConstant};
use rustc_hir::{
    Node,
    def::{DefKind, Res},
    def_id::DefId,
};
use rustc_middle::{
    mir::ConstValue,
    ty::{self, Ty},
};

impl Context<'_> {
    pub(super) fn constant_pattern(&self, pattern: &rustc_hir::Pat<'_>) -> Option<PathRes> {
        if !self.profile.supports("rust.scalar-constant-patterns") {
            return None;
        }
        let rustc_hir::PatKind::Expr(expression) = pattern.kind else {
            return None;
        };
        let rustc_hir::PatExprKind::Path(ref path) = expression.kind else {
            return None;
        };

        self.constant_resolution(self.typeck.qpath_res(path, expression.hir_id))
    }

    pub(super) fn constant_resolution(&self, resolution: Res) -> Option<PathRes> {
        let Res::Def(kind, definition) = resolution else {
            return None;
        };
        if !self.profile.supports("rust.resolved-scalar-constants")
            || !self.local_constant(kind, definition)
        {
            return None;
        }
        let environment = ty::TypingEnv::post_analysis(self.tcx, definition);
        let ty = self
            .tcx
            .try_normalize_erasing_regions(
                environment,
                self.tcx.type_of(definition).instantiate_identity(),
            )
            .ok()?;
        let token = resolved(ty, String::new()).normalized?;
        let constant = ScalarConstant {
            definition: self.tcx.def_path_str(definition),
            declaration: self.constant_declaration(definition)?,
            ty: token,
            value: constant_value(self.tcx.const_eval_poly(definition).ok()?, ty)?,
        };

        Some(PathRes {
            kind: rstyle_pdg::scalar_constants::RESOLUTION.into(),
            target: None,
            description: serde_json::to_string(&constant).ok()?,
        })
    }

    fn local_constant(&self, kind: DefKind, definition: DefId) -> bool {
        matches!(kind, DefKind::Const | DefKind::AssocConst)
            && definition.is_local()
            && self.tcx.generics_of(definition).count() == 0
            && match kind {
                DefKind::Const => true,
                DefKind::AssocConst => matches!(
                    self.tcx.def_kind(self.tcx.parent(definition)),
                    DefKind::Impl { of_trait: false },
                ),
                _ => false,
            }
    }

    fn constant_declaration(&self, definition: DefId) -> Option<String> {
        let span = match self.tcx.hir_node_by_def_id(definition.expect_local()) {
            Node::Item(item) => item.span,
            Node::ImplItem(item) => item.span,
            _ => return None,
        };
        if span.from_expansion() {
            return None;
        }

        self.sources
            .range(self.tcx, span)
            .map(|(source, _)| source.to_string())
    }
}

fn constant_value(value: ConstValue, ty: Ty<'_>) -> Option<String> {
    match ty.kind() {
        ty::Bool => value.try_to_bool().map(|value| value.to_string()),
        ty::Tuple(elements) if elements.is_empty() => Some("()".into()),
        ty::Int(_) => value
            .try_to_scalar_int()
            .map(|value| value.to_int(value.size()).to_string()),
        ty::Uint(_) => value
            .try_to_scalar_int()
            .map(|value| value.to_bits(value.size()).to_string()),
        _ => None,
    }
}
