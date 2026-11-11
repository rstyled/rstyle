use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use super::Config;

/// Sections an override may change: the ones measured per function.
const PER_FUNCTION: &[&str] = &[
    "complexity",
    "mutation_budget",
    "side_effects",
    "strict_style",
];

/// The `strict_style` rules an override may change. The others judge the whole run at once, under
/// the base policy.
const PER_FUNCTION_STRICT_STYLE: &[&str] = &["fn_length", "fn_params", "fn_spacing"];

/// `side_effects` keys read once for the whole run: effects are classified and propagated
/// through the call graph before any function's policy applies.
const RUN_WIDE_SIDE_EFFECTS: &[&str] = &["transitive_depth", "paths"];

/// Rule settings layered onto the base config for functions whose source path matches.
/// Serializes back to the table it was written as, so a baseline sees the policy.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(try_from = "Table", into = "Table")]
pub struct Override {
    patterns: Vec<String>,
    paths: GlobSet,
    rules: Table,
}

impl From<Override> for Table {
    fn from(layer: Override) -> Self {
        let mut table = layer.rules;
        table.insert("paths".into(), layer.patterns.into());
        table
    }
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
        let patterns = rules.remove("paths").ok_or("an override needs `paths`")?;
        let patterns = Vec::<String>::deserialize(patterns).map_err(|e| e.to_string())?;
        let paths = glob_set(&patterns)?;
        if let Some(moved) = super::moved_to_strict_style(&rules) {
            return Err(moved);
        }
        if let Some(key) = rules
            .keys()
            .find(|key| !PER_FUNCTION.contains(&key.as_str()))
        {
            return Err(format!("`{key}` cannot be overridden by path"));
        }
        if let Some(key) = RUN_WIDE_SIDE_EFFECTS.iter().find(|key| {
            rules
                .get("side_effects")
                .is_some_and(|s| s.get(**key).is_some())
        }) {
            return Err(format!("`side_effects.{key}` cannot be overridden by path"));
        }
        if let Some(key) = rules
            .get("strict_style")
            .and_then(Value::as_table)
            .and_then(|rules| {
                rules
                    .keys()
                    .find(|key| !PER_FUNCTION_STRICT_STYLE.contains(&key.as_str()))
            })
        {
            return Err(format!("`strict_style.{key}` cannot be overridden by path"));
        }
        Config::deserialize(Value::Table(rules.clone())).map_err(|e| e.to_string())?;

        Ok(Self {
            patterns,
            paths,
            rules,
        })
    }
}

fn glob_set(paths: &[String]) -> Result<GlobSet, String> {
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
