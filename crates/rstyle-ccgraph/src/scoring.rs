//! Default smaller-graph overlap and experimental cross-product cosine.
//!
//! Both variants use the same pair-selected rounds and weights. Overlap
//! averages label-count matches and normalizes by the smaller graph; cosine
//! self-normalizes with the identical depths and weights in all three
//! kernels. Neither is calibrated, and neither divides twice.

use crate::frac::{ArithError, Frac, as_i128};
use std::collections::BTreeMap;

pub type Histogram = BTreeMap<String, u64>;
pub(crate) type LabelRound<K> = (BTreeMap<K, u64>, BTreeMap<K, u64>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoundScores {
    pub weights: Vec<u64>,
    pub overlap_per_round: Vec<u64>,
    pub cross_per_round: Vec<u128>,
    pub self_a_per_round: Vec<u128>,
    pub self_b_per_round: Vec<u128>,
}

/// Score every round from paired label-count histograms. All rounds must
/// cover the same vertex counts; a mismatch is a diagnostic error.
pub fn score_rounds(
    rounds: &[(Histogram, Histogram)],
    weights: &[u64],
) -> Result<RoundScores, ArithError> {
    score_label_rounds(rounds, weights)
}

/// Label representation is irrelevant to the exact histogram arithmetic.
pub(crate) fn score_label_rounds<K: Ord>(
    rounds: &[LabelRound<K>],
    weights: &[u64],
) -> Result<RoundScores, ArithError> {
    if rounds.len() != weights.len() {
        return Err(ArithError {
            what: "round/weight count mismatch",
        });
    }
    Ok(RoundScores {
        weights: weights.to_vec(),
        overlap_per_round: rounds
            .iter()
            .map(|(hist_a, hist_b)| round_overlap(hist_a, hist_b))
            .collect::<Result<Vec<_>, _>>()?,
        cross_per_round: rounds
            .iter()
            .map(|(hist_a, hist_b)| round_cross(hist_a, hist_b))
            .collect::<Result<Vec<_>, _>>()?,
        self_a_per_round: rounds
            .iter()
            .map(|(hist_a, _)| round_self(hist_a))
            .collect::<Result<Vec<_>, _>>()?,
        self_b_per_round: rounds
            .iter()
            .map(|(_, hist_b)| round_self(hist_b))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn round_overlap<K: Ord>(a: &BTreeMap<K, u64>, b: &BTreeMap<K, u64>) -> Result<u64, ArithError> {
    let mut total: u64 = 0;
    for (label, count_a) in a {
        let count_b = b.get(label).copied().unwrap_or(0);
        total = total
            .checked_add((*count_a).min(count_b))
            .ok_or(ArithError {
                what: "overlap overflow",
            })?;
    }
    Ok(total)
}

fn round_cross<K: Ord>(a: &BTreeMap<K, u64>, b: &BTreeMap<K, u64>) -> Result<u128, ArithError> {
    let mut total: u128 = 0;
    for (label, count_a) in a {
        let count_b = u128::from(b.get(label).copied().unwrap_or(0));
        let term = u128::from(*count_a)
            .checked_mul(count_b)
            .ok_or(ArithError {
                what: "kernel overflow",
            })?;
        total = total.checked_add(term).ok_or(ArithError {
            what: "kernel overflow",
        })?;
    }
    Ok(total)
}

fn round_self<K: Ord>(hist: &BTreeMap<K, u64>) -> Result<u128, ArithError> {
    round_cross(hist, hist)
}

fn weight_sum(weights: &[u64]) -> Result<u128, ArithError> {
    let mut total: u128 = 0;
    for weight in weights {
        total = total.checked_add(u128::from(*weight)).ok_or(ArithError {
            what: "weight overflow",
        })?;
    }
    if total == 0 {
        return Err(ArithError {
            what: "empty weight sum",
        });
    }
    Ok(total)
}

fn weighted(values: &[u64], weights: &[u64]) -> Result<u128, ArithError> {
    accumulate(values.iter().map(|v| u128::from(*v)), weights)
}

fn weighted128(values: &[u128], weights: &[u64]) -> Result<u128, ArithError> {
    accumulate(values.iter().copied(), weights)
}

fn accumulate(values: impl Iterator<Item = u128>, weights: &[u64]) -> Result<u128, ArithError> {
    let mut total: u128 = 0;
    for (value, weight) in values.zip(weights.iter()) {
        let term = value.checked_mul(u128::from(*weight)).ok_or(ArithError {
            what: "weight overflow",
        })?;
        total = total.checked_add(term).ok_or(ArithError {
            what: "weight overflow",
        })?;
    }
    Ok(total)
}

/// `M / min(nA, nB)` where `M` is the weight-averaged match count.
pub fn overlap_score(
    scores: &RoundScores,
    smaller: u64,
    larger: u64,
) -> Result<OverlapDetail, ArithError> {
    if smaller == 0 || larger == 0 {
        return Err(ArithError {
            what: "empty graph has no overlap score",
        });
    }

    let matched = weighted(&scores.overlap_per_round, &scores.weights)?;
    let total_weight = weight_sum(&scores.weights)?;
    let score = Frac::new(
        as_i128(matched)?,
        as_i128(
            total_weight
                .checked_mul(u128::from(smaller))
                .ok_or(fail())?,
        )?,
    )?;
    let larger_coverage = Frac::new(
        as_i128(matched)?,
        as_i128(total_weight.checked_mul(u128::from(larger)).ok_or(fail())?)?,
    )?;
    Ok(OverlapDetail {
        matched,
        total_weight,
        smaller,
        score,
        larger_coverage,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlapDetail {
    pub matched: u128,
    pub total_weight: u128,
    pub smaller: u64,
    pub score: Frac,
    /// Recorded for interpretation only; never the acceptance score.
    pub larger_coverage: Frac,
}

/// `KAB / sqrt(KAA * KBB)` with identical depths and weights everywhere.
pub fn cosine_score(scores: &RoundScores) -> Result<CosineDetail, ArithError> {
    let kab = weighted128(&scores.cross_per_round, &scores.weights)?;
    let kaa = weighted128(&scores.self_a_per_round, &scores.weights)?;
    let kbb = weighted128(&scores.self_b_per_round, &scores.weights)?;
    if kaa == 0 || kbb == 0 {
        return Err(ArithError {
            what: "zero self-kernel is a contract diagnostic",
        });
    }

    Ok(CosineDetail { kab, kaa, kbb })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CosineDetail {
    pub kab: u128,
    pub kaa: u128,
    pub kbb: u128,
}

/// Compare `KAB / sqrt(KAA * KBB) >= threshold` exactly:
/// `KAB^2 * q^2 >= p^2 * KAA * KBB` for threshold `p/q`.
pub fn cosine_passes(detail: &CosineDetail, threshold: Frac) -> Result<bool, ArithError> {
    let left = frac_term(
        detail.kab,
        detail.kab,
        threshold.denominator(),
        threshold.denominator(),
    )?;
    let right = frac_term(
        detail.kaa,
        detail.kbb,
        threshold.numerator(),
        threshold.numerator(),
    )?;
    left.ge(right)
}

fn frac_term(a: u128, b: u128, c: i128, d: i128) -> Result<Frac, ArithError> {
    let num = a.checked_mul(b).ok_or(fail())?;
    let scaled = as_i128(num)?
        .checked_mul(c)
        .ok_or(fail())?
        .checked_mul(d)
        .ok_or(fail())?;
    Frac::new(scaled, 1)
}

fn fail() -> ArithError {
    ArithError {
        what: "scoring overflow",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_and_readable_labels_have_identical_exact_scores() {
        let rounds = vec![
            (
                BTreeMap::from([(2, 3), (10, 5)]),
                BTreeMap::from([(2, 1), (10, 7)]),
            ),
            (
                BTreeMap::from([(1, 4), (11, 4)]),
                BTreeMap::from([(1, 3), (12, 5)]),
            ),
        ];
        let readable: Vec<_> = rounds
            .iter()
            .map(|(first, second)| {
                let rename = |hist: &BTreeMap<u64, u64>| {
                    hist.iter()
                        .map(|(label, count)| (format!("L{label}"), *count))
                        .collect::<Histogram>()
                };
                (rename(first), rename(second))
            })
            .collect();
        assert_eq!(
            score_label_rounds(&rounds, &[2, 1]).unwrap(),
            score_rounds(&readable, &[2, 1]).unwrap()
        );
    }

    #[test]
    fn integer_histograms_preserve_checked_overflow_failures() {
        let histogram = BTreeMap::from([(2, u64::MAX), (10, u64::MAX)]);
        let result = score_label_rounds(&[(histogram.clone(), histogram)], &[1]);
        assert_eq!(result.unwrap_err().what, "overlap overflow");
        let histogram = BTreeMap::from([(2, u64::MAX)]);
        let result = score_label_rounds(&[(histogram.clone(), histogram)], &[1, 2]);
        assert_eq!(result.unwrap_err().what, "round/weight count mismatch");
    }
}
