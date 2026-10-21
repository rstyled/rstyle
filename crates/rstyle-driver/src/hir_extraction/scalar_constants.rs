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

        self.pattern_constant_resolution(expression)
    }

    fn pattern_constant_resolution(&self, expression: &rustc_hir::PatExpr<'_>) -> Option<PathRes> {
        let rustc_hir::PatExprKind::Path(ref path) = expression.kind else {
            return None;
        };

        self.constant_resolution(self.typeck.qpath_res(path, expression.hir_id))
    }

    pub(super) fn range_constants(
        &self,
        pattern: &rustc_hir::Pat<'_>,
    ) -> Option<Vec<rstyle_pdg::HirNode>> {
        let rustc_hir::PatKind::Range(start, end, _) = pattern.kind else {
            return None;
        };
        start
            .into_iter()
            .chain(end)
            .filter(|bound| matches!(bound.kind, rustc_hir::PatExprKind::Path(_)))
            .map(|bound| {
                if !self.profile.supports("rust.scalar-constant-range-bounds") {
                    return None;
                }
                let resolution = self.pattern_constant_resolution(bound)?;
                let constant = ScalarConstant::decode(&resolution).ok()?;
                let mut node = self.node(bound.hir_id, bound.span, "pat_constant");
                node.ty = self
                    .typeck
                    .node_type_opt(pattern.hir_id)
                    .map(|ty| resolved(ty, String::new()));
                node.lit = Some(constant.value);
                node.res = Some(resolution);
                Some(node)
            })
            .collect()
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

    pub(super) fn local_constant(&self, kind: DefKind, definition: DefId) -> bool {
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

    pub(super) fn constant_declaration(&self, definition: DefId) -> Option<String> {
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

impl Context<'_> {
    pub(super) fn native_size_constant(&self, item: &rustc_hir::Item<'_>) -> Option<PathRes> {
        use rustc_hir::{ConstItemRhs, ExprKind, ItemKind};
        if !self.profile.supports("rust.native-borrowed-byte-readers") || item.span.from_expansion()
        {
            return None;
        }
        let ItemKind::Const(_, _, _, ConstItemRhs::Body(body)) = item.kind else {
            return None;
        };

        let value = self.tcx.hir_body(body).value;
        let ExprKind::Call(callee, []) = value.kind else {
            return None;
        };
        let ExprKind::Path(ref path) = callee.kind else {
            return None;
        };
        let typeck = self.tcx.typeck(item.owner_id.def_id);
        let Res::Def(_, definition) = typeck.qpath_res(path, callee.hir_id) else {
            return None;
        };
        let args = typeck.node_args(callee.hir_id);
        if args.len() != 1 || args.type_at(0) != self.tcx.types.usize {
            return None;
        }
        let resolution =
            self.constant_resolution(Res::Def(DefKind::Const, item.owner_id.to_def_id()))?;
        let constant = ScalarConstant::decode(&resolution).ok()?;
        rstyle_pdg::size_constants::declaration(&constant.declaration).ok()?;
        let initializer = self.library_callee(definition);
        let record = rstyle_pdg::size_constants::NativeSize {
            constant,
            initializer: serde_json::from_str(&initializer.description).ok()?,
            argument: "usize".into(),
        };
        Some(PathRes {
            kind: rstyle_pdg::size_constants::RESOLUTION.into(),
            target: None,
            description: serde_json::to_string(&record).ok()?,
        })
    }
}
