//! Exact Try-desugaring observations, without extraction permission.
use super::{
    hir::{Context, reference},
    library_resolution::definition,
    results::payloads,
};
use rstyle_pdg::{PathRes, result_propagation::capture::*};
use rustc_hir::{
    self as hir, Expr, ExprKind,
    def::{DefKind, Res},
    def_id::DefId,
};
use rustc_middle::ty::{self, Ty};
mod borrowed;
mod named_error;

pub(super) enum Protocol {
    Result,
    BorrowedResult,
    ScalarResult,
    NamedErrorResult(rstyle_pdg::named_error_result_propagation::Mode),
    ProductResult,
    Option,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
enum CapturedShape {
    Result(Shape),
    ScalarResult(rstyle_pdg::scalar_result_propagation::capture::Shape),
    NamedErrorResult(rstyle_pdg::named_error_result_propagation::capture::Shape),
    BorrowedResult(rstyle_pdg::borrowed_result_propagation::capture::Shape),
    ProductResult(rstyle_pdg::product_result_propagation::capture::Shape),
    Option(rstyle_pdg::option_propagation::capture::Shape),
}

#[derive(serde::Serialize)]
struct CapturedPropagation {
    identity: &'static str,
    root: rstyle_pdg::HirRef,
    operand: rstyle_pdg::HirRef,
    try_span: rstyle_pdg::ByteSpan,
    operand_span: rstyle_pdg::ByteSpan,
    branch_call: rstyle_pdg::HirRef,
    residual_call: rstyle_pdg::HirRef,
    early_return: rstyle_pdg::HirRef,
    shape: CapturedShape,
    branch: Selection,
    residual: Selection,
    #[serde(skip_serializing_if = "Option::is_none")]
    conversion: Option<Selection>,
    break_arm: Arm,
    continue_arm: Arm,
}

impl Protocol {
    fn metadata(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Result => ("rust.same-error-result-propagation", RESOLUTION, IDENTITY),
            Self::BorrowedResult => (
                "rust.checked-borrowed-result-flow",
                rstyle_pdg::borrowed_result_propagation::capture::RESOLUTION,
                rstyle_pdg::borrowed_result_propagation::capture::IDENTITY,
            ),
            Self::NamedErrorResult(mode) => (mode.capability(), mode.resolution(), mode.identity()),
            Self::ScalarResult => (
                "rust.scalar-success-result-propagation",
                rstyle_pdg::scalar_result_propagation::capture::RESOLUTION,
                rstyle_pdg::scalar_result_propagation::capture::IDENTITY,
            ),
            Self::ProductResult => (
                rstyle_pdg::product_result_propagation::CAPABILITY,
                rstyle_pdg::product_result_propagation::capture::RESOLUTION,
                rstyle_pdg::product_result_propagation::capture::IDENTITY,
            ),
            Self::Option => (
                "rust.concrete-option-propagation",
                rstyle_pdg::option_propagation::capture::RESOLUTION,
                rstyle_pdg::option_propagation::capture::IDENTITY,
            ),
        }
    }
}

impl<'tcx> Context<'tcx> {
    pub(super) fn result_propagation(&self, root: &Expr<'tcx>) -> Option<PathRes> {
        use rstyle_pdg::named_error_result_propagation::Mode;
        [
            Protocol::Result,
            Protocol::BorrowedResult,
            Protocol::ScalarResult,
            Protocol::NamedErrorResult(Mode::UnitToReader),
            Protocol::NamedErrorResult(Mode::UnitToU16Reader),
            Protocol::NamedErrorResult(Mode::ReaderToScalar),
            Protocol::NamedErrorResult(Mode::ReaderToCopyProduct),
            Protocol::NamedErrorResult(Mode::IdToReader),
            Protocol::ProductResult,
        ]
        .into_iter()
        .find_map(|protocol| self.propagation_capture(root, protocol))
    }

    pub(super) fn propagation_capture(
        &self,
        root: &Expr<'tcx>,
        protocol: Protocol,
    ) -> Option<PathRes> {
        let (capability, resolution, identity) = protocol.metadata();
        if !self.profile.supports(capability) {
            return None;
        }
        let witness = self.propagation_witness(root, protocol, identity)?;
        Some(PathRes {
            kind: resolution.into(),
            target: None,
            description: serde_json::to_string(&witness).expect("Try desugaring witness"),
        })
    }

    fn propagation_witness(
        &self,
        root: &Expr<'tcx>,
        protocol: Protocol,
        identity: &'static str,
    ) -> Option<CapturedPropagation> {
        let ExprKind::Match(branch, arms, hir::MatchSource::TryDesugar(_)) = root.kind else {
            return None;
        };
        let [first, second] = arms else {
            return None;
        };
        let [operand] = call_arguments(branch)? else {
            return None;
        };
        let (shape, conversion) = self.protocol_shape(root, branch, protocol)?;
        let (break_arm, continue_arm) = self.propagation_arms(first, second)?;
        let ExprKind::Ret(Some(residual)) = break_arm.body.kind else {
            return None;
        };
        let [usage] = call_arguments(residual)? else {
            return None;
        };
        let mut broken = self.propagation_arm(break_arm)?;
        broken.usage = reference(usage.hir_id);
        let continued = self.propagation_arm(continue_arm)?;
        self.propagation_use(usage, &broken)?;
        self.propagation_use(continue_arm.body, &continued)?;

        Some(CapturedPropagation {
            identity,
            root: reference(root.hir_id),
            operand: reference(operand.hir_id),
            try_span: self.span(root.span),
            operand_span: self.span(operand.span),
            branch_call: reference(branch.hir_id),
            residual_call: reference(residual.hir_id),
            early_return: reference(break_arm.body.hir_id),
            shape,
            branch: self.propagation_call(branch)?,
            residual: self.propagation_call(residual)?,
            conversion,
            break_arm: broken,
            continue_arm: continued,
        })
    }

    fn protocol_shape(
        &self,
        root: &Expr<'tcx>,
        branch: &Expr<'tcx>,
        protocol: Protocol,
    ) -> Option<(CapturedShape, Option<Selection>)> {
        let [operand] = call_arguments(branch)? else {
            return None;
        };

        Some(match protocol {
            Protocol::Result => (
                CapturedShape::Result(self.propagation_shape(operand, branch)?),
                Some(self.propagation_conversion(root, operand)?),
            ),
            Protocol::BorrowedResult => (
                CapturedShape::BorrowedResult(self.borrowed_result_shape(operand, branch)?),
                Some(self.propagation_conversion(root, operand)?),
            ),
            Protocol::NamedErrorResult(mode) => (
                CapturedShape::NamedErrorResult(
                    self.named_error_result_shape(operand, branch, mode)?,
                ),
                Some(self.propagation_conversion(root, operand)?),
            ),
            Protocol::ScalarResult => (
                CapturedShape::ScalarResult(self.scalar_result_shape(operand, branch)?),
                Some(self.propagation_conversion(root, operand)?),
            ),
            Protocol::ProductResult => (
                CapturedShape::ProductResult(self.product_result_shape(operand, branch)?),
                Some(self.propagation_conversion(root, operand)?),
            ),
            Protocol::Option => (
                CapturedShape::Option(self.option_propagation_shape(operand, branch)?),
                None,
            ),
        })
    }

    pub(super) fn propagation_arms(
        &self,
        first: &'tcx hir::Arm<'tcx>,
        second: &'tcx hir::Arm<'tcx>,
    ) -> Option<(&'tcx hir::Arm<'tcx>, &'tcx hir::Arm<'tcx>)> {
        match self
            .propagation_arm(first)?
            .variant
            .path
            .rsplit("::")
            .next()?
        {
            "Break" => Some((first, second)),
            "Continue" => Some((second, first)),
            _ => None,
        }
    }

    fn propagation_shape(&self, operand: &Expr<'tcx>, branch: &Expr<'tcx>) -> Option<Shape> {
        let input = self.typeck.expr_ty(operand);
        let (ok, error) = payloads(self.tcx, input)?;
        if !ok.is_unit() || input != self.result {
            return None;
        }

        let control_flow = self.typeck.expr_ty(branch);
        let ty::Adt(_, args) = control_flow.kind() else {
            return None;
        };
        if args.len() != 2 || !args.type_at(1).is_unit() {
            return None;
        }

        let residual = args.type_at(0);
        let (never, same_error) = payloads(self.tcx, residual)?;
        if !never.is_never() || error != same_error {
            return None;
        }

        Some(Shape {
            result: input.to_string(),
            error: error.to_string(),
            residual: residual.to_string(),
            control_flow: control_flow.to_string(),
        })
    }

    pub(super) fn propagation_arm(&self, arm: &hir::Arm<'tcx>) -> Option<Arm> {
        if arm.guard.is_some() {
            return None;
        }
        let hir::PatKind::Struct(ref path, fields, rest) = arm.pat.kind else {
            return None;
        };
        let [field] = fields else {
            return None;
        };
        if rest.is_some() || field.ident.name.as_str() != "0" || field.is_shorthand {
            return None;
        }

        let binding = field.pat;
        let hir::PatKind::Binding(
            hir::BindingMode(hir::ByRef::No, hir::Mutability::Not),
            id,
            _,
            None,
        ) = binding.kind
        else {
            return None;
        };

        let Res::Def(DefKind::Variant, variant) = self.typeck.qpath_res(path, arm.pat.hir_id)
        else {
            return None;
        };
        let ty::Adt(owner, _) = self.typeck.node_type(arm.pat.hir_id).kind() else {
            return None;
        };
        if self.tcx.parent(variant) != owner.did() {
            return None;
        }

        Some(Arm {
            arm: reference(arm.hir_id),
            pattern: reference(arm.pat.hir_id),
            binding: reference(id),
            usage: reference(arm.body.hir_id),
            variant: definition(self.tcx, variant),
        })
    }

    pub(super) fn propagation_use(&self, expression: &Expr<'tcx>, arm: &Arm) -> Option<()> {
        let ExprKind::Path(ref path) = expression.kind else {
            return None;
        };
        let Res::Local(target) = self.typeck.qpath_res(path, expression.hir_id) else {
            return None;
        };
        (reference(target) == arm.binding).then_some(())
    }

    pub(super) fn propagation_call(&self, expression: &Expr<'tcx>) -> Option<Selection> {
        let ExprKind::Call(callee, _) = expression.kind else {
            return None;
        };
        let ExprKind::Path(ref path) = callee.kind else {
            return None;
        };
        let Res::Def(_, method) = self.typeck.qpath_res(path, callee.hir_id) else {
            return None;
        };

        let callee_ty = self.typeck.expr_ty(callee);
        let ty::FnDef(typed_method, arguments) = callee_ty.kind() else {
            return None;
        };
        if *typed_method != method || !arguments.bound_vars().is_empty() {
            return None;
        }
        self.propagation_selection(expression, method, arguments.skip_binder())
    }

    fn propagation_selection(
        &self,
        root: &Expr<'tcx>,
        method: DefId,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Option<Selection> {
        if self.tcx.def_kind(method) != DefKind::AssocFn
            || args.len() != self.tcx.generics_of(method).count()
            || args
                .iter()
                .any(|arg| !matches!(arg.kind(), ty::GenericArgKind::Type(_)))
        {
            return None;
        }

        let environment = ty::TypingEnv::post_analysis(self.tcx, root.hir_id.owner.def_id);
        let selected = ty::Instance::try_resolve(self.tcx, environment, method, args);
        let instance = selected.ok()??;

        let signature = self
            .tcx
            .fn_sig(method)
            .instantiate(self.tcx, args)
            .map(|signature| signature.skip_binder());
        let normalized = self
            .tcx
            .try_normalize_erasing_regions(environment, signature);
        let signature = normalized.ok()?;
        let [input] = signature.inputs() else {
            return None;
        };

        Some(Selection {
            method: definition(self.tcx, method),
            implementation: definition(self.tcx, instance.def_id()),
            arguments: args.iter().map(|arg| arg.to_string()).collect(),
            implementation_arguments: instance.args.iter().map(|arg| arg.to_string()).collect(),
            input: input.to_string(),
            output: signature.output().to_string(),
        })
    }

    fn propagation_conversion(&self, root: &Expr<'tcx>, operand: &Expr<'tcx>) -> Option<Selection> {
        let (_, error): (Ty<'tcx>, Ty<'tcx>) = payloads(self.tcx, self.typeck.expr_ty(operand))?;
        let method = self
            .tcx
            .get_diagnostic_item(rustc_span::Symbol::intern("from_fn"))?;
        let args = self.tcx.mk_args(&[error.into(), error.into()]);
        self.propagation_selection(root, method, args)
    }
}

pub(super) fn call_arguments<'tcx>(expression: &Expr<'tcx>) -> Option<&'tcx [Expr<'tcx>]> {
    let ExprKind::Call(_, arguments) = expression.kind else {
        return None;
    };
    Some(arguments)
}
