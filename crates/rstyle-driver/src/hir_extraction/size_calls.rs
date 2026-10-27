//! Capture the actual monomorphized scalar argument, never a guessed literal.
use super::hir::Context;
use rstyle_pdg::{
    PathRes,
    size_calls::{CAPABILITY, Capture, RESOLUTION},
};
use rustc_hir::{Expr, def_id::DefId};

impl Context<'_> {
    pub(super) fn size_callee(&self, expression: &Expr<'_>, callee: DefId) -> Option<PathRes> {
        if !self.profile.supports(CAPABILITY)
            || callee.is_local()
            || self.tcx.crate_name(callee.krate).as_str() != "core"
            || !matches!(
                self.tcx.def_path_str(callee).as_str(),
                "core::mem::size_of" | "std::mem::size_of"
            )
        {
            return None;
        }

        let arguments = self.typeck.node_args(expression.hir_id);
        let mut types = arguments.types();
        let argument = types.next()?;
        if types.next().is_some()
            || ![self.tcx.types.u16, self.tcx.types.u32, self.tcx.types.u128].contains(&argument)
        {
            return None;
        }
        let signature = self
            .tcx
            .fn_sig(callee)
            .instantiate(self.tcx, arguments)
            .skip_binder();
        if !signature.inputs().is_empty() || signature.output() != self.tcx.types.usize {
            return None;
        }

        let resolution = self.library_callee(callee);
        let capture = Capture {
            callee: serde_json::from_str(&resolution.description).expect("captured size callee"),
            argument: argument.to_string(),
        };
        Some(PathRes {
            kind: RESOLUTION.into(),
            target: None,
            description: serde_json::to_string(&capture).expect("captured scalar size argument"),
        })
    }
}
