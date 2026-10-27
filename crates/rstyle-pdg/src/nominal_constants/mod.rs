//! Bounded product constants; captured observations never supply their own authority.
mod authority;
mod constructor;
mod initializer;
mod model;
mod source;
#[cfg(test)]
mod tests;

use crate::{HirNode, PathRes};
pub use authority::{Authority, BuildContext, SourceSelection, SourceUnit};
pub use model::{
    Constructor, CoreConstant, IDENTITY, Initializer, ProductConstant, RESOLUTION, ScalarValue,
    SourceSite,
};

pub struct VerifiedValue {
    pub owner_type: String,
    pub field: ScalarValue,
}

pub fn decode(resolution: &PathRes) -> Result<ProductConstant, String> {
    crate::scalar_constants::decode_record(resolution, RESOLUTION)
}

/// The caller supplies mandatory source/build/definition authority, including the actual node.
pub fn verify(node: &HirNode, authority: &dyn Authority) -> Result<VerifiedValue, String> {
    if node.kind != "expr_path" || node.expansion.is_some() || !node.children.is_empty() {
        return Err("Product constant requires an original expression path".into());
    }
    let record = decode(
        node.res
            .as_ref()
            .ok_or("Missing product constant resolution")?,
    )?;
    let owner_type = crate::data_types::token(&record.ty)?;
    if node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(owner_type.as_str()) {
        return Err("Product constant differs from its actual node type".into());
    }

    authority.actual(node, &record)?;
    verify_record(&record, authority)?;
    Ok(VerifiedValue {
        owner_type,
        field: record.field,
    })
}

fn verify_record(record: &ProductConstant, authority: &dyn Authority) -> Result<(), String> {
    model::shape(record)?;
    let build = authority.build()?;
    if !matches!(build.pointer_width, 32 | 64)
        || build
            .cfgs
            .iter()
            .filter(|cfg| cfg.starts_with("target_pointer_width="))
            .count()
            != 1
        || !build
            .cfgs
            .contains(&format!("target_pointer_width=\"{}\"", build.pointer_width))
    {
        return Err("Product constant requires a reviewed 32/64-bit build".into());
    }
    let sources = authority.sources(record)?;
    source::verify(record, &sources, &build)?;
    constructor::verify(record, &sources, &build)?;
    initializer::verify(record, &sources)?;

    authority.local(&record.definition, &record.declaration)?;
    authority.local(&record.implementation, &record.implementation_source)?;
    authority.local(
        &record.constructor.definition,
        &record.constructor.declaration,
    )?;
    authority.local(
        &record.constructor.implementation,
        &record.constructor.implementation_source,
    )?;
    authority.bindings(record, &sources)?;
    authority.owned_copy(record)?;
    authority.core_constant(&record.initializer.core_constant)?;
    Ok(())
}
