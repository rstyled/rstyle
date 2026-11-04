use std::collections::BTreeMap;

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;

use crate::facts::EffectKind;

/// Partial effect weights for matching function source paths.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceWeights {
    paths: SourcePaths,
    weights: BTreeMap<EffectKind, usize>,
}

impl SourceWeights {
    pub(super) fn weight(&self, path: &str, kind: EffectKind) -> Option<usize> {
        if !self.paths.0.is_match(path) {
            return None;
        }
        self.weights.get(&kind).copied()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "Vec<String>")]
struct SourcePaths(GlobSet);

impl TryFrom<Vec<String>> for SourcePaths {
    type Error = String;

    fn try_from(paths: Vec<String>) -> Result<Self, Self::Error> {
        if paths.is_empty() {
            return Err("a source weight override needs at least one path".into());
        }
        let mut builder = GlobSetBuilder::new();
        for path in paths {
            builder.add(Glob::new(&path).map_err(|error| error.to_string())?);
        }
        builder.build().map(Self).map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests;
