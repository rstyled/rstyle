//! Independently pinned selected implementations and reflexive conversion.
use super::capture::{Selection, Witness};
use crate::{
    HirNode,
    data_library::{Callee, Registry},
    unit_store::EvidenceFacts,
};

pub(super) fn verify(
    witness: &Witness,
    operand: &HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let registry = crate::data_library::registry_for(facts)
        .ok_or("Unpinned Result propagation compiler/target")?;
    crate::data_library::verify_sysroot_roots(facts, &["core", "std"], "Result propagation")?;
    let protocol = Protocol {
        registry,
        prefix: "rstyle.result-propagation",
    };
    protocol.sources(SOURCE_DIGESTS)?;

    let shape = &witness.shape;
    if operand.ty.as_ref().map(|ty| ty.observed.as_str()) != Some(shape.result.as_str())
        || facts.signature.return_ty.observed != shape.result
    {
        return Err(
            "Propagation substitutions disagree with concrete input/output observations".into(),
        );
    }

    protocol.selection(&witness.branch, "try-branch", "::branch")?;
    protocol.selection(&witness.residual, "from-residual", "::from_residual")?;
    protocol.selection(&witness.conversion, "identity-from", "::from")?;

    protocol.variant(&witness.break_arm.variant, "Break")?;
    protocol.variant(&witness.continue_arm.variant, "Continue")?;
    concrete_shape(&witness.shape)?;
    signatures(witness)
}

pub(crate) struct Protocol<'a> {
    pub registry: &'a Registry,
    pub prefix: &'static str,
}

impl Protocol<'_> {
    pub fn sources(&self, sources: &[(&str, &str)]) -> Result<(), String> {
        for (path, digest) in sources {
            let digests = if *path == "result.rs" {
                &self.registry.source_digests
            } else {
                &self.registry.result_propagation_source_digests
            };
            if digests.get(*path).map(String::as_str) != Some(*digest) {
                return Err(format!(
                    "Propagation lacks reviewed core source authority for {path}"
                ));
            }
        }
        Ok(())
    }

    pub fn selection(&self, selection: &Selection, role: &str, suffix: &str) -> Result<(), String> {
        let method_key = format!("{}.{role}.method", self.prefix);
        let implementation_key = format!("{}.{role}.implementation", self.prefix);
        if !self
            .registry
            .exact_core_definition(&selection.method, &method_key)
            || !self
                .registry
                .exact_core_definition(&selection.implementation, &implementation_key)
            || !selection.method.path.ends_with(suffix)
            || !selection.implementation.path.ends_with(suffix)
            || selection.method == selection.implementation
        {
            return Err(format!(
                "Propagation lacks its exact selected core {role} implementation"
            ));
        }
        Ok(())
    }

    pub fn variant(&self, variant: &Callee, name: &str) -> Result<(), String> {
        let key = format!("{}.ControlFlow::{name}", self.prefix);
        if !self.registry.exact_core_definition(variant, &key)
            || !variant.path.ends_with(&format!("::ControlFlow::{name}"))
        {
            return Err("Propagation continuation lacks its pinned ControlFlow variant".into());
        }
        Ok(())
    }
}

fn signatures(witness: &Witness) -> Result<(), String> {
    let shape = &witness.shape;
    let branch = &witness.branch;
    let residual = &witness.residual;
    let conversion = &witness.conversion;

    if branch.arguments != [shape.result.as_str()]
        || branch.implementation_arguments != ["()", &shape.error]
        || branch.input != shape.result
        || branch.output != shape.control_flow
        || residual.arguments != [shape.result.as_str(), shape.residual.as_str()]
        || residual.implementation_arguments != ["()", &shape.error, &shape.error]
        || residual.input != shape.residual
        || residual.output != shape.result
        || conversion.arguments != [shape.error.as_str(), shape.error.as_str()]
        || conversion.implementation_arguments != [shape.error.as_str()]
        || conversion.input != shape.error
        || conversion.output != shape.error
    {
        return Err(
            "Propagation substitutions/signatures do not prove exact same-error identity".into(),
        );
    }
    Ok(())
}

const SOURCE_DIGESTS: &[(&str, &str)] = &[
    (
        "result.rs",
        "sha256:777e0f516deba3899c8f16c5511da3ebe7debe23f4bcf6a6353c98a927c0f7d1",
    ),
    (
        "ops/try_trait.rs",
        "sha256:5ccb79ee2c633935620932253924f4b8b09d71905db4f5c117a4a25cf3b7cd00",
    ),
    (
        "ops/control_flow.rs",
        "sha256:503a4cbd71a8825e62538a976dc8d81ad1c86c8d5203b42be3749dd6d29c9be2",
    ),
    (
        "convert/mod.rs",
        "sha256:ec1dfed5b8eef22496fc85eeb0b26a5600944fb8d2f6d6e9aaf81fb45af06de5",
    ),
];

fn concrete_shape(shape: &super::capture::Shape) -> Result<(), String> {
    let result = ["core::result::Result", "std::result::Result"];
    let control = ["core::ops::ControlFlow", "std::ops::ControlFlow"];
    if !result
        .iter()
        .any(|path| shape.result == format!("{path}<(), {}>", shape.error))
        || !result
            .iter()
            .any(|path| shape.residual == format!("{path}<!, {}>", shape.error))
        || !control.iter().any(|path| {
            shape.control_flow == format!("{path}<{}>", shape.residual)
                || shape.control_flow == format!("{path}<{}, ()>", shape.residual)
        })
    {
        return Err(
            "Propagation residual/continuation is not exact Result<!, E>/unit ControlFlow".into(),
        );
    }
    Ok(())
}
