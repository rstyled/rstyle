//! Registered local extraction policies; matching configuration is independent.

use crate::{HirFnFacts, ProfileId};
use serde::{Deserialize, Serialize};

pub const SELECTION_KEY: &str = "extraction_profile";
pub const EVIDENCE_KEY: &str = "rstyle.hir-coverage:selection";
pub const LOCAL_ID: &str = "rstyle.rust.hir-coverage";

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
}

impl ExtractionProfile {
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
        }
    }

    pub fn policy(self) -> ProfileId {
        match self {
            Self::R0 => ProfileId::hir_r0(),
            Self::ConstV1 => ProfileId {
                id: LOCAL_ID.into(),
                version: "0.1.0".into(),
            },
            Self::CallsV1 => ProfileId {
                id: LOCAL_ID.into(),
                version: "0.7.0".into(),
            },
            Self::LoopsV1 => ProfileId {
                id: LOCAL_ID.into(),
                version: "0.6.0".into(),
            },
            Self::ConditionalV1 => ProfileId {
                id: LOCAL_ID.into(),
                version: "0.5.0".into(),
            },
            Self::GuardsV1 => ProfileId {
                id: LOCAL_ID.into(),
                version: "0.4.0".into(),
            },
            Self::MatchV1 => ProfileId {
                id: LOCAL_ID.into(),
                version: "0.3.0".into(),
            },
            Self::CastsV1 => ProfileId {
                id: LOCAL_ID.into(),
                version: "0.2.0".into(),
            },
        }
    }

    pub fn integer_casts(self) -> bool {
        matches!(
            self,
            Self::CastsV1
                | Self::MatchV1
                | Self::GuardsV1
                | Self::ConditionalV1
                | Self::LoopsV1
                | Self::CallsV1
        )
    }

    pub fn primitive_calls(self) -> bool {
        self == Self::CallsV1
    }

    pub fn summaries(self) -> &'static [&'static str] {
        if self.primitive_calls() {
            &[
                "rstyle.primitive.unsigned-abs/1",
                "rstyle.primitive.wrapping-mul/1",
            ]
        } else {
            &[]
        }
    }

    pub fn scalar_loops(self) -> bool {
        matches!(self, Self::LoopsV1 | Self::CallsV1)
    }

    pub fn conditional_source(self) -> bool {
        matches!(self, Self::ConditionalV1 | Self::LoopsV1 | Self::CallsV1)
    }

    pub fn match_guards(self) -> bool {
        matches!(
            self,
            Self::GuardsV1 | Self::ConditionalV1 | Self::LoopsV1 | Self::CallsV1
        )
    }

    pub fn scalar_match(self) -> bool {
        matches!(
            self,
            Self::MatchV1 | Self::GuardsV1 | Self::ConditionalV1 | Self::LoopsV1 | Self::CallsV1
        )
    }

    pub fn capabilities(self) -> &'static [&'static str] {
        match self {
            Self::R0 => &[],
            Self::ConstV1 => &["rust.const-function-bodies"],
            Self::CastsV1 => &["rust.const-function-bodies", "rust.integer-casts"],
            Self::CallsV1 => &[
                "rust.const-function-bodies",
                "rust.integer-casts",
                "rust.scalar-match",
                "rust.scalar-match-guards",
                "rust.conditional-source-correspondence",
                "rust.scalar-loops",
                "rust.divergence",
                "rust.verified-primitive-summaries",
            ],
            Self::LoopsV1 => &[
                "rust.const-function-bodies",
                "rust.integer-casts",
                "rust.scalar-match",
                "rust.scalar-match-guards",
                "rust.conditional-source-correspondence",
                "rust.scalar-loops",
                "rust.divergence",
            ],
            Self::ConditionalV1 => &[
                "rust.const-function-bodies",
                "rust.integer-casts",
                "rust.scalar-match",
                "rust.scalar-match-guards",
                "rust.conditional-source-correspondence",
            ],
            Self::GuardsV1 => &[
                "rust.const-function-bodies",
                "rust.integer-casts",
                "rust.scalar-match",
                "rust.scalar-match-guards",
            ],
            Self::MatchV1 => &[
                "rust.const-function-bodies",
                "rust.integer-casts",
                "rust.scalar-match",
            ],
        }
    }
}

pub fn registered(policy: &ProfileId) -> bool {
    [
        ExtractionProfile::R0,
        ExtractionProfile::ConstV1,
        ExtractionProfile::CastsV1,
        ExtractionProfile::MatchV1,
        ExtractionProfile::GuardsV1,
        ExtractionProfile::ConditionalV1,
        ExtractionProfile::LoopsV1,
        ExtractionProfile::CallsV1,
    ]
    .iter()
    .any(|profile| profile.policy() == *policy)
}

impl std::str::FromStr for ExtractionProfile {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}
