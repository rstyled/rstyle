//! Exact pinned slice metadata semantics and complete concrete receiver transitions.
use super::{
    Result,
    body::{Role, Roles},
    receipts,
    transport::*,
    types::{self, Types},
};
use crate::{
    HirFnFacts,
    concrete_types::{Observation, TypeTree},
    data_receivers::{Capture, Kind},
};

pub(super) fn verify(
    original: &HirFnFacts,
    projection: &Projection,
    roles: &Roles,
    types: &Types,
) -> Result<Capture> {
    let capture = original.build.frontend_options.data::<Capture>()?;
    let ([before], [method]) = (capture.methods.as_slice(), projection.methods.as_slice()) else {
        return Err("Slice instance requires its complete single-method roster".into());
    };
    if capture.identity != crate::data_receivers::IDENTITY || *before != method.original {
        return Err("Slice instance changes original method observations".into());
    }
    let node =
        receipts::find(&original.body, &roles.length).ok_or("Missing checked slice length site")?;
    let [receiver] = node.children.as_slice() else {
        return Err("Slice length receiver roster differs".into());
    };
    let record = &method.concrete;
    if record.call != node.hir
        || before.call != node.hir
        || record.receiver != receiver.hir
        || before.receiver != receiver.hir
        || record.source_span != node.span
        || before.source_span != node.span
    {
        return Err("Slice instance method record loses its exact source/HIR site".into());
    }
    authority(original, node)?;
    types.endpoint(&record.receiver_type, Role::Bytes)?;
    types.endpoint(&record.adjusted_type, Role::Bytes)?;
    types.endpoint(&record.signature_result, Role::Usize)?;
    let [input] = record.signature_inputs.as_slice() else {
        return Err("Slice length signature input roster differs".into());
    };
    types.endpoint(input, Role::Bytes)?;
    adjustments(method, types)?;

    Ok(Capture {
        identity: capture.identity.clone(),
        methods: vec![materialize(record, types)?],
    })
}

fn authority(original: &HirFnFacts, node: &crate::HirNode) -> Result<()> {
    let registry = crate::data_library::verify_64bit_sysroot(original, "Slice length instance")?;
    let operation = registry
        .operations
        .iter()
        .find(|op| op.id == "rstyle.data.slice.len")
        .ok_or("Missing pinned slice length contract")?;
    let callee = crate::data_library::callee(node)?;
    let libraries: crate::data_library::Libraries =
        crate::data_frontend::evidence(original, crate::data_library::KEY)?;

    if libraries.identity != crate::data_library::IDENTITY
        || libraries.libraries.len() != 1
        || libraries.libraries[0].crate_name != "core"
        || libraries.libraries[0].crate_hash != registry.core_hash
        || callee.path != operation.path
        || callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || callee.definition_hash != operation.definition_hash
        || callee.implementation.is_some()
        || operation.referent != "slice"
        || operation.result != "usize"
    {
        return Err("Slice length changes its exact pinned inherent metadata operation".into());
    }

    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing original slice length dispatch")?;
    if dispatch.kind != "resolved_method"
        || dispatch.op != operation.path
        || dispatch.result_ty != "usize"
        || dispatch.operand_tys.len() != 1
    {
        return Err("Slice length original dispatch differs".into());
    }

    let call = receipts::inventory_call(original, &node.hir)?;
    if call["hir"] != serde_json::to_value(&node.hir).map_err(|e| e.to_string())?
        || call["expanded"] != false
        || call["status"] != "unresolved"
        || call["requested"]["definition_identity"] != callee.definition_hash
        || call["requested"]["path"] != callee.path
        || call["requested"]["crate"] != "core"
        || call["selected"].is_object()
    {
        return Err("Slice length loses its exact original generic call receipt".into());
    }
    Ok(())
}

fn adjustments(method: &Method, types: &Types) -> Result<()> {
    let ([dereference, borrow], [before_deref, before_borrow]) = (
        method.concrete.adjustments.as_slice(),
        method.original.adjustments.as_slice(),
    ) else {
        return Err(
            "Slice length requires complete original dereference/shared-borrow adjustments".into(),
        );
    };
    if dereference.kind != Kind::BuiltinDereference
        || before_deref.kind != dereference.kind
        || borrow.kind != Kind::SharedBorrow
        || before_borrow.kind != borrow.kind
        || *types::identity(&dereference.target)?
            != (Type::Slice {
                element: Box::new(types::builtin("u8")),
            })
    {
        return Err("Slice length changes adjustment order, kinds or targets".into());
    }
    types.endpoint(&borrow.target, Role::Bytes)
}

fn materialize(record: &ConcreteMethod, types: &Types) -> Result<crate::data_receivers::Method> {
    let shared = Observation::Captured {
        ty: types
            .tree(Role::Bytes)
            .ok_or("Missing concrete byte slice")?,
    };
    Ok(crate::data_receivers::Method {
        call: record.call.clone(),
        source_span: record.source_span,
        receiver: record.receiver.clone(),
        receiver_type: shared.clone(),
        adjusted_type: shared.clone(),
        adjustments: vec![
            crate::data_receivers::Adjustment {
                kind: Kind::BuiltinDereference,
                target: Observation::Captured {
                    ty: TypeTree::Slice(Box::new(TypeTree::Scalar("u8".into()))),
                },
            },
            crate::data_receivers::Adjustment {
                kind: Kind::SharedBorrow,
                target: shared.clone(),
            },
        ],
        signature_inputs: vec![shared],
        signature_result: Observation::Captured {
            ty: TypeTree::Scalar("usize".into()),
        },
    })
}
