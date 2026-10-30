//! Capture actual environment storage using the existing concrete type observer.

use super::Capture;
use crate::hir_extraction::hir::reference;
use rstyle_pdg::{HirNode, PathRes, option_callbacks as records};
use rustc_hir::{self as hir, ExprKind, Node};
use rustc_middle::hir::place::{PlaceBase, ProjectionKind};
use rustc_middle::ty;

impl<'tcx> Capture<'_, 'tcx> {
    pub(super) fn callback_environments(&mut self, node: &mut HirNode) -> Result<(), String> {
        self.callback_environments_in(node, None)
    }

    // rstyle-allow(side_effect_density): Recursive capture visitor installs callback observations in the original mutable node tree.
    fn callback_environments_in(
        &mut self,
        node: &mut HirNode,
        owner: Option<&rstyle_pdg::HirRef>,
    ) -> Result<(), String> {
        // Unsupported bodies retain actual captures and explicit unknown storage.
        if node.kind == "expr_closure" && !node.children.is_empty() {
            node.res = Some(self.callback_environment(node, owner == Some(&node.hir))?);
        }
        let callback = immediate_map_or_callback(node);
        node.children
            .iter_mut()
            .try_for_each(|child| self.callback_environments_in(child, callback.as_ref()))
    }

    fn callback_environment(
        &mut self,
        node: &HirNode,
        shared_owner: bool,
    ) -> Result<PathRes, String> {
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
        let stored = arguments.as_closure().tupled_upvars_ty();
        let shared_place = captures
            .first()
            .filter(|_| shared_owner && captures.len() == 1)
            .copied()
            .filter(|place| shared_formal_place(self.ctx.tcx, place, closure.def_id, stored));
        let shared_formal = self.shared_formal_capture(node, shared_place)?;
        let environment = records::Environment {
            captures: captures.iter().copied().map(capture_place).collect(),
            stored_type: self.callback_storage(stored, shared_formal.is_some())?,
            shared_formal,
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
        shared_formal: bool,
    ) -> Result<rstyle_pdg::concrete_types::Observation, String> {
        let supported = matches!(stored.kind(), ty::Tuple(fields) if fields.len() == 1
            && matches!(fields[0].kind(), ty::Ref(_, referent, rustc_hir::Mutability::Not)
                if matches!(referent.kind(), ty::Uint(ty::UintTy::U8))));
        if supported || shared_formal {
            return self.observe(stored);
        }
        // Do not recursively duplicate arbitrary captured nominal declarations
        // merely to reject a closure outside the bounded storage contract.
        Ok(rstyle_pdg::concrete_types::Observation::Unsupported {
            reason: "Callback environment is not exactly one shared primitive byte".into(),
        })
    }

    fn shared_formal_capture(
        &mut self,
        node: &HirNode,
        place: Option<&ty::CapturedPlace<'tcx>>,
    ) -> Result<Option<records::SharedCapture>, String> {
        if !self
            .ctx
            .profile
            .supports("rust.shared-helper-option-callbacks")
        {
            return Ok(None);
        }
        let Some(place) = place else {
            return Ok(None);
        };
        if !shared_helper_body(node, &reference(place.get_root_variable())) {
            return Ok(None);
        }

        let projection = &place.place.projections[0];
        let root_type = self.observe(place.place.base_ty)?;
        let projected = records::SharedProjection::SharedDeref {
            before: root_type.clone(),
            after: self.observe(projection.ty)?,
        };
        let uses = self.shared_capture_uses(node, place.get_root_variable())?;
        Ok(Some(records::SharedCapture {
            closure: node.hir.clone(),
            root_type,
            projections: vec![projected],
            uses,
        }))
    }

    fn shared_capture_uses(
        &mut self,
        node: &HirNode,
        root: hir::HirId,
    ) -> Result<Vec<records::SharedUse>, String> {
        let mut paths = Vec::new();
        rstyle_pdg::frontend::collect_kind(node, "expr_path", &mut paths);
        let root = reference(root);
        paths
            .into_iter()
            .filter(|path| {
                path.res.as_ref().is_some_and(|resolution| {
                    resolution.kind == "local" && resolution.target.as_ref() == Some(&root)
                })
            })
            .map(|path| {
                let id = hir::HirId {
                    owner: hir::OwnerId {
                        def_id: hir::def_id::LocalDefId {
                            local_def_index: hir::def_id::DefIndex::from_u32(path.hir.owner),
                        },
                    },
                    local_id: hir::ItemLocalId::from_u32(path.hir.local),
                };
                let ty = self
                    .ctx
                    .typeck
                    .node_type_opt(id)
                    .ok_or("Missing shared capture use type")?;
                Ok(records::SharedUse {
                    hir: path.hir.clone(),
                    ty: self.observe(ty)?,
                })
            })
            .collect()
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

fn shared_formal_place<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    captured: &ty::CapturedPlace<'tcx>,
    closure: hir::def_id::LocalDefId,
    stored: ty::Ty<'tcx>,
) -> bool {
    let PlaceBase::Upvar(upvar) = captured.place.base else {
        return false;
    };
    if upvar.closure_expr_id != closure
        || captured.info.capture_kind != ty::UpvarCapture::ByRef(ty::BorrowKind::Immutable)
        || captured.mutability != hir::Mutability::Not
        || !immutable_formal(tcx, upvar.var_path.hir_id)
    {
        return false;
    }
    let ty::Ref(_, referent, hir::Mutability::Not) = captured.place.base_ty.kind() else {
        return false;
    };
    let ty::Adt(definition, _) = referent.kind() else {
        return false;
    };
    let [projection] = captured.place.projections.as_slice() else {
        return false;
    };
    let ty::Tuple(fields) = stored.kind() else {
        return false;
    };

    definition.is_struct()
        && projection.kind == ProjectionKind::Deref
        && projection.ty == *referent
        && fields.len() == 1
        && fields[0] == captured.place.base_ty
}

fn immutable_formal(tcx: ty::TyCtxt<'_>, root: hir::HirId) -> bool {
    let Node::Pat(pattern) = tcx.hir_node(root) else {
        return false;
    };
    if !matches!(
        pattern.kind,
        hir::PatKind::Binding(
            hir::BindingMode(hir::ByRef::No, hir::Mutability::Not),
            _,
            _,
            None
        )
    ) {
        return false;
    }
    tcx.hir_body_owned_by(root.owner.def_id)
        .params
        .iter()
        .any(|parameter| parameter.pat.hir_id == root)
}

fn immediate_map_or_callback(node: &HirNode) -> Option<rstyle_pdg::HirRef> {
    if node.kind != "expr_method_call" || node.children.len() != 3 {
        return None;
    }

    let resolution = node.res.as_ref()?;
    let callee: rstyle_pdg::data_library::Callee =
        serde_json::from_str(&resolution.description).ok()?;
    if callee.crate_name != "core"
        || callee.implementation.is_some()
        || !matches!(
            callee.path.as_str(),
            "core::option::Option::<T>::map_or" | "std::option::Option::<T>::map_or"
        )
    {
        return None;
    }
    node.children.last().map(|callback| callback.hir.clone())
}

fn shared_helper_body(node: &HirNode, root: &rstyle_pdg::HirRef) -> bool {
    let [pattern, body] = node.children.as_slice() else {
        return false;
    };
    let binding = if pattern.kind == "pat_shared_copy" {
        let [binding] = pattern.children.as_slice() else {
            return false;
        };
        binding
    } else {
        pattern
    };
    if node.expansion.is_some()
        || body.expansion.is_some()
        || body.kind != "expr_method_call"
        || body.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("bool")
        || !body
            .res
            .as_ref()
            .is_some_and(|res| res.kind == "resolved_inherent_callee")
        || binding.kind != "pat_binding"
        || binding.binding_mut != Some(false)
        || binding.param_index.is_some()
        || binding.binding.is_none()
        || !binding.children.is_empty()
        || binding.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("u8")
    {
        return false;
    }
    let [receiver, payload] = body.children.as_slice() else {
        return false;
    };

    capture_path(receiver, root) && capture_path(payload, &binding.hir)
}

fn capture_path(node: &HirNode, target: &rstyle_pdg::HirRef) -> bool {
    node.kind == "expr_path"
        && node.expansion.is_none()
        && node.children.is_empty()
        && node.res.as_ref().is_some_and(|resolution| {
            resolution.kind == "local" && resolution.target.as_ref() == Some(target)
        })
}
