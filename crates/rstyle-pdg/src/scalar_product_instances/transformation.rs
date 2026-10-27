//! Exact paired instantiation, without aggregate ownership or graph permission.
use super::{Argument, Observation};
use crate::{concrete_types::TypeTree, nonzero_values, unit_store::EvidenceFacts};

/// The saved suffix type and original generic declaration agree in both instances.
/// Actual suffix values/origins, aggregate cleanup and callback guards still need proof.
pub struct Checked<'a> {
    input: &'a Observation,
    output: &'a Observation,
    payload: nonzero_values::CheckedType,
}

impl<'a> Checked<'a> {
    pub fn input(&self) -> &'a Observation {
        self.input
    }

    pub fn output(&self) -> &'a Observation {
        self.output
    }

    pub fn payload(&self) -> nonzero_values::CheckedType {
        self.payload
    }
}

pub fn verify<'a>(
    input: &'a Observation,
    output: &'a Observation,
    facts: &dyn EvidenceFacts,
) -> Result<Checked<'a>, String> {
    input.check_shape()?;
    output.check_shape()?;
    if input.nominal != output.nominal
        || input.source != output.source
        || input.fields[0] != output.fields[0]
        || input.fields[1].declaration != output.fields[1].declaration
        || !matches!(
            (input.argument, output.argument),
            (Argument::U8, Argument::NonZeroU8) | (Argument::U16, Argument::NonZeroU16)
        )
    {
        return Err("Nonzero product transformation changes its original declaration, saved suffix type or exact substitution".into());
    }
    let TypeTree::NonZero(observation) = &output.fields[1].ty else {
        return Err("Nonzero product transformation requires its exact core payload".into());
    };
    let payload = nonzero_values::verify(observation, facts)?;

    Ok(Checked {
        input,
        output,
        payload,
    })
}
