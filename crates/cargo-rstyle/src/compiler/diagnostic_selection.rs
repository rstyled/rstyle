//! Explicit diagnostic population; core retains all measurements and rule evaluation.
use super::CompilerSourceScope;
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum, Serialize, Deserialize,
)]
#[clap(rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub(crate) enum Rule {
    FnLength,
    FnParams,
    FnSpacing,
    CyclomaticComplexity,
    CognitiveComplexity,
    MutationPerTarget,
    MutatedTargets,
    SideEffectDensity,
}

impl Rule {
    fn name(self) -> &'static str {
        match self {
            Self::FnLength => "fn_length",
            Self::FnParams => "fn_params",
            Self::FnSpacing => "fn_spacing",
            Self::CyclomaticComplexity => "cyclomatic_complexity",
            Self::CognitiveComplexity => "cognitive_complexity",
            Self::MutationPerTarget => "mutation_per_target",
            Self::MutatedTargets => "mutated_targets",
            Self::SideEffectDensity => "side_effect_density",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub rules: Vec<Rule>,
    pub source_scope: CompilerSourceScope,
}

impl Request {
    pub(crate) fn includes(&self, rule: &str) -> bool {
        rule == "invalid_suppression"
            || self.rules.is_empty()
            || self.rules.iter().any(|requested| requested.name() == rule)
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if !self.rules.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err("Compiler diagnostic rules must be unique and canonical".into());
        }
        Ok(())
    }

    pub(super) fn needs_complexity(&self) -> bool {
        self.includes("cyclomatic_complexity") || self.includes("cognitive_complexity")
    }
}

impl super::Args {
    pub(crate) fn compiler_diagnostic_request(&self) -> Option<Request> {
        if self.compiler_diagnostic_rules.is_empty()
            && self.compiler_diagnostic_scope == CompilerSourceScope::Captured
        {
            return None;
        }

        let mut rules = self.compiler_diagnostic_rules.clone();
        rules.sort_unstable();
        rules.dedup();
        Some(Request {
            rules,
            source_scope: self.compiler_diagnostic_scope,
        })
    }
}
