//! Reuse the one checked member proof only with its exact immutable inputs.
use super::*;
use serde_json::Value;

pub(super) struct Binding {
    signature: crate::SignatureFacts,
    interface: Value,
    member: Value,
    compiled: Value,
}

impl Binding {
    pub(super) fn capture(facts: &HirFnFacts) -> Result<Self, String> {
        Ok(Self {
            signature: facts.signature.clone(),
            interface: observation(facts, concrete_types::KEY)?.clone(),
            member: observation(facts, crate::tryfrom_calls::KEY)?.clone(),
            compiled: observation(facts, "rstyle.rust.compiled-crate")?.clone(),
        })
    }

    pub(super) fn verify(&self, facts: &HirFnFacts) -> Result<(), String> {
        if self.signature != facts.signature
            || &self.interface != observation(facts, concrete_types::KEY)?
            || &self.member != observation(facts, crate::tryfrom_calls::KEY)?
            || &self.compiled != observation(facts, "rstyle.rust.compiled-crate")?
        {
            return Err(
                "Copy conversion member receipt belongs to another exact observation/signature"
                    .into(),
            );
        }
        Ok(())
    }
}

fn observation<'a>(facts: &'a HirFnFacts, key: &str) -> Result<&'a Value, String> {
    facts
        .build
        .frontend_options
        .get(key)
        .ok_or_else(|| format!("Copy conversion retained observation missing: {key}"))
}
