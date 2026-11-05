//! Resolved source-facing HIR records. No graph derivation lives here.

mod for_loops;

use rstyle_pdg::frontend::*;
use rustc_hir::{Expr, ExprKind, HirId, def::Res};
use rustc_middle::ty::{self, Ty, TyCtxt, TypeckResults};
use rustc_span::Span;

pub struct Context<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub typeck: &'tcx TypeckResults<'tcx>,
    pub function_span: Span,
    pub source_bytes: SourceText,
    pub sources: super::sources::Sources,
    pub unit_context: Option<rstyle_pdg::nominal_boundary::UnitContext>,
    pub result: Ty<'tcx>,
    pub profile: rstyle_pdg::extraction_profile::ExtractionProfile,
}

pub fn reference(id: HirId) -> HirRef {
    HirRef {
        krate: 0,
        owner: id.owner.def_id.local_def_index.as_u32(),
        local: id.local_id.as_u32(),
    }
}

pub fn resolved(ty: Ty<'_>, spelling: String) -> ResolvedTy {
    let normalized = match ty.kind() {
        ty::Int(_) | ty::Uint(_) | ty::Bool | ty::Char => Some(ty.to_string()),
        ty::Float(ty::FloatTy::F32 | ty::FloatTy::F64) => Some(ty.to_string()),
        ty::Tuple(types) if types.is_empty() => Some("unit".to_string()),
        _ => None,
    };

    ResolvedTy {
        normalized,
        observed: ty.to_string(),
        spelling,
    }
}

impl<'tcx> Context<'tcx> {
    pub fn span(&self, span: Span) -> ByteSpan {
        let span = span.source_callsite();
        let file = self
            .tcx
            .sess
            .source_map()
            .lookup_source_file(self.function_span.lo());
        let origin = file.original_relative_byte_pos(self.function_span.lo()).0;
        ByteSpan {
            start: file
                .original_relative_byte_pos(span.lo())
                .0
                .saturating_sub(origin),
            end: file
                .original_relative_byte_pos(span.hi())
                .0
                .saturating_sub(origin),
        }
    }

    pub fn snippet(&self, span: Span) -> String {
        let span = self.span(span);
        self.source_bytes
            .get(span.start as usize..span.end as usize)
            .unwrap_or_default()
            .to_string()
    }

    pub fn node(&self, id: HirId, span: Span, kind: &str) -> HirNode {
        HirNode {
            hir: reference(id),
            kind: kind.to_string(),
            span: self.span(span),
            snippet: self.snippet(span),
            ty: None,
            res: None,
            dispatch: None,
            expansion: None,
            binding: None,
            binding_mut: None,
            lit: None,
            param_index: None,
            is_unsafe_block: false,
            children: Vec::new(),
        }
    }

    pub(super) fn dispatch_operands(
        &self,
        expr: &Expr<'tcx>,
    ) -> Option<(String, Vec<&'tcx Expr<'tcx>>)> {
        Some(match expr.kind {
            ExprKind::Binary(op, left, right) => (op.node.as_str().to_string(), vec![left, right]),
            ExprKind::AssignOp(op, left, right) => {
                (op.node.as_str().to_string(), vec![left, right])
            }
            ExprKind::Unary(op, value) => (op.as_str().to_string(), vec![value]),
            ExprKind::Index(base, index, _) => ("[]".into(), vec![base, index]),
            ExprKind::Cast(value, _) => ("as".to_string(), vec![value]),
            ExprKind::Call(callee, args) => {
                let resolution = self.direct_callee(callee)?;
                let value: serde_json::Value =
                    serde_json::from_str(&resolution.description).ok()?;
                let path = if resolution.kind == rstyle_pdg::size_calls::RESOLUTION {
                    &value["callee"]["path"]
                } else {
                    &value["path"]
                };
                (path.as_str()?.to_string(), args.iter().collect())
            }
            ExprKind::MethodCall(_, receiver, args, _) => (
                self.tcx
                    .def_path_str(self.typeck.type_dependent_def_id(expr.hir_id)?),
                std::iter::once(receiver).chain(args.iter()).collect(),
            ),
            _ => return None,
        })
    }
}

impl Context<'_> {
    fn instantiated_callee(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
        capture: bool,
    ) -> PathRes {
        let mut resolution = self.library_callee(callee);
        if capture {
            let mut record: rstyle_pdg::data_library::Callee =
                serde_json::from_str(&resolution.description).expect("captured library callee");
            record.implementation = self.selected_implementation(expression, callee);
            resolution.description =
                serde_json::to_string(&record).expect("captured library callee");
        }
        resolution
    }

    fn scalar_from_capture(&self, expression: &Expr<'_>, callee: rustc_hir::def_id::DefId) -> bool {
        if !self.profile.supports("rust.scalar-from-conversions")
            || !self
                .tcx
                .def_path_str(callee)
                .ends_with("::convert::From::from")
        {
            return false;
        }
        let signature = self
            .tcx
            .fn_sig(callee)
            .instantiate(self.tcx, self.typeck.node_args(expression.hir_id))
            .skip_binder();

        signature.inputs() == [self.tcx.types.u16] && signature.output() == self.tcx.types.u32
            || self.profile.supports("rust.usize-from-u8")
                && signature.inputs() == [self.tcx.types.u8]
                && signature.output() == self.tcx.types.usize
            || self.profile.supports("rust.tagged-unit-classifiers")
                && signature.inputs() == [self.tcx.types.u16]
                && signature.output() == self.tcx.types.usize
            || self
                .profile
                .supports("rust.unsigned-index-conversion-family")
                && signature.inputs() == [self.tcx.types.u16]
                && signature.output() == self.tcx.types.u64
            || self.profile.supports("rust.u64-from-u32")
                && signature.inputs() == [self.tcx.types.u32]
                && signature.output() == self.tcx.types.u64
    }

    fn int_conversion_capture(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> bool {
        if !self.profile.supports("rust.bitset-cardinality")
            || !self
                .tcx
                .def_path_str(callee)
                .ends_with("::convert::TryFrom::try_from")
        {
            return false;
        }
        let signature = self
            .tcx
            .fn_sig(callee)
            .instantiate(self.tcx, self.typeck.node_args(expression.hir_id))
            .skip_binder();
        let ty::Adt(definition, arguments) = signature.output().kind() else {
            return false;
        };

        self.tcx
            .is_diagnostic_item(rustc_span::sym::Result, definition.did())
            && (signature.inputs() == [self.tcx.types.u32]
                && arguments.type_at(0) == self.tcx.types.usize
                || self.profile.supports("rust.end-of-input")
                    && signature.inputs() == [self.tcx.types.usize]
                    && arguments.type_at(0) == self.tcx.types.u16)
    }

    // Observational selection only; this does not authorize trait calls or
    // replace independent source ownership, type, effect or drop evidence.
    fn local_from_capture(&self, expression: &Expr<'_>, callee: rustc_hir::def_id::DefId) -> bool {
        self.tcx
            .associated_item(callee)
            .trait_container(self.tcx)
            .is_some_and(|definition| {
                self.tcx
                    .is_diagnostic_item(rustc_span::sym::From, definition)
            })
            && self.tcx.item_name(callee) == rustc_span::sym::from
            && self
                .selected_callee(expression, callee)
                .is_some_and(|selected| {
                    selected.is_local() && self.tcx.generics_of(selected).count() == 0
                })
    }

    fn range_from_helper(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> Option<PathRes> {
        if !self.profile.supports("rust.fresh-range-results")
            || !self.local_from_capture(expression, callee)
        {
            return None;
        }

        let signature = self
            .tcx
            .fn_sig(callee)
            .instantiate(self.tcx, self.typeck.node_args(expression.hir_id))
            .skip_binder();
        let ty::Adt(definition, arguments) = signature.output().kind() else {
            return None;
        };
        if Some(definition.did()) != self.tcx.lang_items().range_struct()
            || arguments.len() != 1
            || arguments.type_at(0) != self.tcx.types.usize
        {
            return None;
        }

        let selected = self.selected_callee(expression, callee)?;
        let mut resolution = self.helper_callee(selected, "resolved_direct_callee", None);
        let mut record: serde_json::Value = serde_json::from_str(&resolution.description).ok()?;
        let selection = self.instantiated_callee(expression, callee, true);
        record["trait_selection"] = serde_json::from_str(&selection.description).ok()?;
        resolution.description = record.to_string();

        Some(resolution)
    }

    fn selected_callee(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> Option<rustc_hir::def_id::DefId> {
        crate::ids::selected_instance(
            self.tcx,
            expression.hir_id.owner.def_id,
            callee,
            self.typeck.node_args(expression.hir_id),
        )
        .map(|instance| instance.def_id())
    }

    fn selected_implementation(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> Option<Box<rstyle_pdg::data_library::Callee>> {
        let implementation = self.library_callee(self.selected_callee(expression, callee)?);
        Some(Box::new(
            serde_json::from_str(&implementation.description).expect("captured implementation"),
        ))
    }

    pub(super) fn library_callee(&self, callee: rustc_hir::def_id::DefId) -> PathRes {
        let callee = rstyle_pdg::data_library::Callee {
            path: self.tcx.def_path_str(callee),
            crate_name: self.tcx.crate_name(callee.krate).to_string(),
            crate_hash: if callee.is_local() {
                "local-unencoded".into()
            } else {
                self.tcx.crate_hash(callee.krate).to_string()
            },
            definition_hash: format!("{:?}", self.tcx.def_path_hash(callee)),
            implementation: None,
        };

        PathRes {
            kind: "resolved_callee".into(),
            target: None,
            description: serde_json::to_value(callee)
                .expect("serialize resolved callee")
                .to_string(),
        }
    }

    fn direct_callee(&self, expression: &Expr<'_>) -> Option<PathRes> {
        if !self.profile.supports("rust.verified-helper-calls") {
            return None;
        }
        let ExprKind::Path(ref path) = expression.kind else {
            return None;
        };
        let Res::Def(kind, callee) = self.typeck.qpath_res(path, expression.hir_id) else {
            return None;
        };
        if self.profile.supports("rust.scalar-enum-values")
            && callee.is_local()
            && matches!(
                kind,
                rustc_hir::def::DefKind::Ctor(rustc_hir::def::CtorOf::Variant, _)
            )
        {
            return Some(self.library_callee(callee));
        }
        if !callee.is_local() && kind == rustc_hir::def::DefKind::AssocFn && self.profile.is_data()
        {
            return Some(self.external_associated_callee(expression, callee));
        }
        if kind == rustc_hir::def::DefKind::Fn
            && let Some(resolution) = self.size_callee(expression, callee)
        {
            return Some(resolution);
        }
        let inherent = self.inherent_callee(expression, callee);
        if self.tcx.generics_of(callee).count() != 0 && inherent.is_none() {
            return None;
        }
        if kind != rustc_hir::def::DefKind::Fn && inherent.is_none() {
            return None;
        }

        let dependency = self.compiled_dependency(expression, callee, kind);
        Some(self.helper_resolution(
            callee,
            "resolved_direct_callee",
            HelperEvidence {
                owner: inherent,
                compiled_dependency: dependency,
            },
        ))
    }

    fn external_associated_callee(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> PathRes {
        if let Some(resolution) = super::trait_members::default_call(self, expression, callee) {
            return resolution;
        }
        if let Some(resolution) = self.range_from_helper(expression, callee) {
            return resolution;
        }

        self.associated_callee(expression, callee)
    }

    fn associated_callee(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> PathRes {
        let capture = self.scalar_from_capture(expression, callee)
            || self.int_conversion_capture(expression, callee)
            || self.local_from_capture(expression, callee);
        self.instantiated_callee(expression, callee, capture)
    }

    fn compiled_dependency(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
        kind: rustc_hir::def::DefKind,
    ) -> Option<super::crate_association::Dependency> {
        (super::crate_association::enabled(self.profile)
            && kind == rustc_hir::def::DefKind::Fn
            && self.typeck.node_args(expression.hir_id).is_empty())
        .then(|| super::crate_association::dependency(self.tcx, callee))
        .flatten()
    }

    fn inherent_callee(
        &self,
        expression: &Expr<'_>,
        callee: rustc_hir::def_id::DefId,
    ) -> Option<rustc_hir::def_id::DefId> {
        let owner = super::inherent::implementation(self.tcx, callee, self.profile)?;
        self.typeck
            .node_args(expression.hir_id)
            .iter()
            .all(|argument| {
                matches!(
                    argument.kind(),
                    rustc_middle::ty::GenericArgKind::Lifetime(_)
                )
            })
            .then_some(owner)
    }

    pub(super) fn helper_callee(
        &self,
        callee: rustc_hir::def_id::DefId,
        kind: &str,
        owner: Option<rustc_hir::def_id::DefId>,
    ) -> PathRes {
        self.helper_resolution(
            callee,
            kind,
            HelperEvidence {
                owner,
                compiled_dependency: None,
            },
        )
    }

    fn helper_resolution(
        &self,
        callee: rustc_hir::def_id::DefId,
        kind: &str,
        evidence: HelperEvidence,
    ) -> PathRes {
        PathRes {
            kind: kind.into(),
            target: None,
            description: serde_json::to_string(&DirectCallee {
                path: self.tcx.def_path_str(callee),
                crate_name: self.tcx.crate_name(callee.krate).to_string(),
                local: callee.is_local(),
                safe: callee_safety(self.tcx, callee),
                definition_identity: format!("{:?}", self.tcx.def_path_hash(callee)),
                inherent: evidence
                    .owner
                    .map(|owner| super::inherent::call_owner(self.tcx, owner)),
                compiled_dependency: evidence.compiled_dependency,
            })
            .expect("resolved callee record"),
        }
    }
}

struct HelperEvidence {
    owner: Option<rustc_hir::def_id::DefId>,
    compiled_dependency: Option<super::crate_association::Dependency>,
}

#[derive(serde::Serialize)]
struct DirectCallee {
    path: String,
    #[serde(rename = "crate")]
    crate_name: String,
    local: bool,
    definition_identity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    safe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inherent: Option<rstyle_pdg::inherent_calls::CallOwner>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compiled_dependency: Option<super::crate_association::Dependency>,
}

fn callee_safety(tcx: TyCtxt<'_>, callee: rustc_hir::def_id::DefId) -> Option<bool> {
    matches!(
        tcx.def_kind(callee),
        rustc_hir::def::DefKind::Fn
            | rustc_hir::def::DefKind::AssocFn
            | rustc_hir::def::DefKind::Ctor(..)
    )
    .then(|| {
        tcx.fn_sig(callee)
            .instantiate_identity()
            .skip_binder()
            .safety()
            .is_safe()
    })
}

pub(crate) fn kind(kind: &ExprKind<'_>) -> &'static str {
    match kind {
        ExprKind::Block(..) => "expr_block",
        ExprKind::Binary(..) => "expr_binary",
        ExprKind::Unary(..) => "expr_unary",
        ExprKind::Lit(..) => "expr_lit",
        ExprKind::Path(..) => "expr_path",
        ExprKind::Tup(..) => "expr_tup",
        ExprKind::If(..) => "expr_if",
        ExprKind::Ret(..) => "expr_ret",
        ExprKind::Assign(..) => "expr_assign",
        ExprKind::AssignOp(..) => "expr_assign_op",
        ExprKind::Call(..) => "expr_call",
        ExprKind::MethodCall(..) => "expr_method_call",
        ExprKind::Closure(..) => "expr_closure",
        ExprKind::ConstBlock(..) => "expr_const_block",
        ExprKind::AddrOf(..) => "expr_addr_of",
        ExprKind::Field(..) => "expr_field",
        ExprKind::Index(..) => "expr_index",
        ExprKind::Loop(..) => "expr_loop",
        ExprKind::Match(..) => "expr_match",
        ExprKind::Cast(..) => "expr_cast",
        ExprKind::Break(..) => "expr_break",
        ExprKind::Continue(..) => "expr_continue",
        ExprKind::Let(..) => "expr_let",
        ExprKind::Yield(..) => "expr_yield",
        ExprKind::Array(..) => "expr_array",
        ExprKind::Repeat(..) => "expr_repeat",
        ExprKind::Struct(..) => "expr_struct",
        _ => "expr_unsupported",
    }
}
