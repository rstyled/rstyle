//! Exact selected-unit association records, before dependency linking.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

pub const UNIT_KEY: &str = "inherent_unit_definitions";
pub const UNIT_IDENTITY: &str = "rstyle.helper.inherent-unit/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitDefinitions {
    pub identity: String,
    pub definitions: Vec<Association>,
}

/// Association identity only; concrete field trees remain in the independently
/// verified callee interface/definition, never erased from matching interfaces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Association {
    pub definition_identity: String,
    pub implementation_identity: String,
    pub implementation_span: ByteSpan,
    pub owner_nominal: crate::concrete_types::Nominal,
    pub source_file: String,
    pub function_span: ByteSpan,
}

impl From<Definition> for Association {
    fn from(record: Definition) -> Self {
        Self {
            definition_identity: record.definition_identity,
            implementation_identity: record.implementation_identity,
            implementation_span: record.implementation_span,
            owner_nominal: record.owner_nominal,
            source_file: record.source_file,
            function_span: record.function_span,
        }
    }
}

impl UnitDefinitions {
    pub fn validate(&self) -> Result<(), String> {
        if self.identity != UNIT_IDENTITY
            || self.definitions.len() > crate::concrete_types::MAX_ENTRIES
            || serde_json::to_vec(self)
                .map_err(|error| error.to_string())?
                .len()
                > 8 * 1024 * 1024
        {
            return Err(
                "Inherent selected-unit identity or explicit record/byte budget disagrees".into(),
            );
        }

        let sources: BTreeSet<_> = self
            .definitions
            .iter()
            .map(|record| {
                (
                    &record.source_file,
                    record.function_span.start,
                    record.function_span.end,
                )
            })
            .collect();
        if sources.len() != self.definitions.len() {
            return Err("Ambiguous inherent source-definition association".into());
        }

        let mut definitions = BTreeSet::new();
        let mut implementations = BTreeMap::new();
        for record in &self.definitions {
            if !definitions.insert(&record.definition_identity) {
                return Err("Duplicate inherent selected-unit definition".into());
            }
            let implementation = (
                &record.owner_nominal,
                &record.source_file,
                record.implementation_span,
            );
            if implementations
                .insert(&record.implementation_identity, implementation)
                .is_some_and(|previous| previous != implementation)
            {
                return Err("Inherent implementation has contradictory unit ownership".into());
            }
        }

        Ok(())
    }
}

pub(super) fn verify(facts: &HirFnFacts, own: &Definition) -> Result<(), String> {
    let association = facts
        .build
        .frontend_options
        .inherent_association(&own.definition_identity)?;
    let expected = Association::from(own.clone());
    if association.as_ref() != &expected
        || own.function_span != facts.fn_span
        || facts.build.frontend_options.get("source_file")
            != Some(&serde_json::json!(own.source_file))
    {
        return Err(
            "Inherent callee ownership disagrees with selected-unit association evidence".into(),
        );
    }

    Ok(())
}
