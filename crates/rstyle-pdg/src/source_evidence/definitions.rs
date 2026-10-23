//! Bind a selected member to the exact captured function, without owner authority.
use crate::HirFnFacts;

pub(crate) fn require_definition(
    facts: &HirFnFacts,
    path: &str,
    identity: &str,
) -> Result<(), String> {
    if facts.name != path
        || facts
            .build
            .frontend_options
            .get("definition_identity")
            .and_then(serde_json::Value::as_str)
            != Some(identity)
    {
        return Err("Selected definition differs from exact captured function identity".into());
    }
    Ok(())
}
