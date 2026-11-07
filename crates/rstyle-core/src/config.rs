use crate::facts::EffectKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub exclude: Excludes,
    pub fn_length: FnLengthConfig,
    pub fn_params: FnParamsConfig,
    pub fn_spacing: FnSpacingConfig,
    pub complexity: ComplexityConfig,
    pub mutation_budget: MutationBudgetConfig,
    pub side_effects: SideEffectsConfig,
    pub grouping: GroupingConfig,
    /// Left out of what a baseline compares while nothing is on, so a project that hasn't asked for
    /// these rules sees no change in its policy. Turning one on is one.
    #[serde(skip_serializing_if = "StrictStyleConfig::all_off")]
    pub strict_style: StrictStyleConfig,
}

/// Opinions about how code could be shaped rather than limits it passed, so each is off unless a
/// project turns it on. One that's on reports and fails a run like any other style rule. A rule
/// that's off says nothing about a policy, whatever its other settings hold.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StrictStyleConfig {
    #[serde(skip_serializing_if = "GuardClauseConfig::is_off")]
    pub guard_clause: GuardClauseConfig,
    /// A parameter whose name says what it means, held as plain text or an integer.
    pub primitive_obsession: bool,
}

impl StrictStyleConfig {
    fn all_off(&self) -> bool {
        !self.guard_clause.enabled && !self.primitive_obsession
    }
}

/// An `if` whose `else` only returns, around a body that could stop being nested.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GuardClauseConfig {
    pub enabled: bool,
    /// Fewer statements than this under the `if` are no main path to bring out of it. An empty
    /// block never is, so 0 reads as 1.
    pub min_nested_statements: usize,
}

impl GuardClauseConfig {
    fn is_off(&self) -> bool {
        !self.enabled
    }
}

impl Default for GuardClauseConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_nested_statements: 2,
        }
    }
}

/// Findings start strictly above either configured complexity limit.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ComplexityConfig {
    pub max_cyclomatic: usize,
    pub max_cognitive: usize,
}

impl Default for ComplexityConfig {
    fn default() -> Self {
        Self {
            max_cyclomatic: 12,
            max_cognitive: 12,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Excludes(pub Vec<String>);

impl Default for Excludes {
    fn default() -> Self {
        Self(vec!["target/**".into()])
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct FnLengthConfig {
    pub max_lines: usize,
}

impl Default for FnLengthConfig {
    fn default() -> Self {
        Self { max_lines: 50 }
    }
}

/// Bodies with at least `min_statements` top-level statements need a blank line between phases.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct FnSpacingConfig {
    /// Top-level statements (tail expression included) at which the rule starts to
    /// apply. Counted in statements, not lines, so long initializers don't trigger it.
    /// 0 disables the rule.
    pub min_statements: usize,
}

impl Default for FnSpacingConfig {
    fn default() -> Self {
        Self { min_statements: 5 }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Granularity {
    #[default]
    Root,
    Field,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SideEffectsConfig {
    pub max_density_milli: usize,
    pub min_statements: usize,
    /// Functions with fewer code lines than this are exempt from the density rule.
    pub min_lines: usize,
    /// How many call hops an effect propagates up the call graph (0 disables).
    pub transitive_depth: usize,
    pub weights: Weights,
    pub paths: EffectPaths,
}

impl Default for SideEffectsConfig {
    fn default() -> Self {
        Self {
            max_density_milli: 500,
            min_statements: 3,
            min_lines: 10,
            transitive_depth: 3,
            weights: Weights::default(),
            paths: EffectPaths::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, serde::Serialize)]
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
            network: strings(&[
                "std::net::TcpStream::",
                "std::net::TcpListener::",
                "std::net::UdpSocket::",
                "reqwest::",
                "tokio::net::",
                "hyper::",
            ]),
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

/// Rule ids, validated on load so a typo never silently changes grouping.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "Vec<String>")]
pub struct RuleList(pub Vec<String>);

impl TryFrom<Vec<String>> for RuleList {
    type Error = String;

    fn try_from(rules: Vec<String>) -> Result<Self, String> {
        match rules
            .iter()
            .find(|r| !crate::suppress::RULE_IDS.contains(&r.as_str()))
        {
            Some(unknown) => Err(format!("unknown rule `{unknown}`")),
            None => Ok(Self(rules)),
        }
    }
}

/// Which violations must be fixed together (see `groups`). A fix's footprint is the
/// function itself plus, for `caller_rules`, the functions that call it.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GroupingConfig {
    /// Rules whose fix edits the callers too (a signature change). Other rules only
    /// touch the function itself; their callers show up as soft hints.
    pub caller_rules: RuleList,
    /// How many caller hops a `caller_rules` fix reaches. Wider values create hub groups.
    pub caller_hops: usize,
}

impl Default for GroupingConfig {
    fn default() -> Self {
        Self {
            caller_rules: RuleList(vec!["fn_params".into()]),
            caller_hops: 1,
        }
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
        assert_eq!(config.fn_spacing.min_statements, 5);
        assert_eq!(config.side_effects.weights.io, 3);
    }

    #[test]
    fn a_baseline_compares_strict_style_only_once_a_rule_is_on() {
        let policy = |input: &str| serde_json::to_value(Config::parse(input).unwrap()).unwrap();
        let off = policy("[strict_style.guard_clause]\nenabled = false");
        assert!(off.get("strict_style").is_none(), "{off}");
        assert_eq!(off, policy(""));

        let tuned = "[strict_style.guard_clause]\nmin_nested_statements = 5";
        assert_eq!(policy(tuned), policy(""), "a rule that's off has no limits");

        let on = policy("[strict_style.guard_clause]\nenabled = true");
        let rule = &on["strict_style"]["guard_clause"];
        assert_eq!(rule["enabled"], true, "{on}");
        assert_eq!(rule["min_nested_statements"], 2, "{on}");
    }

    #[test]
    fn strict_style_rules_are_off_unless_asked_for() {
        let default = Config::parse("").unwrap().strict_style;
        assert!(!default.guard_clause.enabled);
        assert!(!default.primitive_obsession);
        assert_eq!(default.guard_clause.min_nested_statements, 2);

        let set = "[strict_style.guard_clause]\nenabled = true\nmin_nested_statements = 4";
        let set = Config::parse(set).unwrap().strict_style.guard_clause;
        assert!(set.enabled);
        assert_eq!(set.min_nested_statements, 4);

        assert!(Config::parse("[strict_style.guard_clauses]\nenabled = true").is_err());
        assert!(Config::parse("[strict_style.guard_clause]\nenable = true").is_err());
        assert!(Config::parse("[strict_style]\nguard_clause = true").is_err());
    }

    /// The example is where a project reads what it can set, so a rule's table has to state every
    /// setting, and state what it defaults to.
    fn example_states_the_defaults_of<Rule: Serialize + Default>(rule: &str) {
        let example: toml::Table = toml::from_str(include_str!("../../../rstyle.toml")).unwrap();
        let defaults = toml::Value::try_from(Rule::default()).unwrap();
        assert_eq!(example["strict_style"][rule], defaults, "{rule}");
    }

    #[test]
    fn the_example_config_states_strict_styles_defaults() {
        example_states_the_defaults_of::<GuardClauseConfig>("guard_clause");
    }

    #[test]
    fn grouping_rules_are_validated() {
        assert!(Config::parse("[grouping]\ncaller_rules = [\"fn_param\"]").is_err());
        let ok =
            Config::parse("[grouping]\ncaller_rules = [\"fn_length\"]\ncaller_hops = 2").unwrap();
        assert_eq!(ok.grouping.caller_rules.0, ["fn_length"]);
        assert_eq!(Config::default().grouping.caller_hops, 1);
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
        // Address getters are not network IO; sockets are.
        assert_eq!(paths.classify("std::net::SocketAddr::ip"), None);
        assert_eq!(
            paths.classify("std::net::UdpSocket::send_to"),
            Some(EffectKind::Network)
        );
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
