//! The compiler-advisory observations a capture records when the advisory run asks
//! for them: each function's style facts, and its dependence graph for pattern search,
//! derived by the core from the document.

use super::Reader;
use crate::frontend::HirFnFacts;
use rstyle_core::advisory::{self, Observation, generated_effects::CONNECTED_KEY, patterns};
use rstyle_core::cpg::model::{Document, Function};
use rstyle_core::cpg::records::{Measured, UNMEASURED, measure};
use rstyle_core::facts::FnFacts;

/// Each function's core facts, in document order, when the advisory run asks for them.
pub(super) fn derive(document: &Document) -> Result<Option<Vec<FnFacts>>, String> {
    if !requested(advisory::CAPTURE_ENV) {
        return Ok(None);
    }
    let facts = match requested(patterns::CAPTURE_ENV) {
        true => rstyle_core::cpg::all_facts(document)?,
        false => rstyle_core::cpg::rule_facts(document)?,
    };
    Ok(Some(facts))
}

fn requested(variable: &str) -> bool {
    std::env::var_os(variable).as_deref() == Some(std::ffi::OsStr::new("1"))
}

impl Reader<'_> {
    /// The advisory observation of a function, and its graph where patterns are searched.
    pub(super) fn advisory(
        &self,
        function: &Function,
        facts: &mut HirFnFacts,
    ) -> Result<(), String> {
        let (Some(derived), Some(sources)) = (self.advisory.as_ref(), self.sources.as_ref()) else {
            return Ok(());
        };
        let identity = self
            .tables
            .definition(&function.definition)?
            .stable_identity
            .as_str();
        let collected = derived.iter().find(|facts| facts.id == identity).cloned();
        let options = &mut facts.build.frontend_options;
        let observation = match measure(function, collected, sources)? {
            Measured::Known(mut collected) => {
                if let Some(graph) = collected.pdg.take() {
                    let graph = patterns::Observation {
                        schema: patterns::KEY.into(),
                        owner: collected.id.clone(),
                        graph,
                    };
                    let value = serde_json::to_value(graph).map_err(|error| error.to_string())?;
                    options.insert(patterns::KEY.into(), value);
                }
                Observation::Known {
                    metrics: advisory::METRICS.into(),
                    facts: collected,
                }
            }
            Measured::Generated(observation) => {
                let value = serde_json::to_value(observation).map_err(|error| error.to_string())?;
                options.insert(CONNECTED_KEY.into(), value);
                unmeasured()
            }
            Measured::Unavailable => unmeasured(),
        };

        let value = serde_json::to_value(observation).map_err(|error| error.to_string())?;
        options.insert(advisory::KEY.into(), value);
        Ok(())
    }
}

fn unmeasured() -> Observation {
    Observation::Unavailable {
        reason: UNMEASURED.into(),
    }
}
