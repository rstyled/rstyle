//! `?` propagations, as the observation the frontend records for each protocol it
//! verifies: the branch and residual calls, their selected implementations, the error
//! conversion and the two arms of the lowering.

mod shapes;

use super::{Context, hir_ref, identity};
use crate::frontend::{ByteSpan, HirRef, PathRes};
use crate::named_error_result_propagation::Mode;
use crate::result_propagation::capture::{Arm, Selection};
use rstyle_core::cpg::model::{Node, NodeKind, TargetKind};
use serde::Serialize;

#[derive(Clone, Copy)]
enum Protocol {
    Result,
    BorrowedResult,
    ScalarResult,
    NamedErrorResult(Mode),
    ProductResult,
    Option,
}

#[derive(Serialize)]
#[serde(untagged)]
enum CapturedShape {
    Result(crate::result_propagation::capture::Shape),
    ScalarResult(crate::scalar_result_propagation::capture::Shape),
    NamedErrorResult(crate::named_error_result_propagation::capture::Shape),
    BorrowedResult(crate::borrowed_result_propagation::capture::Shape),
    ProductResult(crate::product_result_propagation::capture::Shape),
    Option(crate::option_propagation::capture::Shape),
}

#[derive(Serialize)]
struct CapturedPropagation {
    identity: &'static str,
    root: HirRef,
    operand: HirRef,
    try_span: ByteSpan,
    operand_span: ByteSpan,
    branch_call: HirRef,
    residual_call: HirRef,
    early_return: HirRef,
    shape: CapturedShape,
    branch: Selection,
    residual: Selection,
    #[serde(skip_serializing_if = "Option::is_none")]
    conversion: Option<Selection>,
    break_arm: Arm,
    continue_arm: Arm,
}

impl Protocol {
    fn metadata(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Result => (
                "rust.same-error-result-propagation",
                crate::result_propagation::capture::RESOLUTION,
                crate::result_propagation::capture::IDENTITY,
            ),
            Self::BorrowedResult => (
                "rust.checked-borrowed-result-flow",
                crate::borrowed_result_propagation::capture::RESOLUTION,
                crate::borrowed_result_propagation::capture::IDENTITY,
            ),
            Self::NamedErrorResult(mode) => (mode.capability(), mode.resolution(), mode.identity()),
            Self::ScalarResult => (
                "rust.scalar-success-result-propagation",
                crate::scalar_result_propagation::capture::RESOLUTION,
                crate::scalar_result_propagation::capture::IDENTITY,
            ),
            Self::ProductResult => (
                crate::product_result_propagation::CAPABILITY,
                crate::product_result_propagation::capture::RESOLUTION,
                crate::product_result_propagation::capture::IDENTITY,
            ),
            Self::Option => (
                "rust.concrete-option-propagation",
                crate::option_propagation::capture::RESOLUTION,
                crate::option_propagation::capture::IDENTITY,
            ),
        }
    }
}

impl<'a> Context<'_, 'a> {
    /// The first protocol, in the frontend's order, that observes a `?`.
    pub(super) fn propagation(&self, node: &Node) -> Option<PathRes> {
        if node.kind != NodeKind::Propagate {
            return None;
        }
        [
            Protocol::Option,
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
        .find_map(|protocol| self.propagation_capture(node, protocol))
    }

    fn propagation_capture(&self, root: &Node, protocol: Protocol) -> Option<PathRes> {
        let (capability, resolution, identity) = protocol.metadata();
        if !self.reader.profile.supports(capability) {
            return None;
        }
        let witness = self.propagation_witness(root, protocol, identity)?;
        Some(PathRes {
            kind: resolution.into(),
            target: None,
            description: serde_json::to_string(&witness).ok()?,
        })
    }

    fn propagation_witness(
        &self,
        root: &Node,
        protocol: Protocol,
        identity: &'static str,
    ) -> Option<CapturedPropagation> {
        let children = self.body.children(root).ok()?;
        let [branch, first, second] = children.as_slice() else {
            return None;
        };
        let operand = single_argument(self, branch)?;
        let (shape, conversion) = self.protocol_shape(branch, operand, protocol)?;
        let (break_arm, continue_arm) = self.propagation_arms(first, second)?;
        let early_return = self.body.child(break_arm, 1).ok()?;
        if early_return.original_kind != "expr_ret" {
            return None;
        }
        let residual = self.body.child(early_return, 0).ok()?;
        let usage = single_argument(self, residual)?;
        let mut broken = self.propagation_arm(break_arm)?;
        broken.usage = hir_ref(&usage.id);
        let continued = self.propagation_arm(continue_arm)?;
        self.propagation_use(usage, &broken)?;
        self.propagation_use(self.body.child(continue_arm, 1).ok()?, &continued)?;

        Some(CapturedPropagation {
            identity,
            root: hir_ref(&root.id),
            operand: hir_ref(&operand.id),
            try_span: self.written(root),
            operand_span: self.written(operand),
            branch_call: hir_ref(&branch.id),
            residual_call: hir_ref(&residual.id),
            early_return: hir_ref(&early_return.id),
            shape,
            branch: self.propagation_call(branch, operand)?,
            residual: self.propagation_call(residual, usage)?,
            conversion,
            break_arm: broken,
            continue_arm: continued,
        })
    }

    /// The `Break` arm, then the `Continue` arm.
    fn propagation_arms(&self, first: &'a Node, second: &'a Node) -> Option<(&'a Node, &'a Node)> {
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

    /// An unguarded arm matching a variant's only field into a plain binding.
    fn propagation_arm(&self, arm: &'a Node) -> Option<Arm> {
        let parts = self.body.children(arm).ok()?;
        let [pattern, body] = parts.as_slice() else {
            return None;
        };
        let fields = self.body.children(pattern).ok()?;
        let [binding] = fields.as_slice() else {
            return None;
        };
        let plain = binding.original_kind == "pat_binding"
            && self.declared(binding).is_some_and(|(_, mutable)| !mutable);
        let variant = self.target_definition(pattern, TargetKind::Variant)?;
        let owner = self
            .reader
            .ty(pattern.ty.as_deref()?)
            .ok()?
            .definition
            .as_deref()?;
        if pattern.original_kind != "pat_struct"
            || !plain
            || identity(variant, "enum") != Some(owner)
        {
            return None;
        }

        Some(Arm {
            arm: hir_ref(&arm.id),
            pattern: hir_ref(&pattern.id),
            binding: hir_ref(&binding.id),
            usage: hir_ref(&body.id),
            variant: self.library_record(variant)?,
        })
    }

    /// Whether a use is the arm's own binding.
    fn propagation_use(&self, usage: &Node, arm: &Arm) -> Option<()> {
        let target = self.body.reference(&usage.id)?.target.as_ref()?;
        let local = usage.original_kind == "expr_path"
            && target.kind == TargetKind::Symbol
            && target.id.as_deref().map(hir_ref).as_ref() == Some(&arm.binding);
        local.then_some(())
    }

    /// A call to a trait member the lowering makes, with the implementation selection
    /// finds and the signature it is called at.
    fn propagation_call(&self, call: &'a Node, argument: &Node) -> Option<Selection> {
        let callee = self.body.child(call, 0).ok()?;
        let method = self.target_definition(callee, TargetKind::Definition)?;
        let reference = self.body.reference(&call.id)?;
        let arguments = reference.instance_arguments.as_deref().unwrap_or_default();
        let count = method
            .extensions
            .get("rust:compiler-identity")
            .and_then(|identity| identity.get("generics")?.get("count")?.as_u64())?;
        if identity(method, "def_kind")? != "AssocFn" || arguments.len() as u64 != count {
            return None;
        }
        let implementation = self
            .reader
            .tables
            .definition(reference.selected.as_ref()?.id.as_deref()?)
            .ok()?;
        let implementation_arguments = reference.selected_arguments.as_deref().unwrap_or_default();

        Some(Selection {
            method: self.library_record(method)?,
            implementation: self.library_record(implementation)?,
            arguments: self.spellings(arguments)?,
            implementation_arguments: self.spellings(implementation_arguments)?,
            input: self.spelling(argument.ty.as_deref()?)?,
            output: self.spelling(call.ty.as_deref()?)?,
        })
    }

    /// `From::from` of an error to its own type, as the unit records the selection.
    fn propagation_conversion(&self, error: &str) -> Option<Selection> {
        let conversion = self
            .reader
            .unit
            .extensions
            .as_ref()?
            .get("rust:reflexive-conversion")?;
        let definition = |key: &str| {
            self.reader
                .tables
                .definition(conversion.get(key)?.as_str()?)
                .ok()
        };
        let error = self.spelling(error)?;
        Some(Selection {
            method: self.library_record(definition("method")?)?,
            implementation: self.library_record(definition("implementation")?)?,
            arguments: vec![error.clone(), error.clone()],
            implementation_arguments: vec![error.clone()],
            input: error.clone(),
            output: error,
        })
    }

    fn spelling(&self, ty: &str) -> Option<String> {
        Some(self.reader.ty(ty).ok()?.spelling.clone())
    }

    fn spellings(&self, types: &[String]) -> Option<Vec<String>> {
        types.iter().map(|ty| self.spelling(ty)).collect()
    }
}

/// A call's one argument.
fn single_argument<'a>(context: &Context<'_, 'a>, call: &'a Node) -> Option<&'a Node> {
    if call.original_kind != "expr_call" {
        return None;
    }
    match context.body.children(call).ok()?.as_slice() {
        [_, argument] => Some(argument),
        _ => None,
    }
}
