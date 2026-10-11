//! The nine numerical features and their cosine gate.
//!
//! Coordinates, in order: NumOfCtrlEdges, NumOfExcEdges, NumOfDataEdges,
//! NumOfDeclNodes, NumOfAssignNodes, NumOfCtrlNodes, NumOfStateNodes,
//! NumOfOtherNodes, NumOfReference. Edge occurrences preserve multiplicity
//! and self-edges; different families on identical endpoints stay separate.

use crate::frac::{ArithError, Frac, as_i128};
use crate::graph::{CcLabel, ProjectedGraph};
use rstyle_pdg::EdgeFamily;

pub const FEATURE_NAMES: [&str; 9] = [
    "NumOfCtrlEdges",
    "NumOfExcEdges",
    "NumOfDataEdges",
    "NumOfDeclNodes",
    "NumOfAssignNodes",
    "NumOfCtrlNodes",
    "NumOfStateNodes",
    "NumOfOtherNodes",
    "NumOfReference",
];

pub type Vector9 = [u64; 9];

/// Count projected edge occurrences by family, nodes by label, plus the
/// recomputed reference count. Node counts must sum to projected size.
pub fn numerical_features(graph: &ProjectedGraph, reference_count: u64) -> Vector9 {
    let mut vector = [0u64; 9];
    for edge in &graph.edges {
        vector[family_coordinate(edge.family)] += 1;
    }

    for node in &graph.nodes {
        vector[label_coordinate(node.label)] += 1;
    }

    vector[8] = reference_count;
    vector
}

fn family_coordinate(family: EdgeFamily) -> usize {
    match family {
        EdgeFamily::Control => 0,
        EdgeFamily::Execution => 1,
        EdgeFamily::Data => 2,
    }
}

fn label_coordinate(label: CcLabel) -> usize {
    match label {
        CcLabel::Decl => 3,
        CcLabel::Assign => 4,
        CcLabel::Ctrl => 5,
        CcLabel::State => 6,
        CcLabel::Other => 7,
    }
}

/// Nodes counted across the five classes must equal the projected size.
pub fn node_counts_sum_to_size(vector: &Vector9, size: usize) -> bool {
    vector[3..8].iter().sum::<u64>() == size as u64
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CosineParts {
    pub dot: u128,
    pub norm_a_squared: u128,
    pub norm_b_squared: u128,
}

pub fn cosine_parts(a: &Vector9, b: &Vector9) -> Result<CosineParts, ArithError> {
    let dot = sum_products(a.iter().zip(b.iter()).map(|(x, y)| (*x, *y)))?;
    let norm_a = sum_products(a.iter().zip(a.iter()).map(|(x, y)| (*x, *y)))?;
    let norm_b = sum_products(b.iter().zip(b.iter()).map(|(x, y)| (*x, *y)))?;

    if norm_a == 0 || norm_b == 0 {
        return Err(ArithError {
            what: "zero-norm vector is a contract diagnostic",
        });
    }

    Ok(CosineParts {
        dot,
        norm_a_squared: norm_a,
        norm_b_squared: norm_b,
    })
}

fn sum_products(terms: impl Iterator<Item = (u64, u64)>) -> Result<u128, ArithError> {
    let mut total: u128 = 0;
    for (x, y) in terms {
        total = total.checked_add(mul_u128(x, y)?).ok_or(ArithError {
            what: "cosine overflow",
        })?;
    }
    Ok(total)
}

fn mul_u128(x: u64, y: u64) -> Result<u128, ArithError> {
    u128::from(x).checked_mul(u128::from(y)).ok_or(ArithError {
        what: "cosine overflow",
    })
}

/// Compare `dot / sqrt(norm_a * norm_b) >= threshold` without square roots:
/// `dot^2 >= t^2 * norm_a * norm_b`, all as exact rationals.
pub fn cosine_passes(parts: CosineParts, threshold: Frac) -> Result<bool, ArithError> {
    let overflow = ArithError {
        what: "cosine overflow",
    };
    let left = frac_squared(as_i128(parts.dot)?, 1)?;
    let norms = parts
        .norm_a_squared
        .checked_mul(parts.norm_b_squared)
        .ok_or(overflow)?;
    let right =
        frac_squared(threshold.numerator(), threshold.denominator())?.checked_mul_frac(norms, 1)?;

    left.ge(right)
}

fn frac_squared(num: i128, den: i128) -> Result<Frac, ArithError> {
    let overflow = ArithError {
        what: "cosine overflow",
    };
    if num < 0 || den <= 0 {
        return Err(overflow);
    }

    let num = (num as u128).checked_mul(num as u128).ok_or(overflow)?;
    let den = (den as u128).checked_mul(den as u128).ok_or(overflow)?;

    Frac::new(as_i128(num)?, as_i128(den)?)
}

trait MulFrac {
    fn checked_mul_frac(self, num: u128, den: u128) -> Result<Frac, ArithError>;
}

impl MulFrac for Frac {
    fn checked_mul_frac(self, num: u128, den: u128) -> Result<Frac, ArithError> {
        self.checked_mul(Frac::new(as_i128(num)?, as_i128(den)?)?)
    }
}
