//! Read complete initialized pointer-free bool arrays from rustc const evaluation.
use super::hir::Context;
use rstyle_pdg::{
    PathRes,
    bool_constants::{self, BoolArray, ConstFunction},
};
use rustc_abi::Size;
use rustc_hir::{
    ConstItemRhs, ExprKind, ItemKind,
    def::{DefKind, Res},
};
use rustc_middle::{
    mir::{
        ConstValue,
        interpret::{GlobalAlloc, alloc_range},
    },
    ty::{self, Ty, consts::ConstExt},
};

impl<'tcx> Context<'tcx> {
    pub(super) fn bool_array_resolution(&self, resolution: Res) -> Option<PathRes> {
        let Res::Def(kind, definition) = resolution else {
            return None;
        };
        if !self.profile.supports(bool_constants::CAPABILITY)
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
        let ty::Array(element, length) = ty.kind() else {
            return None;
        };
        let length = length.try_to_target_usize(self.tcx)?;
        if *element != self.tcx.types.bool || !(1..=bool_constants::MAX_LENGTH).contains(&length) {
            return None;
        }

        let declaration = self.constant_declaration(definition)?;
        bool_constants::array_declaration(&declaration).ok()?;
        let values = self.bool_array_value(definition, ty, length)?;
        let initializer = self.array_initializer(definition)?;
        let token = rstyle_pdg::data_types::TypeToken::Array {
            element: Box::new(rstyle_pdg::data_types::TypeToken::Scalar("bool".into())),
            length,
        }
        .text()
        .ok()?;
        encode(
            bool_constants::RESOLUTION,
            &BoolArray {
                definition: self.tcx.def_path_str(definition),
                declaration,
                ty: token,
                values,
                initializer,
            },
        )
    }

    fn bool_array_value(
        &self,
        definition: rustc_hir::def_id::DefId,
        ty: Ty<'tcx>,
        length: u64,
    ) -> Option<Vec<bool>> {
        let environment = ty::TypingEnv::post_analysis(self.tcx, definition);
        let layout = self.tcx.layout_of(environment.as_query_input(ty)).ok()?;
        if layout.size.bytes() != length || layout.align.abi.bytes() != 1 {
            return None;
        }

        let ConstValue::Indirect { alloc_id, offset } =
            self.tcx.const_eval_poly(definition).ok()?
        else {
            return None;
        };
        let GlobalAlloc::Memory(allocation) = self.tcx.global_alloc(alloc_id) else {
            return None;
        };
        let allocation = allocation.inner();
        let end = offset.bytes().checked_add(length)?;
        if end > allocation.size().bytes() {
            return None;
        }
        let bytes = allocation
            .get_bytes_strip_provenance(&self.tcx, alloc_range(offset, Size::from_bytes(length)))
            .ok()?;
        bytes
            .iter()
            .map(|value| match value {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            })
            .collect()
    }

    fn array_initializer(&self, definition: rustc_hir::def_id::DefId) -> Option<String> {
        let rustc_hir::Node::Item(item) = self.tcx.hir_node_by_def_id(definition.expect_local())
        else {
            return None;
        };
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

        let typeck = self.tcx.typeck(definition.expect_local());
        let Res::Def(DefKind::Fn, initializer) = typeck.qpath_res(path, callee.hir_id) else {
            return None;
        };
        if !initializer.is_local() || self.tcx.parent(initializer) != self.tcx.parent(definition) {
            return None;
        }
        Some(self.tcx.def_path_str(initializer))
    }

    pub(super) fn const_array_item(
        &self,
        item: &rustc_hir::Item<'tcx>,
    ) -> Option<(&'static str, PathRes)> {
        if !self.profile.supports(bool_constants::CAPABILITY) || item.span.from_expansion() {
            return None;
        }
        let definition = item.owner_id.to_def_id();
        if matches!(item.kind, ItemKind::Const(..))
            && let Some(resolution) =
                self.bool_array_resolution(Res::Def(DefKind::Const, definition))
        {
            return Some(("stmt_bool_array", resolution));
        }
        if !matches!(item.kind, ItemKind::Fn { .. }) {
            return None;
        }

        let declaration = self.constant_declaration(definition)?;
        bool_constants::function_declaration(&declaration).ok()?;
        let ty = self
            .tcx
            .fn_sig(definition)
            .instantiate_identity()
            .skip_binder()
            .output();
        let ty::Array(element, length) = ty.kind() else {
            return None;
        };
        if *element != self.tcx.types.bool {
            return None;
        }
        let token = rstyle_pdg::data_types::TypeToken::Array {
            element: Box::new(rstyle_pdg::data_types::TypeToken::Scalar("bool".into())),
            length: length.try_to_target_usize(self.tcx)?,
        }
        .text()
        .ok()?;
        Some((
            "stmt_const_array_fn",
            encode(
                bool_constants::FUNCTION,
                &ConstFunction {
                    definition: self.tcx.def_path_str(definition),
                    declaration,
                    ty: token,
                },
            )?,
        ))
    }
}

fn encode<T: serde::Serialize>(kind: &str, record: &T) -> Option<PathRes> {
    Some(PathRes {
        kind: kind.into(),
        target: None,
        description: serde_json::to_string(record).ok()?,
    })
}
