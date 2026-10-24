//! Bounded original product constants; observations never grant admission.
mod constructor;
mod evaluate;
mod identity;
mod initializer;

use super::hir::Context;
use rstyle_pdg::{HirNode, PathRes, concrete_types::TypeTree, nominal_constants::ProductConstant};
use rustc_hir::{
    ExprKind, Node,
    def::{DefKind, Res},
    def_id::DefId,
};
use rustc_middle::ty;

#[derive(Debug)]
pub(super) enum CaptureFailure {
    Unsupported(&'static str),
    Evaluation(String),
}

impl std::fmt::Display for CaptureFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(reason) => {
                write!(formatter, "Unsupported product constant: {reason}")
            }
            Self::Evaluation(reason) => {
                write!(formatter, "Product constant evaluation failed: {reason}")
            }
        }
    }
}

/// Re-resolve the actual expression instead of trusting its serialized path text.
pub(super) fn selected(context: &Context<'_>, node: &HirNode) -> Option<DefId> {
    if node.kind != "expr_path" {
        return None;
    }
    let id = rustc_hir::HirId {
        owner: rustc_hir::OwnerId {
            def_id: rustc_hir::def_id::LocalDefId {
                local_def_index: rustc_hir::def_id::DefIndex::from_u32(node.hir.owner),
            },
        },
        local_id: rustc_hir::ItemLocalId::from_u32(node.hir.local),
    };
    let Node::Expr(expression) = context.tcx.hir_node(id) else {
        return None;
    };
    let ExprKind::Path(ref path) = expression.kind else {
        return None;
    };

    let Res::Def(DefKind::AssocConst, definition) = context.typeck.qpath_res(path, id) else {
        return None;
    };
    (candidate(context, definition)
        && context.typeck.node_type_opt(id)
            == Some(
                context
                    .tcx
                    .type_of(definition)
                    .instantiate_identity()
                    .skip_norm_wip(),
            ))
    .then_some(definition)
}

pub(super) fn resolution(
    context: &Context<'_>,
    definition: DefId,
    owner: TypeTree,
    node: &HirNode,
) -> Result<
    (
        PathRes,
        Option<rstyle_pdg::product_constant_values::SelectedUse>,
    ),
    String,
> {
    match capture(context, definition, owner) {
        Ok(record) => {
            let selected =
                selection(context, definition, node, &record).map_err(|error| error.to_string())?;
            Ok((
                PathRes {
                    kind: rstyle_pdg::nominal_constants::RESOLUTION.into(),
                    target: None,
                    description: serde_json::to_string(&record)
                        .map_err(|error| error.to_string())?,
                },
                Some(selected),
            ))
        }
        Err(failure) => Ok((
            PathRes {
                kind: match &failure {
                    CaptureFailure::Unsupported(_) => "unsupported_product_constant",
                    CaptureFailure::Evaluation(_) => "failed_product_constant_evaluation",
                }
                .into(),
                target: None,
                description: failure.to_string(),
            },
            None,
        )),
    }
}

fn selection(
    context: &Context<'_>,
    definition: DefId,
    node: &HirNode,
    record: &ProductConstant,
) -> Result<rstyle_pdg::product_constant_values::SelectedUse, CaptureFailure> {
    use rstyle_pdg::product_constant_values::{Definition, SelectedUse};
    let owner = context
        .tcx
        .type_of(definition)
        .instantiate_identity()
        .skip_norm_wip();
    let ty::Adt(owner, _) = owner.kind() else {
        return Err(CaptureFailure::Unsupported(
            "missing selected product owner",
        ));
    };
    let declaration = identity::declaration(context, owner.did())?;

    Ok(SelectedUse {
        hir: node.hir.clone(),
        span: node.span,
        constant: Definition {
            definition: identity::local(context, definition)?,
            source_file: record.declaration.file.clone(),
            source_span: record.declaration.span,
        },
        implementation: Definition {
            definition: identity::local(context, context.tcx.parent(definition))?,
            source_file: record.implementation_source.file.clone(),
            source_span: record.implementation_source.span,
        },
        tuple_constructor: Definition {
            definition: identity::local(
                context,
                owner
                    .non_enum_variant()
                    .ctor_def_id()
                    .ok_or(CaptureFailure::Unsupported(
                        "missing selected tuple constructor",
                    ))?,
            )?,
            source_file: declaration.file,
            source_span: declaration.span,
        },
    })
}

pub(super) fn unsupported_resolution(reason: String) -> PathRes {
    PathRes {
        kind: "unsupported_product_constant".into(),
        target: None,
        description: reason,
    }
}

/// Cheap selection before the parent performs full concrete-type observation.
pub(super) fn candidate(context: &Context<'_>, definition: DefId) -> bool {
    let tcx = context.tcx;
    if !definition.is_local()
        || tcx.def_kind(definition) != DefKind::AssocConst
        || tcx.generics_of(definition).count() != 0
        || !matches!(
            tcx.def_kind(tcx.parent(definition)),
            DefKind::Impl { of_trait: false }
        )
    {
        return false;
    }
    matches!(tcx.type_of(definition).instantiate_identity().skip_norm_wip().kind(),
        ty::Adt(owner, arguments) if owner.is_struct() && owner.did().is_local() && arguments.is_empty())
}

pub(super) fn capture(
    context: &Context<'_>,
    definition: DefId,
    owner: TypeTree,
) -> Result<ProductConstant, CaptureFailure> {
    if !candidate(context, definition) {
        return Err(CaptureFailure::Unsupported(
            "not a local concrete inherent product constant",
        ));
    }
    let ty = evaluate::owner(context, definition, &owner)?;
    let declaration = identity::declaration(context, definition)?;
    let (implementation, implementation_source) = identity::implementation(context, definition)?;
    let (initializer, selected) = initializer::capture(context, definition, ty)?;
    let constructor = constructor::capture(context, selected, ty, &owner)?;
    let field = evaluate::field(context, definition, ty)?;

    Ok(ProductConstant {
        identity: rstyle_pdg::nominal_constants::IDENTITY.into(),
        definition: identity::local(context, definition)?,
        implementation: identity::local(context, implementation)?,
        declaration,
        implementation_source,
        ty: owner,
        field,
        initializer,
        constructor,
    })
}
