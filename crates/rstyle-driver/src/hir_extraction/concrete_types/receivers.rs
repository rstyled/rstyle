//! Exact resolved receiver transitions, captured only for the Data profile.

use super::Capture;
use crate::hir_extraction::hir::reference;
use rstyle_pdg::{HirFnFacts, HirNode, data_receivers as records};
use rustc_hir::{Expr, ExprKind, Node};
use rustc_middle::ty::adjustment::{Adjust, AutoBorrow, AutoBorrowMutability, DerefAdjustKind};
use rustc_middle::ty::{self, Ty};

struct ReceiverTypes {
    source: rstyle_pdg::concrete_types::Observation,
    adjusted: rstyle_pdg::concrete_types::Observation,
    adjustments: Vec<records::Adjustment>,
}

impl<'tcx> Capture<'_, 'tcx> {
    pub(super) fn install_receivers(&mut self, facts: &mut HirFnFacts) -> Result<(), String> {
        let mut methods = Vec::new();
        self.receiver_records(&facts.body, &mut methods)?;
        let capture = records::Capture {
            identity: records::IDENTITY.into(),
            methods,
        };
        facts.build.frontend_options.insert_data(capture);

        Ok(())
    }

    fn receiver_records(
        &mut self,
        node: &HirNode,
        methods: &mut Vec<records::Method>,
    ) -> Result<(), String> {
        if node.kind == "expr_method_call" {
            if methods.len() >= records::MAX_RECORDS {
                return Err("Method receiver capture exceeds record budget".into());
            }
            let id = rustc_hir::HirId {
                owner: rustc_hir::OwnerId {
                    def_id: rustc_hir::def_id::LocalDefId {
                        local_def_index: rustc_hir::def_id::DefIndex::from_u32(node.hir.owner),
                    },
                },
                local_id: rustc_hir::ItemLocalId::from_u32(node.hir.local),
            };
            let Node::Expr(expression) = self.ctx.tcx.hir_node(id) else {
                return Err("Method receiver HIR identity is not an expression".into());
            };
            methods.push(self.receiver_record(expression)?);
        }
        for child in &node.children {
            self.receiver_records(child, methods)?;
        }

        Ok(())
    }

    fn receiver_record(&mut self, expression: &Expr<'tcx>) -> Result<records::Method, String> {
        let ExprKind::MethodCall(_, receiver, _, _) = expression.kind else {
            return Err("Method receiver HIR kind mismatch".into());
        };
        let callee = self
            .ctx
            .typeck
            .type_dependent_def_id(expression.hir_id)
            .ok_or("Missing resolved method definition")?;
        let signature = self
            .ctx
            .tcx
            .fn_sig(callee)
            .instantiate(self.ctx.tcx, self.ctx.typeck.node_args(expression.hir_id))
            .skip_binder();
        let character_ascii = matches!(self.ctx.typeck.expr_ty(receiver).kind(), ty::Char)
            && self.ctx.profile.supports("rust.character-ascii-predicate")
            && self
                .ctx
                .tcx
                .is_diagnostic_item(rustc_span::Symbol::intern("char_is_ascii"), callee)
            && matches!(signature.output().kind(), ty::Bool)
            && signature.inputs().len() == 1
            && shared_character(signature.inputs()[0]);
        let types = self.receiver_types(receiver, character_ascii)?;

        Ok(records::Method {
            call: reference(expression.hir_id),
            source_span: self.ctx.span(expression.span),
            receiver: reference(receiver.hir_id),
            receiver_type: types.source,
            adjusted_type: types.adjusted,
            adjustments: types.adjustments,
            signature_inputs: signature
                .inputs()
                .iter()
                .map(|ty| self.signature_type(*ty, character_ascii))
                .collect::<Result<Vec<_>, _>>()?,
            signature_result: self.signature_type(signature.output(), character_ascii)?,
        })
    }

    fn signature_type(
        &mut self,
        ty: rustc_middle::ty::Ty<'tcx>,
        character_ascii: bool,
    ) -> Result<rstyle_pdg::concrete_types::Observation, String> {
        let environment = rustc_middle::ty::TypingEnv::post_analysis(
            self.ctx.tcx,
            self.ctx.typeck.hir_owner.def_id,
        );
        match self
            .ctx
            .tcx
            .try_normalize_erasing_regions(environment, rustc_middle::ty::Unnormalized::new_wip(ty))
        {
            Ok(ty) => self.method_observation(ty, character_ascii),
            Err(_) => Ok(rstyle_pdg::concrete_types::Observation::Unsupported {
                reason: "Method signature type cannot be normalized".into(),
            }),
        }
    }

    fn method_observation(
        &mut self,
        ty: Ty<'tcx>,
        character_ascii: bool,
    ) -> Result<rstyle_pdg::concrete_types::Observation, String> {
        if character_ascii && shared_character(ty) {
            use rstyle_pdg::concrete_types::{Observation, TypeTree};
            return Ok(Observation::Captured {
                ty: TypeTree::Shared(Box::new(TypeTree::Scalar("char".into()))),
            });
        }
        self.observe(ty)
    }

    fn receiver_types(
        &mut self,
        receiver: &Expr<'tcx>,
        character_ascii: bool,
    ) -> Result<ReceiverTypes, String> {
        let adjustments = self.ctx.typeck.expr_adjustments(receiver);
        if adjustments.len() > records::MAX_ADJUSTMENTS {
            return Err("Method receiver capture exceeds adjustment budget".into());
        }
        let adjustments = adjustments
            .iter()
            .map(|adjustment| {
                Ok(records::Adjustment {
                    kind: adjustment_kind(&adjustment.kind),
                    target: self.method_observation(adjustment.target, character_ascii)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(ReceiverTypes {
            source: self.observe(self.ctx.typeck.expr_ty(receiver))?,
            adjusted: self
                .method_observation(self.ctx.typeck.expr_ty_adjusted(receiver), character_ascii)?,
            adjustments,
        })
    }
}

fn adjustment_kind(kind: &Adjust) -> records::Kind {
    use records::Kind;
    match kind {
        Adjust::Deref(DerefAdjustKind::Builtin) => Kind::BuiltinDereference,
        Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not)) => Kind::SharedBorrow,
        Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Mut { .. })) => Kind::MutableBorrow,
        _ => Kind::Unsupported(
            match kind {
                Adjust::NeverToAny => "never_coercion",
                Adjust::Deref(DerefAdjustKind::Overloaded(_)) => "overloaded_dereference",
                Adjust::Deref(DerefAdjustKind::Pin) => "pin_dereference",
                Adjust::Borrow(AutoBorrow::RawPtr(_)) => "raw_borrow",
                Adjust::Borrow(AutoBorrow::Pin(_)) => "pin_borrow",
                Adjust::Pointer(_) => "pointer_coercion",
                Adjust::GenericReborrow(_) => "generic_reborrow",
                _ => unreachable!("supported adjustments handled above"),
            }
            .into(),
        ),
    }
}

fn shared_character(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Ref(_, inner, rustc_hir::Mutability::Not) if matches!(inner.kind(), ty::Char))
}
