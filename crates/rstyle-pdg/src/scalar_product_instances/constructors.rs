//! Exact binary tuple-constructor evidence; no Copy, cleanup or field-read authority.
mod identity;
mod schema;
mod selection;
mod source;

use super::Observation as Instance;
use crate::{HirNode, nominal_boundary::resolution::Resolution, unit_store::EvidenceFacts};
pub use schema::{Callee, Declaration, Field, GenericArgument, Observation, Signature, ValueType};

pub const IDENTITY: &str = "rstyle.rust.scalar-product-constructor/1";
pub const RESOLUTION: &str = "observed_scalar_product_constructor";

pub(crate) struct Context<'a> {
    pub resolution: &'a Resolution,
    pub instance: &'a Instance,
}

pub(crate) struct CheckedConstructor<'a> {
    node: &'a HirNode,
    fields: [&'a HirNode; 2],
}

impl<'a> CheckedConstructor<'a> {
    pub(crate) fn node(&self) -> &'a HirNode {
        self.node
    }
    pub(crate) fn fields(&self) -> [&'a HirNode; 2] {
        self.fields
    }
}

pub(crate) fn verify<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
    context: &Context<'_>,
) -> Result<CheckedConstructor<'a>, String> {
    context.instance.check_shape()?;
    let observation = Observation::decode(node)?;
    identity::verify(&observation, facts, context.instance)?;
    selection::verify(&observation, facts, context.instance)?;

    let fields = source::verify(node, facts, context, &observation)?;
    Ok(CheckedConstructor { node, fields })
}
