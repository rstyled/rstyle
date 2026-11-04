//! The compiler-advisory observations a capture records when the advisory run asks
//! for them: each function's style facts, and its dependence graph for pattern search,
//! derived by the core from the document.

use super::Reader;
use crate::frontend::HirFnFacts;
use rstyle_core::advisory::{self, Observation, patterns};
use rstyle_core::cpg::model::{Document, Function};
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
        let Some(derived) = self.advisory.as_ref() else {
            return Ok(());
        };
        let position = self
            .unit
            .functions
            .as_deref()
            .unwrap_or_default()
            .iter()
            .position(|candidate| std::ptr::eq(candidate, function))
            .ok_or("Function is not in its document")?;
        // A function whose signature a macro writes has no original source to measure.
        let generated = !std::ptr::eq(super::located(function), &function.span);
        let observation = match derived.get(position).filter(|_| !generated) {
            Some(collected) => self.known(collected, facts)?,
            None => {
                if let Some(collected) = derived.get(position).filter(|_| generated) {
                    self.generated_effects(function, (collected, facts))?;
                }
                Observation::Unavailable {
                    reason: "Existing compiler collector has no original function source".into(),
                }
            }
        };

        let value = serde_json::to_value(observation).map_err(|error| error.to_string())?;
        facts
            .build
            .frontend_options
            .insert(advisory::KEY.into(), value);
        Ok(())
    }
}

impl Reader<'_> {
    /// A measured function's facts, with its graph moved to the pattern observation.
    fn known(&self, collected: &FnFacts, facts: &mut HirFnFacts) -> Result<Observation, String> {
        let mut collected = collected.clone();
        if let Some(graph) = collected.pdg.take() {
            let graph = patterns::Observation {
                schema: patterns::KEY.into(),
                owner: collected.id.clone(),
                graph,
            };
            let value = serde_json::to_value(graph).map_err(|error| error.to_string())?;
            facts
                .build
                .frontend_options
                .insert(patterns::KEY.into(), value);
        }

        Ok(Observation::Known {
            metrics: advisory::METRICS.into(),
            facts: Box::new(collected),
        })
    }

    /// A macro-generated function's own events, located at the macro's call site.
    fn generated_effects(
        &self,
        function: &Function,
        (collected, facts): (&FnFacts, &mut HirFnFacts),
    ) -> Result<(), String> {
        use rstyle_core::advisory::generated_effects::{CONNECTED_KEY, Connections, Observation};
        let located = super::located(function);
        let content = self
            .file(&located.file)?
            .content
            .as_deref()
            .ok_or("The generated function's file has no content")?;
        let before = content
            .get(..located.start)
            .ok_or("The call site lies outside its file")?;
        let line = before.matches('\n').count() + 1;
        let column = before
            .rsplit('\n')
            .next()
            .unwrap_or_default()
            .chars()
            .count()
            + 1;
        let observation = Observation {
            metrics: CONNECTED_KEY.into(),
            name: facts.name.clone(),
            line,
            column,
            mutations: collected.mutations.clone(),
            calls: collected.calls.clone(),
            intrinsic_effects: collected.effects.clone(),
            connections: Some(Connections {
                id: collected.id.clone(),
                implements: collected.implements.clone(),
                edges: collected.edges.clone(),
            }),
        };
        let value = serde_json::to_value(observation).map_err(|error| error.to_string())?;
        facts
            .build
            .frontend_options
            .insert(CONNECTED_KEY.into(), value);

        Ok(())
    }
}
