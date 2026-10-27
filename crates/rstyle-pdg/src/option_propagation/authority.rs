//! Exact builtin Option residual conversion; no general Try permission.
use super::capture::{Shape, Witness};
use crate::result_propagation::authority::Protocol;
use crate::{HirNode, unit_store::EvidenceFacts};

pub(super) fn verify(
    witness: &Witness,
    operand: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned Option propagation compiler/target")?;
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "Option propagation")?;
    let protocol = Protocol {
        registry,
        prefix: "rstyle.option-propagation",
    };
    protocol.sources(SOURCE_DIGESTS)?;

    let shape = &witness.shape;
    if operand.ty.as_ref().map(|ty| ty.observed.as_str()) != Some(shape.input.as_str())
        || facts.signature.return_ty.observed != shape.output
        || shape.payload != "&u8"
    {
        return Err(
            "Option propagation substitutions disagree with complete type observations".into(),
        );
    }
    concrete_shape(shape)?;
    protocol.selection(&witness.branch, "try-branch", "::branch")?;
    protocol.selection(&witness.residual, "from-residual", "::from_residual")?;
    protocol.variant(&witness.break_arm.variant, "Break")?;
    protocol.variant(&witness.continue_arm.variant, "Continue")?;
    signatures(witness)
}

pub(super) fn signatures(witness: &Witness) -> Result<(), String> {
    let shape = &witness.shape;
    let branch = &witness.branch;
    let residual = &witness.residual;
    if branch.arguments != [shape.input.as_str()]
        || branch.implementation_arguments != [shape.payload.as_str()]
        || branch.input != shape.input
        || branch.output != shape.control_flow
        || residual.arguments != [shape.output.as_str(), shape.residual.as_str()]
        || residual.implementation_arguments != [shape.output_payload.as_str()]
        || residual.input != shape.residual
        || residual.output != shape.output
    {
        return Err("Option propagation lacks exact input/output residual substitutions".into());
    }

    Ok(())
}

pub(super) fn concrete_shape(shape: &Shape) -> Result<(), String> {
    let option = ["core::option::Option", "std::option::Option"];
    let control = ["core::ops::ControlFlow", "std::ops::ControlFlow"];
    if !option
        .iter()
        .any(|path| shape.input == format!("{path}<{}>", shape.payload))
        || !option
            .iter()
            .any(|path| shape.output == format!("{path}<{}>", shape.output_payload))
        || !option
            .iter()
            .any(|path| shape.residual == format!("{path}<!>"))
        || !control.iter().any(|path| {
            shape.control_flow == format!("{path}<{}, {}>", shape.residual, shape.payload)
        })
    {
        return Err(
            "Option propagation requires exact Option<!> residual and shared success".into(),
        );
    }
    Ok(())
}

const SOURCE_DIGESTS: &[(&str, &str)] = &[
    (
        "option.rs",
        "sha256:693df7245cf3e669b2c442b62c96c76c43ad1f9cfe79f6bf4306e71c32fa9f5d",
    ),
    (
        "ops/try_trait.rs",
        "sha256:5ccb79ee2c633935620932253924f4b8b09d71905db4f5c117a4a25cf3b7cd00",
    ),
    (
        "ops/control_flow.rs",
        "sha256:503a4cbd71a8825e62538a976dc8d81ad1c86c8d5203b42be3749dd6d29c9be2",
    ),
];
