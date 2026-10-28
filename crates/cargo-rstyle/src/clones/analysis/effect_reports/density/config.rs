use crate::clones::Args;
use rstyle_core::{
    config::{EffectPaths, Excludes, SideEffectsConfig, Weights},
    exclusions::Matcher,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub(in crate::clones) struct Settings {
    pub(super) exclude: Vec<String>,
    pub(super) side_effects: SideEffectsConfig,
    #[serde(skip)]
    pub(super) exclusions: Matcher,
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Configuration {
    exclude: Excludes,
    side_effects: Policy,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Policy {
    max_density_milli: usize,
    min_statements: usize,
    min_lines: usize,
    weights: Weights,
    paths: EffectPaths,
}

impl Default for Policy {
    fn default() -> Self {
        let default = SideEffectsConfig::default();
        Self {
            max_density_milli: default.max_density_milli,
            min_statements: default.min_statements,
            min_lines: default.min_lines,
            weights: default.weights,
            paths: default.paths,
        }
    }
}

pub(in crate::clones) fn configuration(args: &Args) -> Result<Option<Settings>, String> {
    if !args.effect_density {
        return Ok(None);
    }
    let config = match &args.effect_config {
        None => Configuration::default(),
        Some(path) => {
            let text =
                std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?
        }
    };

    let exclusions = Matcher::compile(&config.exclude.0)?;
    let policy = config.side_effects;
    Ok(Some(Settings {
        exclude: config.exclude.0,
        exclusions,
        side_effects: SideEffectsConfig {
            max_density_milli: policy.max_density_milli,
            min_statements: policy.min_statements,
            min_lines: policy.min_lines,
            transitive_depth: 2,
            weights: policy.weights,
            paths: policy.paths,
        },
    }))
}
