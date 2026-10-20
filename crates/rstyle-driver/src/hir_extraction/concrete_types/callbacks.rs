//! Capture actual environment storage using the existing concrete type observer.

use super::Capture;
use crate::hir_extraction::hir::reference;
use rstyle_pdg::{HirNode, PathRes, option_callbacks as records};
use rustc_hir::{ExprKind, Node};
use rustc_middle::ty;

impl<'tcx> Capture<'_, 'tcx> {
    pub(super) fn callback_environments(&mut self, node: &mut HirNode) -> Result<(), String> {
        // Unsupported coroutine/async bodies remain outside this capture
        // boundary rather than aborting unrelated functions in the Cargo unit.
        if node.kind == "expr_closure" && !node.children.is_empty() {
            node.res = Some(self.callback_environment(node)?);
        }
        node.children
            .iter_mut()
            .try_for_each(|child| self.callback_environments(child))
    }

    fn callback_environment(&mut self, node: &HirNode) -> Result<PathRes, String> {
        let owner = rustc_hir::def_id::LocalDefId {
            local_def_index: rustc_hir::def_id::DefIndex::from_u32(node.hir.owner),
        };
        let id = rustc_hir::HirId {
            owner: rustc_hir::OwnerId { def_id: owner },
            local_id: rustc_hir::ItemLocalId::from_u32(node.hir.local),
        };
        let Node::Expr(expression) = self.ctx.tcx.hir_node(id) else {
            return Err("Callback identity is not a compiler expression".into());
        };
        let ExprKind::Closure(closure) = expression.kind else {
            return Err("Callback compiler kind mismatch".into());
        };
        let ty::Closure(definition, arguments) = self.ctx.typeck.expr_ty(expression).kind() else {
            return Err("Callback has no compiler closure type".into());
        };
        if *definition != closure.def_id.to_def_id() {
            return Err("Callback compiler owner mismatch".into());
        }

        let captures = self.ctx.tcx.closure_captures(closure.def_id);
        if captures.len() > rstyle_pdg::concrete_types::MAX_FIELDS {
            return Err("Callback environment exceeds concrete field budget".into());
        }
        let environment = records::Environment {
            captures: captures.iter().copied().map(capture_place).collect(),
            stored_type: self.callback_storage(arguments.as_closure().tupled_upvars_ty())?,
        };
        Ok(PathRes {
            kind: records::ENVIRONMENT_KIND.into(),
            target: None,
            description: serde_json::to_string(&environment).map_err(|error| error.to_string())?,
        })
    }

    fn callback_storage(
        &mut self,
        stored: ty::Ty<'tcx>,
    ) -> Result<rstyle_pdg::concrete_types::Observation, String> {
        let supported = matches!(stored.kind(), ty::Tuple(fields) if fields.len() == 1
            && matches!(fields[0].kind(), ty::Ref(_, referent, rustc_hir::Mutability::Not)
                if matches!(referent.kind(), ty::Uint(ty::UintTy::U8))));
        if supported {
            return self.observe(stored);
        }
        // Do not recursively duplicate arbitrary captured nominal declarations
        // merely to reject a closure outside the bounded storage contract.
        Ok(rstyle_pdg::concrete_types::Observation::Unsupported {
            reason: "Callback environment is not exactly one shared primitive byte".into(),
        })
    }
}

fn capture_place(place: &ty::CapturedPlace<'_>) -> records::Capture {
    let mode = match place.info.capture_kind {
        ty::UpvarCapture::ByRef(ty::BorrowKind::Immutable) => records::CaptureMode::Shared,
        ty::UpvarCapture::ByRef(ty::BorrowKind::UniqueImmutable) => records::CaptureMode::Unique,
        ty::UpvarCapture::ByRef(ty::BorrowKind::Mutable) => records::CaptureMode::Mutable,
        ty::UpvarCapture::ByValue => records::CaptureMode::Value,
        ty::UpvarCapture::ByUse => records::CaptureMode::Use,
    };
    records::Capture {
        root: reference(place.get_root_variable()),
        mode,
        projections: place.place.projections.len(),
    }
}
