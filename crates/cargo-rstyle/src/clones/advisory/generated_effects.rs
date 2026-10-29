//! Partial expanded-body observations never stand in for authored function facts.
use rstyle_core::{
    advisory::generated_effects::{KEY, Observation},
    config::Config,
    facts::EffectEvent,
    rules::{EffectEvents, effects_from_events},
};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Measurement {
    key: crate::clones::FunctionKey,
    path: String,
    #[serde(flatten)]
    observation: Observation,
    direct_effects: Vec<EffectEvent>,
}

pub(super) fn render(rows: &[Measurement]) -> String {
    rows.iter().map(|row| {
        let body = &row.observation;
        format!(
            "\n{}:{}:{}: {}: {} direct generated-body effects (no source metrics or transitive coverage)",
            row.path, body.line, body.column, body.name, row.direct_effects.len(),
        )
    }).collect()
}

pub(super) fn read(
    function: &crate::clones::Function,
    config: &Config,
    root: &std::path::Path,
) -> Result<Option<Measurement>, String> {
    let Some(raw) = function.facts.build.frontend_options.get(KEY) else {
        return Ok(None);
    };
    let observation = Observation::read(raw)?;
    if observation.name != function.facts.name
        || !function
            .facts
            .source_diagnostics
            .iter()
            .any(|finding| finding.construct == "macro_generated_function")
    {
        return Err("Generated effects disagree with captured expanded owner".into());
    }

    let direct_effects = effects_from_events(
        EffectEvents {
            mutations: &observation.mutations,
            calls: &observation.calls,
            effects: &observation.intrinsic_effects,
        },
        config,
    )
    .into_iter()
    .map(|effect| EffectEvent {
        kind: effect.kind,
        detail: effect.label,
        line: effect.line,
    })
    .collect();
    Ok(Some(Measurement {
        key: function.key,
        path: super::source_path(function, root, true)?,
        observation,
        direct_effects,
    }))
}
