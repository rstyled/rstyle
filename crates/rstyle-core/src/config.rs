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
    /// Left out of what a baseline compares, since compiler diagnostics don't read it: changing it
    /// shouldn't read as a change in their policy.
    #[serde(skip_serializing)]
    pub ignored_result: IgnoredResultConfig,
    /// Left out of what a baseline compares for the same reason: only `rstyle risk` reads it.
    #[serde(skip_serializing)]
    pub risk: RiskConfig,
    pub grouping: GroupingConfig,
}

/// How much a kind of change matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Low,
    Medium,
    High,
}

/// A severity for calling one callee more often than before.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CallRule {
    /// The callee's identity, exactly as `rstyle risk` prints it.
    pub callee: String,
    pub level: Severity,
}

/// How much each kind of change between two runs matters, for `rstyle risk`. Nothing here has a
/// default: how much a change matters is a project's to say, so a change nothing here rates stays
/// unrated rather than being given a severity. Rules are checked on load, so a typo can't quietly
/// turn one off.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(try_from = "UncheckedRisk")]
pub struct RiskConfig {
    /// A function's dependence graph changed, or it was added or removed.
    pub pdg_change: Option<Severity>,
    /// Lowers a medium `pdg_change` to low when at most this many functions changed. 0 is no limit.
    /// It's a policy on size, not a proof that behavior is preserved.
    pub pdg_low_risk_max_functions: usize,
    pub calls: Vec<CallRule>,
}

impl RiskConfig {
    pub fn is_set(&self) -> bool {
        self.pdg_change.is_some() || !self.calls.is_empty()
    }
}

#[derive(Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct UncheckedRisk {
    pdg_change: Option<Severity>,
    pdg_low_risk_max_functions: usize,
    calls: Vec<CallRule>,
}

impl TryFrom<UncheckedRisk> for RiskConfig {
    type Error = String;

    fn try_from(risk: UncheckedRisk) -> Result<Self, String> {
        if risk.pdg_low_risk_max_functions > 0 && risk.pdg_change.is_none() {
            return Err("risk.pdg_low_risk_max_functions requires pdg_change".to_owned());
        }
        let mut callees = std::collections::BTreeSet::new();
        for rule in &risk.calls {
            let named = !rule.callee.is_empty() && rule.callee.trim() == rule.callee;
            if !named || !callees.insert(&rule.callee) {
                return Err("risk.calls require distinct nonempty callee identities".to_owned());
            }
        }
        Ok(Self {
            pdg_change: risk.pdg_change,
            pdg_low_risk_max_functions: risk.pdg_low_risk_max_functions,
            calls: risk.calls,
        })
    }
}

/// A share from 1 to 100, checked on load so a typo can't quietly change what a rule reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(try_from = "usize")]
pub struct Percent(pub usize);

impl TryFrom<usize> for Percent {
    type Error = String;

    fn try_from(percent: usize) -> Result<Self, String> {
        match percent {
            1..=100 => Ok(Self(percent)),
            _ => Err(format!("{percent} is not a percentage from 1 to 100")),
        }
    }
}

/// How much of a callee's calls it takes for one that drops its result to stand out.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct IgnoredResultConfig {
    /// Fewer calls than this say too little about what a callee's callers expect.
    pub min_calls: usize,
    /// How much of a callee's calls must use its result. 100 reports nothing, since then no call
    /// can have dropped it.
    pub min_used_percent: Percent,
}

impl Default for IgnoredResultConfig {
    fn default() -> Self {
        Self {
            min_calls: 5,
            min_used_percent: Percent(90),
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
    fn risk_rules_have_no_defaults() {
        let risk = Config::parse("").unwrap().risk;
        assert!(!risk.is_set());
        assert_eq!(risk.pdg_change, None);

        let set = Config::parse(
            "[risk]\npdg_change = \"medium\"\npdg_low_risk_max_functions = 3\n\
             [[risk.calls]]\ncallee = \"go::os.WriteFile\"\nlevel = \"high\"\n",
        )
        .unwrap()
        .risk;
        assert!(set.is_set());
        assert_eq!(set.pdg_change, Some(Severity::Medium));
        assert_eq!(set.pdg_low_risk_max_functions, 3);
        assert_eq!(set.calls[0].callee, "go::os.WriteFile");
        assert_eq!(set.calls[0].level, Severity::High);

        let only_a_call = "[[risk.calls]]\ncallee = \"f\"\nlevel = \"low\"\n";
        assert!(Config::parse(only_a_call).unwrap().risk.is_set());
    }

    #[test]
    fn a_risk_rule_that_would_quietly_do_nothing_is_an_error() {
        let error = |input: &str| Config::parse(input).unwrap_err().to_string();
        assert!(error("[risk]\npdg_change = \"severe\"").contains("severe"));
        assert!(error("[risk]\ndependency_change = \"high\"").contains("dependency_change"));
        assert!(error("[risk]\npdg_low_risk_max_functions = 3").contains("requires pdg_change"),);

        let rule =
            |callee: &str| format!("[[risk.calls]]\ncallee = \"{callee}\"\nlevel = \"low\"\n");
        for bad in [rule(""), rule(" f"), format!("{0}{0}", rule("f"))] {
            assert!(error(&bad).contains("distinct nonempty"), "{bad}");
        }
        assert!(error("[[risk.calls]]\ncallee = \"f\"").contains("level"));
        assert!(error("[[risk.calls]]\nlevel = \"low\"").contains("callee"));
    }

    #[test]
    fn a_baseline_does_not_compare_risk_rules() {
        let policy = |input: &str| serde_json::to_value(Config::parse(input).unwrap()).unwrap();
        assert_eq!(policy("[risk]\npdg_change = \"high\""), policy(""));
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

    #[test]
    fn ignored_result_has_defaults_a_project_can_change_and_a_share_must_be_one() {
        let default = Config::parse("").unwrap().ignored_result;
        assert_eq!(default.min_calls, 5);
        assert_eq!(default.min_used_percent, Percent(90));

        let set = Config::parse("[ignored_result]\nmin_calls = 20\nmin_used_percent = 75").unwrap();
        assert_eq!(set.ignored_result.min_calls, 20);
        assert_eq!(set.ignored_result.min_used_percent, Percent(75));

        for wrong in [
            "min_used_percent = 0",
            "min_used_percent = 101",
            "min_call = 5",
        ] {
            let input = format!("[ignored_result]\n{wrong}");
            assert!(Config::parse(&input).is_err(), "{wrong}");
        }
    }

    #[test]
    fn a_baseline_compares_nothing_of_ignored_result() {
        let config = Config::parse("[ignored_result]\nmin_calls = 20").unwrap();
        let policy = serde_json::to_value(&config).unwrap();
        assert!(policy.get("ignored_result").is_none(), "{policy}");
    }
}
