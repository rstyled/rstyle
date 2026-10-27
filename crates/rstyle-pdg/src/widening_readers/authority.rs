//! Exact selected readers, error owners and core methods precede callback flow.
use super::roles::Roles;
use crate::{
    HirNode,
    concrete_types::{InterfaceObservations, Observation, TypeTree},
    copy_tagged_values::origins::WireContract,
    unit_store::EvidenceFacts,
};
use serde_json::Value;

/// Completed source and selected-dependency checks for one widening reader.
#[derive(Clone)]
pub struct Proof {
    pub(crate) input: TypeTree,
    pub(crate) output: TypeTree,
    pub(crate) intermediate: TypeTree,
    pub(crate) storage: [TypeTree; 3],
    pub(crate) reader_contract: WireContract,
    pub(crate) error_contract: WireContract,
    pub(crate) reader_dependency: Value,
    pub(crate) error_dependency: Value,
}

pub(super) fn verify(roles: &Roles<'_>, facts: &dyn EvidenceFacts) -> Result<Proof, String> {
    let output = output(facts)?;

    let TypeTree::Result { err, .. } = &output else {
        unreachable!()
    };

    let input = TypeTree::Result {
        ok: Box::new(tuple(roles.width.scalar())),
        err: err.clone(),
    };

    let intermediate = super::intermediate(roles.width);

    let (reader_contract, reader_dependency) = reader(roles, &input, facts)?;

    let (error_contract, error_dependency) = error(roles, err, facts)?;

    let mut proof = Proof {
        input,
        output,
        intermediate,
        storage: std::array::from_fn(|_| TypeTree::Tuple(vec![])),
        reader_contract,
        error_contract,
        reader_dependency,
        error_dependency,
    };
    methods(roles, &proof, facts)?;
    super::conversion(roles, facts)?;
    proof.storage = super::environments::verify(roles, &proof, facts)?;

    Ok(proof)
}

fn output(facts: &dyn EvidenceFacts) -> Result<TypeTree, String> {
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    interface.validate()?;

    let Observation::Captured { ty } = &interface.result else {
        return Err("Widening reader requires its complete observed Result".into());
    };

    let TypeTree::Result { ok, err } = ty else {
        return Err("Widening reader output must be a concrete Result".into());
    };

    let TypeTree::Struct { nominal, .. } = err.as_ref() else {
        return Err("Widening reader requires its original nominal error".into());
    };

    if ok.as_ref() != &tuple("usize")
        || !facts.nominal_resolution()?.checked_no_drop(nominal)
        || facts.signature.return_ty.normalized.as_deref()
            != Some(crate::data_types::token(ty)?.as_str())
    {
        return Err(
            "Widening reader changes its tuple or independently proved error cleanup".into(),
        );
    }

    Ok(ty.clone())
}

fn reader(
    roles: &Roles<'_>,
    input: &TypeTree,
    facts: &dyn EvidenceFacts,
) -> Result<(WireContract, Value), String> {
    let [slice, message] = roles.reader.children.as_slice() else {
        return Err("Widening reader changes its whole checked-reader argument list".into());
    };
    crate::source_evidence::verify_binding_read(slice, &roles.slice.pat, "Changed reader slice")?;
    crate::source_evidence::verify_binding_read(
        message,
        &roles.message.pat,
        "Changed reader message",
    )?;

    let dependency = crate::helper_calls::verify_use(roles.reader, facts)?;

    let contract = crate::helper_calls::guarded_effects::validate_shape(dependency.summary())?;

    let crate::helper_calls::VerifiedDependency::Shared(view) = &dependency else {
        return Err("Widening reader requires its complete selected original reader owner".into());
    };

    let callee = view.frontend();

    if callee.name.rsplit("::").next()
        != Some(if roles.width == crate::native_reader_width::Width::U16 {
            "try_read_u16"
        } else {
            "try_read_u32"
        })
        || !crate::data_documents::fallible_readers::verify_source(&callee)?
        || dependency.summary().argument_types
            != [
                crate::data_aggregates::token(slice)?,
                crate::data_aggregates::token(message)?,
            ]
        || dependency.summary().result_type != crate::data_types::token(input)?
        || crate::data_aggregates::token(roles.reader)? != dependency.summary().result_type
    {
        return Err("Widening reader changes its complete proved four-byte native reader".into());
    }

    Ok((contract.clone(), dependency.wire()?))
}

fn error(
    roles: &Roles<'_>,
    error: &TypeTree,
    facts: &dyn EvidenceFacts,
) -> Result<(WireContract, Value), String> {
    let dependency = crate::helper_calls::verify_use(roles.constructor, facts)?;

    let contract = crate::helper_calls::named_errors::validate_shape(dependency.summary())?;

    let crate::helper_calls::VerifiedDependency::Shared(view) = &dependency else {
        return Err("Widening failure requires its complete selected original constructor".into());
    };

    let callee = view.frontend();

    let constructor = crate::named_error_payloads::verify_function(&callee)?;

    if dependency.summary().identity != crate::helper_calls::named_errors::SUMMARY
        || callee.name.rsplit("::").next() != Some("invalid_usize")
        || !matches!(constructor.variant,
            crate::named_error_payloads::Constructor::Named { variant, .. }
                if variant == "InvalidUsize")
        || dependency.summary().argument_types
            != [crate::data_aggregates::token(roles.message_read)?]
        || dependency.summary().result_type != crate::data_types::token(error)?
        || crate::data_aggregates::token(roles.constructor)? != dependency.summary().result_type
        || crate::named_error_payloads::error_tree(&callee)?.as_ref() != Some(error)
    {
        return Err(
            "Widening failure changes its proved InvalidUsize owner or static route".into(),
        );
    }

    Ok((contract.clone(), dependency.wire()?))
}

fn methods(roles: &Roles<'_>, proof: &Proof, facts: &dyn EvidenceFacts) -> Result<(), String> {
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "widening reader")?;

    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned widening compiler/target")?;

    if registry.source_digests.get("result.rs").map(String::as_str)
        != Some("sha256:777e0f516deba3899c8f16c5511da3ebe7debe23f4bcf6a6353c98a927c0f7d1")
    {
        return Err("Widening reader loses its reviewed complete Result source".into());
    }

    let capture = crate::data_receivers::verify(facts)?;

    let conversion = super::conversion_type(roles.width);

    for (node, input, output, paths) in [
        (roles.source, &proof.input, &proof.output, AND_THEN),
        (roles.map, &conversion, &proof.intermediate, MAP),
        (roles.map_err, &proof.intermediate, &proof.output, MAP_ERR),
    ] {
        method(node, paths, facts)?;
        let record = capture
            .methods
            .iter()
            .find(|record| record.call == node.hir)
            .ok_or("Missing widening method receiver/signature record")?;
        signature(record, input, output)?;
    }

    Ok(())
}

const AND_THEN: &[&str] = &[
    "std::result::Result::<T, E>::and_then",
    "core::result::Result::<T, E>::and_then",
];
const MAP: &[&str] = &[
    "std::result::Result::<T, E>::map",
    "core::result::Result::<T, E>::map",
];
const MAP_ERR: &[&str] = &[
    "std::result::Result::<T, E>::map_err",
    "core::result::Result::<T, E>::map_err",
];

fn method(
    node: &HirNode,
    paths: &'static [&'static str],
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let callee = crate::data_library::callee(node)?;
    crate::data_library::verify_core_definition(
        &callee,
        facts,
        crate::data_library::CoreDefinition {
            paths,
            unavailable: "Unpinned widening method compiler/target",
            mismatch: "Widening method lacks its exact selected core definition",
        },
    )?;

    if node.binding.is_some()
        || node.lit.is_some()
        || node.is_unsafe_block
        || node.expansion.is_some()
    {
        return Err("Widening method changes its original ordinary invocation".into());
    }

    let observed = &node
        .ty
        .as_ref()
        .ok_or("Missing widening method output")?
        .observed;
    crate::data_receivers::verify_dispatch(node, &callee.path, &[observed])
}

fn signature(
    record: &crate::data_receivers::Method,
    input: &TypeTree,
    output: &TypeTree,
) -> Result<(), String> {
    let input = Observation::Captured { ty: input.clone() };

    if record.receiver_type != input
        || record.adjusted_type != input
        || !record.adjustments.is_empty()
        || record.signature_inputs.len() != 2
        || record.signature_inputs[0] != input
        || !matches!(&record.signature_inputs[1], Observation::Unsupported { reason }
            if reason == "Outside the concrete drop-free/shared type observation grammar")
        || record.signature_result != (Observation::Captured { ty: output.clone() })
    {
        return Err("Widening method changes its concrete by-value interface".into());
    }

    Ok(())
}

pub(super) fn tuple(first: &str) -> TypeTree {
    TypeTree::Tuple(vec![
        TypeTree::Scalar(first.into()),
        TypeTree::Scalar("usize".into()),
    ])
}
