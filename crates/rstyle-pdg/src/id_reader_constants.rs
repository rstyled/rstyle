//! Bounded ID byte widths, retaining the generated-to-original initializer chain.
mod model;
pub(crate) mod source;
#[cfg(test)]
mod tests;

use crate::{HirNode, PathRes};
pub use model::{Annotation, IDENTITY, Layout, Observation, Site};

pub const CAPABILITY: &str = "rust.named-error-id-reader-flow";
pub const RESOLUTION: &str = "resolved_id_reader_size";
pub const ANNOTATION_RESOLUTION: &str = "resolved_id_reader_array_annotation";

/// Every obligation requires independently checked source/build authority.
pub trait Authority {
    fn actual(&self, node: &HirNode, record: &Observation) -> Result<(), String>;
    fn annotation(&self, node: &HirNode, record: &Annotation) -> Result<(), String>;
    fn chain(&self, record: &Observation) -> Result<(), String>;
}

pub fn decode(resolution: &PathRes) -> Result<Observation, String> {
    crate::scalar_constants::decode_record(resolution, RESOLUTION)
}

pub fn verify(node: &HirNode, authority: &dyn Authority) -> Result<u64, String> {
    if node.kind != "expr_path"
        || node.expansion.is_some()
        || !node.children.is_empty()
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("usize")
    {
        return Err("ID reader width requires an original usize expression path".into());
    }

    let record = decode(
        node.res
            .as_ref()
            .ok_or("Missing ID reader width observation")?,
    )?;
    model::shape(&record)?;
    authority.actual(node, &record)?;
    authority.chain(&record)?;
    Ok(record.value)
}

pub fn verify_annotation(node: &HirNode, authority: &dyn Authority) -> Result<u64, String> {
    if node.kind != "ty_annotation" || node.expansion.is_some() || !node.children.is_empty() {
        return Err("ID reader array width requires an original complete annotation".into());
    }
    let record: Annotation = crate::scalar_constants::decode_record(
        node.res
            .as_ref()
            .ok_or("Missing ID reader annotation observation")?,
        ANNOTATION_RESOLUTION,
    )?;
    if record.span != node.span
        || record.length.start < record.span.start
        || record.length.end > record.span.end
        || record.length.start >= record.length.end
    {
        return Err("ID reader annotation and length coordinates disagree".into());
    }

    model::shape(&record.constant)?;
    authority.annotation(node, &record)?;
    authority.chain(&record.constant)?;
    Ok(record.constant.value)
}
