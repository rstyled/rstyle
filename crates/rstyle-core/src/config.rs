use crate::facts::EffectKind;
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub exclude: Excludes,
    pub fn_length: FnLengthConfig,
    pub fn_params: FnParamsConfig,
    pub mutation_budget: MutationBudgetConfig,
    pub side_effects: SideEffectsConfig,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct Excludes(pub Vec<String>);

impl Default for Excludes {
    fn default() -> Self {
        Self(vec!["target/**".into()])
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FnLengthConfig {
    pub max_lines: usize,
}

impl Default for FnLengthConfig {
    fn default() -> Self {
        Self { max_lines: 50 }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FnParamsConfig {
    pub max_params: usize,
    pub count_self: bool,
}

impl Default for FnParamsConfig {
    fn default() -> Self {
        Self {
            max_params: 4,
            count_self: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    #[default]
    Root,
    Field,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MutationBudgetConfig {
    pub max_mutations_per_target: usize,
    pub max_mutated_targets: usize,
    pub granularity: Granularity,
}

impl Default for MutationBudgetConfig {
    fn default() -> Self {
        Self {
            max_mutations_per_target: 3,
            max_mutated_targets: 3,
            granularity: Granularity::Root,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SideEffectsConfig {
    pub max_density_milli: usize,
    pub min_statements: usize,
    /// Functions with fewer code lines than this are exempt from the density rule.
    pub min_lines: usize,
    pub weights: Weights,
    pub paths: EffectPaths,
}

impl Default for SideEffectsConfig {
    fn default() -> Self {
        Self {
            max_density_milli: 500,
            min_statements: 3,
            min_lines: 10,
            weights: Weights::default(),
            paths: EffectPaths::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Weights {
    pub mutation: usize,
    pub io: usize,
    pub network: usize,
    pub global: usize,
    pub r#unsafe: usize,
    pub time: usize,
    pub random: usize,
    pub panic: usize,
    pub unknown: usize,
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            mutation: 1,
            io: 3,
            network: 3,
            global: 2,
            r#unsafe: 4,
            time: 1,
            random: 1,
            panic: 0,
            unknown: 1,
        }
    }
}

impl Weights {
    pub fn of(&self, kind: EffectKind) -> usize {
        match kind {
            EffectKind::Mutation => self.mutation,
            EffectKind::Io => self.io,
            EffectKind::Network => self.network,
            EffectKind::Global => self.global,
            EffectKind::Unsafe => self.r#unsafe,
            EffectKind::Time => self.time,
            EffectKind::Random => self.random,
            EffectKind::Panic => self.panic,
            EffectKind::Unknown => self.unknown,
        }
    }
}

/// Callee path prefixes per effect kind. The longest matching prefix wins.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EffectPaths {
    pub io: Vec<String>,
    pub network: Vec<String>,
    pub time: Vec<String>,
    pub random: Vec<String>,
    pub panic: Vec<String>,
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(ToString::to_string).collect()
}

const DEFAULT_IO: &[&str] = &[
    "std::fs::",
    "std::io::",
    "std::env::set_var",
    "std::path::Path::exists",
    "std::path::Path::try_exists",
    "std::path::Path::is_file",
    "std::path::Path::is_dir",
    "std::path::Path::metadata",
    "std::path::Path::canonicalize",
    "std::path::Path::read_dir",
    // Only calls that actually run a process; `Command::env` and friends are builders.
    "std::process::Command::output",
    "std::process::Command::spawn",
    "std::process::Command::status",
    "std::process::exit",
    "std::process::abort",
];

impl Default for EffectPaths {
    fn default() -> Self {
        Self {
            io: strings(DEFAULT_IO),
            network: strings(&["std::net::", "reqwest::", "tokio::net::", "hyper::"]),
            time: strings(&["std::time::SystemTime::now", "std::time::Instant::now"]),
            random: strings(&["rand::"]),
            panic: strings(&["core::panicking::", "std::panicking::"]),
        }
    }
}

impl EffectPaths {
    /// Classifies a resolved callee path; the longest matching prefix wins.
    pub fn classify(&self, callee: &str) -> Option<EffectKind> {
        let tables = [
            (EffectKind::Io, &self.io),
            (EffectKind::Network, &self.network),
            (EffectKind::Time, &self.time),
            (EffectKind::Random, &self.random),
            (EffectKind::Panic, &self.panic),
        ];
        tables
            .iter()
            .flat_map(|(kind, prefixes)| prefixes.iter().map(move |p| (*kind, p)))
            .filter(|(_, prefix)| callee.starts_with(prefix.as_str()))
            .max_by_key(|(_, prefix)| prefix.len())
            .map(|(kind, _)| kind)
    }
}

impl Config {
    pub fn parse(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_gives_defaults() {
        let config = Config::parse("").unwrap();
        assert_eq!(config.fn_length.max_lines, 50);
        assert_eq!(config.fn_params.max_params, 4);
        assert_eq!(config.side_effects.weights.io, 3);
    }

    #[test]
    fn unknown_keys_are_errors() {
        assert!(Config::parse("typo = 1").is_err());
        assert!(Config::parse("[fn_length]\nmax_line = 1").is_err());
    }

    #[test]
    fn partial_override_keeps_other_defaults() {
        let config = Config::parse("[side_effects.weights]\nio = 9").unwrap();
        assert_eq!(config.side_effects.weights.io, 9);
        assert_eq!(config.side_effects.weights.network, 3);
    }

    #[test]
    fn classify_matches_prefixes() {
        let paths = EffectPaths::default();
        assert_eq!(paths.classify("std::fs::write"), Some(EffectKind::Io));
        assert_eq!(
            paths.classify("std::net::TcpStream::connect"),
            Some(EffectKind::Network)
        );
        assert_eq!(paths.classify("alloc::vec::Vec::len"), None);
    }

    #[test]
    fn process_builders_are_not_effects_but_running_is() {
        let paths = EffectPaths::default();
        assert_eq!(
            paths.classify("std::process::Command::output"),
            Some(EffectKind::Io)
        );
        assert_eq!(paths.classify("std::process::Command::env"), None);
    }

    #[test]
    fn classify_longest_prefix_wins() {
        let mut paths = EffectPaths::default();
        paths.random.push("std::io::random::".into());
        assert_eq!(
            paths.classify("std::io::random::x"),
            Some(EffectKind::Random)
        );
        assert_eq!(paths.classify("std::io::stdin"), Some(EffectKind::Io));
    }
}
