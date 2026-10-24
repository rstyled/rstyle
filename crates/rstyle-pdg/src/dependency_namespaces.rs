//! Captured namespace observations grant no source or callable authority.
pub mod capture;
pub mod proof;

use crate::HirFnFacts;
use std::sync::Arc;

pub const CAPABILITY: &str = "rust.checked-anonymous-dependency-namespaces";
pub const GENERATED_CAPABILITY: &str = "rust.checked-generated-dependency-namespaces";

/// Retain observations without granting source, callable or dependency authority.
pub fn function_capture(facts: &HirFnFacts) -> Result<Option<Arc<capture::UnitCapture>>, String> {
    let options = &facts.build.frontend_options;
    if !options.contains_key(capture::KEY) {
        return Ok(None);
    }

    let record = options.data::<Arc<capture::UnitCapture>>()?;
    if record.identity != capture::IDENTITY {
        return Err("Unsupported owning dependency namespace capture identity".into());
    }
    Ok(Some(Arc::clone(record.as_ref())))
}

/// Exactly one record belongs to an owning unit, including empty observations.
/// Duplicate identical records are still an invalid unit inventory.
pub fn unit_capture(functions: &[HirFnFacts]) -> Result<Option<Arc<capture::UnitCapture>>, String> {
    crate::frontend::unit_records::single(
        functions,
        function_capture,
        "Duplicate owning dependency namespace capture",
    )
}
