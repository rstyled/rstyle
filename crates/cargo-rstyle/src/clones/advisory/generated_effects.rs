//! Partial expanded-body observations never stand in for authored function facts.
use rstyle_core::{
    advisory::generated_effects::{CONNECTED_KEY, Connections, KEY, Observation},
    config::Config,
    rules::{Effect, EffectEvents, effects_from_events},
};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Measurement {
    pub(super) key: crate::clones::FunctionKey,
    path: String,
    #[serde(flatten)]
    observation: Observation,
    #[serde(serialize_with = "serialize_effects")]
    pub(super) direct_effects: Vec<Effect>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_propagated"
    )]
    pub(super) propagated_effects: Option<Vec<Effect>>,
}

#[derive(Serialize)]
struct EffectRef<'a> {
    kind: rstyle_core::facts::EffectKind,
    detail: &'a str,
    line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    via: Option<&'a str>,
}

fn serialize_effects<S: serde::Serializer>(
    effects: &[Effect],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(effects.iter().map(|effect| EffectRef {
        kind: effect.kind,
        detail: &effect.label,
        line: effect.line,
        via: effect.via.as_deref(),
    }))
}

fn serialize_propagated<S: serde::Serializer>(
    effects: &Option<Vec<Effect>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match effects {
        Some(effects) => serialize_effects(effects, serializer),
        None => serializer.serialize_none(),
    }
}

impl Measurement {
    pub(super) fn connections(&self) -> Option<&Connections> {
        self.observation.connections.as_ref()
    }
    pub(super) fn path(&self) -> &str {
        &self.path
    }
    pub(super) fn owner(&self) -> rstyle_core::graph::EffectOwner<'_> {
        rstyle_core::graph::EffectOwner {
            name: &self.observation.name,
            mutations: &self.observation.mutations,
            edges: self
                .connections()
                .map_or(&[], |connections| connections.edges.as_slice()),
        }
    }
}

pub(super) fn render(rows: &[Measurement]) -> String {
    rows.iter().map(|row| {
        let body = &row.observation;
        let propagated = row.propagated_effects.as_ref().map_or_else(
            || "propagation unavailable".to_owned(),
            |effects| format!("{} effects after propagation", effects.len()),
        );
        format!(
            "\n{}:{}:{}: {}: {} direct generated-body effects, {propagated} (partial coverage; source metrics unavailable)",
            row.path, body.line, body.column, body.name, row.direct_effects.len(),
        )
    }).collect()
}

pub(super) fn read(
    function: &crate::clones::Function,
    config: &Config,
    root: &std::path::Path,
) -> Result<Option<Measurement>, String> {
    let options = &function.facts.build.frontend_options;
    let Some(raw) = options.get(CONNECTED_KEY).or_else(|| options.get(KEY)) else {
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
    );
    Ok(Some(Measurement {
        key: function.key,
        path: super::source_path(function, root, true)?,
        observation,
        direct_effects,
        propagated_effects: None,
    }))
}
