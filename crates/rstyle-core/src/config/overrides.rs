use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use toml::{Table, Value};

use super::Config;

/// Sections an override may change: the ones measured per function.
const PER_FUNCTION: &[&str] = &[
    "fn_length",
    "fn_params",
    "fn_spacing",
    "complexity",
    "mutation_budget",
    "side_effects",
];

/// Rule settings layered onto the base config for functions whose source path matches.
#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "Table")]
pub struct Override {
    paths: GlobSet,
    rules: Table,
}

impl Override {
    pub(super) fn matches(&self, path: &str) -> bool {
        self.paths.is_match(path)
    }

    pub(super) fn rules(&self) -> &Table {
        &self.rules
    }
}

impl TryFrom<Table> for Override {
    type Error = String;

    fn try_from(mut rules: Table) -> Result<Self, String> {
        let paths = rules.remove("paths").ok_or("an override needs `paths`")?;
        let paths = glob_set(Vec::deserialize(paths).map_err(|e| e.to_string())?)?;
        if let Some(key) = rules
            .keys()
            .find(|key| !PER_FUNCTION.contains(&key.as_str()))
        {
            return Err(format!("`{key}` cannot be overridden by path"));
        }
        if rules
            .get("side_effects")
            .and_then(|s| s.get("transitive_depth"))
            .is_some()
        {
            return Err("`side_effects.transitive_depth` cannot be overridden by path".into());
        }
        Config::deserialize(Value::Table(rules.clone())).map_err(|e| e.to_string())?;

        Ok(Self { paths, rules })
    }
}

fn glob_set(paths: Vec<String>) -> Result<GlobSet, String> {
    if paths.is_empty() {
        return Err("an override needs at least one path".into());
    }
    let mut builder = GlobSetBuilder::new();
    for glob in paths.iter().flat_map(|path| globs(path)) {
        builder.add(Glob::new(&glob).map_err(|e| e.to_string())?);
    }
    builder.build().map_err(|e| e.to_string())
}

/// A glob is used as is. A plain name is a module anywhere in the tree:
/// `adapters` matches `**/adapters/**` and `**/adapters.rs`; `x/main.rs` matches `**/x/main.rs`.
fn globs(path: &str) -> Vec<String> {
    if path.contains(['*', '?', '[', '{']) {
        return vec![path.into()];
    }
    let path = path.trim_matches('/');
    if path.ends_with(".rs") {
        return vec![format!("**/{path}")];
    }
    vec![format!("**/{path}/**"), format!("**/{path}.rs")]
}

/// Deep-merges `layer` into `base`: tables merge key by key, everything else replaces.
pub(super) fn merge(base: &mut Table, layer: &Table) {
    for (key, value) in layer {
        match (base.get_mut(key), value) {
            (Some(Value::Table(into)), Value::Table(from)) => merge(into, from),
            _ => {
                base.insert(key.clone(), value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests;
