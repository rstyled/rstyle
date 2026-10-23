//! Strict product constructor replay against an independently inspected owner.
use super::super::instances::RetainedConstructor;
use super::super::product_instances::ProductInstanceRecord;
use super::*;
use crate::product_instances::SpanArgument;
use serde_json::Value;

const SCHEMA: &str = "rstyle.product-constructor-sidecar/1";

#[cfg(test)]
mod tests;

#[derive(Serialize)]
pub(super) struct Export<'a> {
    schema: &'static str,
    owner_namespace: &'a UnitNamespace,
    definition_ordinal: usize,
    argument: &'static str,
    observation: &'a Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Import {
    schema: String,
    owner_namespace: UnitNamespace,
    definition_ordinal: usize,
    argument: String,
    observation: Value,
}

/// Replay accepts proof inputs, never serialized graphs or activation tokens.
/// Neither this session nor its result associates the constructor with a caller.
pub struct ProductConstructorReplay {
    owner: Arc<UnitStore>,
}

impl ProductConstructorReplay {
    pub fn new(
        owner: Arc<UnitStore>,
        inspect: impl FnOnce(&[HirFnFacts]) -> Result<(), String>,
    ) -> Result<Self, String> {
        Ok(Self {
            owner: super::instances::checked_original_owner(owner, inspect)?,
        })
    }

    pub fn owner(&self) -> &Arc<UnitStore> {
        &self.owner
    }

    /// Recompute the source, effects, origins and graph under strict policy.
    pub fn replay(&self, bytes: &[u8]) -> Result<UnitStore, String> {
        super::instances::decode_and_replay(bytes, |import| self.replay_inputs(import))
    }

    pub(super) fn replay_inputs(&self, import: Import) -> Result<UnitStore, String> {
        if import.schema != SCHEMA || import.owner_namespace != self.owner.namespace {
            return Err("Product sidecar differs from its independently selected owner".into());
        }
        let argument = parse_argument(&import.argument)?;

        crate::verification_policy::strict(|| {
            let store = UnitStore::capture_product_constructor(
                Arc::clone(&self.owner),
                import.definition_ordinal,
                &import.observation,
                argument,
            )?;
            store.validate_evidence()?;
            Ok(store)
        })
    }
}

fn parse_argument(value: &str) -> Result<SpanArgument, String> {
    match value {
        "Span" => Ok(SpanArgument::Span),
        "RangeUsize" => Ok(SpanArgument::RangeUsize),
        _ => Err("Product sidecar requires its exact concrete argument".into()),
    }
}

fn argument_name(argument: SpanArgument) -> &'static str {
    match argument {
        SpanArgument::Span => "Span",
        SpanArgument::RangeUsize => "RangeUsize",
    }
}

impl UnitStore {
    /// Export the exact retained proof inputs after independently rechecking them.
    pub fn write_product_constructor_sidecar(&self, writer: impl Write) -> Result<(), String> {
        super::instances::write_constructor::<ProductInstanceRecord>(self, writer)
    }
}

impl super::instances::ConstructorSidecar for ProductInstanceRecord {
    type Export<'a> = Export<'a>;

    fn export<'a>(input: super::super::instances::RetainedInput<'a, Self>) -> Export<'a> {
        Export {
            schema: SCHEMA,
            owner_namespace: &input.owner.namespace,
            definition_ordinal: input.ordinal,
            argument: argument_name(input.argument),
            observation: input.record.observation(),
        }
    }
}
