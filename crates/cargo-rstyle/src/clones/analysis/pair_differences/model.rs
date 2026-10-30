use rstyle_pdg::{HirFnFacts, HirNode, HirRef};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Coverage {
    Complete,
    Unsupported,
    Ambiguous,
    BudgetExhausted,
}

/// Coverage is of this closed native expression comparison, never semantics.
#[derive(Debug)]
pub(crate) struct Assessment<'a> {
    pub coverage: Coverage,
    pub correspondence: Vec<Correspondence<'a>>,
    pub holes: Vec<Hole<'a>>,
    pub reason: Option<&'static str>,
}

/// Exact borrowed members and their completed comparison travel together.
/// Construction remains inside the native consumer's checked endpoint path.
pub(crate) struct ComparedPair<'a> {
    pub(super) left: &'a HirFnFacts,
    pub(super) right: &'a HirFnFacts,
    pub(super) builtin_operations: Option<[usize; 2]>,
    pub(super) assessment: Assessment<'a>,
}

impl<'a> ComparedPair<'a> {
    pub(crate) fn assessment(&self) -> &Assessment<'a> {
        &self.assessment
    }

    pub(crate) fn into_assessment(self) -> Assessment<'a> {
        self.assessment
    }

    pub(crate) fn helper_inputs(&self) -> Result<HelperInputs<'a>, HelperUnavailable> {
        if self.assessment.coverage != Coverage::Complete {
            return Err(HelperUnavailable {
                coverage: self.assessment.coverage,
                reason: self
                    .assessment
                    .reason
                    .unwrap_or("native comparison is incomplete"),
            });
        }
        let builtin_operations = self.builtin_operations.ok_or(HelperUnavailable {
            coverage: Coverage::Unsupported,
            reason: "helper profile requires exclusively u32 signature and native expression nodes",
        })?;
        Ok(HelperInputs {
            left: self.left,
            right: self.right,
            builtin_operations,
        })
    }
}

#[derive(Debug)]
pub(crate) struct HelperInputs<'a> {
    pub left: &'a HirFnFacts,
    pub right: &'a HirFnFacts,
    pub builtin_operations: [usize; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HelperUnavailable {
    pub coverage: Coverage,
    pub reason: &'static str,
}

impl Assessment<'_> {
    pub(super) fn unavailable(coverage: Coverage, reason: &'static str) -> Self {
        Self {
            coverage,
            correspondence: vec![],
            holes: vec![],
            reason: Some(reason),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Correspondence<'a> {
    pub left: &'a HirRef,
    pub right: &'a HirRef,
    pub role: Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Role {
    Root,
    BlockTail,
    UnaryOperand,
    BinaryLeft,
    BinaryRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HoleKind {
    Literal,
    FormalRole,
    Operation,
    Shape,
}

#[derive(Debug)]
pub(crate) struct Hole<'a> {
    pub left: &'a HirNode,
    pub right: &'a HirNode,
    pub kind: HoleKind,
    pub role: Role,
}
