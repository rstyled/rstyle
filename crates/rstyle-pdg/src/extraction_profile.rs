//! Registered local extraction policies; matching configuration is independent.

use crate::{HirFnFacts, ProfileId};
use serde::{Deserialize, Serialize};

pub const SELECTION_KEY: &str = "extraction_profile";
/// Private driver argument, consumed before forwarding the rustc invocation.
pub const DRIVER_SELECTION_FLAG: &str = "--rstyle-extraction-profile";
pub const EVIDENCE_KEY: &str = "rstyle.hir-coverage:selection";
pub const DATA_ID: &str = "rstyle.rust.hir-data";
pub const DATA_MATCHING_ID: &str = "rstyle.ccgraph.data/1.0.0";
pub const DATA_CALLS_MATCHING_ID: &str = "rstyle.ccgraph.data-calls/1.0.0";
pub const LOCAL_ID: &str = "rstyle.rust.hir-coverage";

const EXPANDED_CAPABILITIES: &[&str] = &[
    "rust.const-function-bodies",
    "rust.integer-casts",
    "rust.scalar-match",
    "rust.scalar-match-guards",
    "rust.conditional-source-correspondence",
    "rust.scalar-loops",
    "rust.divergence",
    "rust.verified-primitive-summaries",
    "rust.verified-helper-calls",
    "rust.concrete-read-only-data",
    "rust.verified-read-only-helper-calls",
    "rust.verified-inherent-helper-calls",
    "rust.scalar-or-patterns",
    "rust.scalar-range-patterns",
    "rust.resolved-scalar-constants",
    "rust.scalar-constant-patterns",
    "rust.scalar-constant-range-bounds",
    "rust.scalar-assertions",
    "rust.cfg-source-removal",
    "rust.cfg-inactive-definitions",
    "rust.exhaustive-scalar-match",
    "rust.nested-scalar-or-patterns",
    "rust.nominal-constant-context",
    "rust.aggregate-literal-values",
    "rust.repeat-array-values",
    "rust.named-struct-values",
];

/// Registered evidence identities, including historical staged contracts.
/// Cargo selection exposes only R0 and the complete expanded contract.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractionProfile {
    #[default]
    #[serde(rename = "r0")]
    R0,
    #[serde(rename = "hir-const-v1")]
    ConstV1,
    #[serde(rename = "hir-casts-v1")]
    CastsV1,
    #[serde(rename = "hir-match-v1")]
    MatchV1,
    #[serde(rename = "hir-guards-v1")]
    GuardsV1,
    #[serde(rename = "hir-conditional-v1")]
    ConditionalV1,
    #[serde(rename = "hir-loops-v1")]
    LoopsV1,
    #[serde(rename = "hir-calls-v1")]
    CallsV1,
    #[serde(rename = "hir-helpers-v1")]
    HelpersV1,
    #[serde(rename = "hir-data-v1")]
    DataV1,
    #[serde(rename = "hir-data-v2")]
    DataV2,
    #[serde(rename = "hir-data-v3")]
    DataV3,
}

impl ExtractionProfile {
    /// Select a supported Cargo extraction mode; historical tokens are evidence only.
    pub fn select(value: &str) -> Result<Self, String> {
        match value {
            "r0" => Ok(Self::R0),
            "hir-expanded-v1" => Ok(Self::CallsV1),
            "hir-helpers-v1" => Ok(Self::HelpersV1),
            "hir-data-v1" => Ok(Self::DataV1),
            "hir-data-v2" => Ok(Self::DataV2),
            "hir-data-v3" => Ok(Self::DataV3),
            _ => {
                Err("Extraction selection must be r0 or hir-expanded-v1, or hir-helpers-v1, or hir-data-v1, or hir-data-v2, or hir-data-v3".into())
            }
        }
    }

    /// Interpret frozen frontend evidence, including historical staged tokens.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "r0" => Ok(Self::R0),
            "hir-const-v1" => Ok(Self::ConstV1),
            "hir-casts-v1" => Ok(Self::CastsV1),
            "hir-match-v1" => Ok(Self::MatchV1),
            "hir-guards-v1" => Ok(Self::GuardsV1),
            "hir-conditional-v1" => Ok(Self::ConditionalV1),
            "hir-loops-v1" => Ok(Self::LoopsV1),
            "hir-calls-v1" => Ok(Self::CallsV1),
            "hir-helpers-v1" => Ok(Self::HelpersV1),
            "hir-data-v1" => Ok(Self::DataV1),
            "hir-data-v2" => Ok(Self::DataV2),
            "hir-data-v3" => Ok(Self::DataV3),
            _ => Err(format!("Unregistered extraction profile {value:?}")),
        }
    }

    pub fn from_facts(facts: &HirFnFacts) -> Result<Self, String> {
        match facts.build.frontend_options.get(SELECTION_KEY) {
            None => Ok(Self::R0),
            Some(value) => value
                .as_str()
                .ok_or_else(|| "Malformed extraction profile evidence".to_string())
                .and_then(Self::parse),
        }
    }

    pub fn token(self) -> &'static str {
        match self {
            Self::R0 => "r0",
            Self::ConstV1 => "hir-const-v1",
            Self::CastsV1 => "hir-casts-v1",
            Self::MatchV1 => "hir-match-v1",
            Self::GuardsV1 => "hir-guards-v1",
            Self::ConditionalV1 => "hir-conditional-v1",
            Self::LoopsV1 => "hir-loops-v1",
            Self::CallsV1 => "hir-calls-v1",
            Self::HelpersV1 => "hir-helpers-v1",
            Self::DataV1 => "hir-data-v1",
            Self::DataV2 => "hir-data-v2",
            Self::DataV3 => "hir-data-v3",
        }
    }

    fn local_contract(self) -> Option<(&'static str, usize)> {
        match self {
            Self::R0 => None,
            Self::ConstV1 => Some(("0.1.0", 1)),
            Self::CastsV1 => Some(("0.2.0", 2)),
            Self::MatchV1 => Some(("0.3.0", 3)),
            Self::GuardsV1 => Some(("0.4.0", 4)),
            Self::ConditionalV1 => Some(("0.5.0", 5)),
            Self::LoopsV1 => Some(("0.6.0", 7)),
            Self::CallsV1 => Some(("0.7.0", 8)),
            Self::HelpersV1 => Some(("0.1.0", 9)),
            Self::DataV1 => Some(("1.0.0", 10)),
            Self::DataV2 => Some(("2.0.0", 11)),
            Self::DataV3 => Some(("3.14.0", 26)),
        }
    }

    pub fn policy(self) -> ProfileId {
        match self.local_contract() {
            None => ProfileId::hir_r0(),
            Some((version, _)) => ProfileId {
                id: if self.is_data() {
                    DATA_ID
                } else if self == Self::HelpersV1 {
                    "rstyle.rust.hir-helpers"
                } else {
                    LOCAL_ID
                }
                .into(),
                version: version.into(),
            },
        }
    }

    pub fn supports(self, capability: &str) -> bool {
        self.capabilities().contains(&capability)
    }

    pub fn is_data(self) -> bool {
        matches!(self, Self::DataV1 | Self::DataV2 | Self::DataV3)
    }

    pub fn data_matching(self) -> Option<&'static str> {
        match self {
            Self::DataV1 => Some(DATA_MATCHING_ID),
            Self::DataV2 | Self::DataV3 => Some(DATA_CALLS_MATCHING_ID),
            _ => None,
        }
    }

    pub fn summaries(self) -> &'static [&'static str] {
        if matches!(self, Self::DataV2 | Self::DataV3) {
            return &[
                "rstyle.primitive.unsigned-abs/1",
                "rstyle.primitive.wrapping-mul/1",
                "rstyle.helper.source-body/1",
                "rstyle.helper.shared-return/1",
                "rstyle.helper.read-only-body/1",
                "rstyle.data.slice.len/1",
                "rstyle.data.slice.is-empty/1",
                "rstyle.data.str.len/1",
                "rstyle.data.str.is-empty/1",
            ];
        }
        if self == Self::DataV1 {
            return &[
                "rstyle.primitive.unsigned-abs/1",
                "rstyle.primitive.wrapping-mul/1",
                "rstyle.helper.source-body/1",
                "rstyle.helper.shared-return/1",
                "rstyle.data.slice.len/1",
                "rstyle.data.slice.is-empty/1",
                "rstyle.data.str.len/1",
                "rstyle.data.str.is-empty/1",
            ];
        }
        if self.supports("rust.verified-helper-calls") {
            return &[
                "rstyle.primitive.unsigned-abs/1",
                "rstyle.primitive.wrapping-mul/1",
                "rstyle.helper.source-body/1",
            ];
        }
        if self.supports("rust.verified-primitive-summaries") {
            &[
                "rstyle.primitive.unsigned-abs/1",
                "rstyle.primitive.wrapping-mul/1",
            ]
        } else {
            &[]
        }
    }

    pub fn capabilities(self) -> &'static [&'static str] {
        let count = self.local_contract().map_or(0, |(_, count)| count);
        &EXPANDED_CAPABILITIES[..count]
    }
}

pub fn registered(policy: &ProfileId) -> bool {
    registered_profiles()
        .iter()
        .any(|profile| profile.policy() == *policy)
}

fn registered_profiles() -> [ExtractionProfile; 12] {
    [
        ExtractionProfile::R0,
        ExtractionProfile::ConstV1,
        ExtractionProfile::CastsV1,
        ExtractionProfile::MatchV1,
        ExtractionProfile::GuardsV1,
        ExtractionProfile::ConditionalV1,
        ExtractionProfile::LoopsV1,
        ExtractionProfile::CallsV1,
        ExtractionProfile::HelpersV1,
        ExtractionProfile::DataV1,
        ExtractionProfile::DataV2,
        ExtractionProfile::DataV3,
    ]
}

pub fn local_policy(policy: &ProfileId) -> bool {
    policy.id != crate::R0_POLICY_ID && registered(policy)
}

pub fn policy_supports(policy: &ProfileId, capability: &str) -> bool {
    registered_profiles()
        .iter()
        .any(|profile| profile.policy() == *policy && profile.supports(capability))
}

impl std::str::FromStr for ExtractionProfile {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
