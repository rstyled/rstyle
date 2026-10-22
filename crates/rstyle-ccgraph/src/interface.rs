//! Interface serialization and Jaro-Winkler string features.
//!
//! Each interface serializes as compact JSON with unescaped Unicode: an array
//! of `[role, normalized_type, variadic]` tuples. `IntroPara` holds inputs,
//! `ReturnPara` holds outputs, `Name` is the unqualified identifier. The three
//! Jaro-Winkler scores aggregate by arithmetic mean with equal weights.

use crate::frac::{ArithError, Frac};
use crate::graph::InterfaceElement;
use std::collections::{BTreeMap, VecDeque};
#[cfg(test)]
mod tests;

/// Serialize one interface side exactly as the profile requires.
pub fn serialize_interface(elements: &[InterfaceElement]) -> String {
    let tuples: Vec<(String, String, bool)> = elements
        .iter()
        .map(|e| (e.role.clone(), e.normalized_type.clone(), e.variadic))
        .collect();
    serde_json::to_string(&tuples).expect("interface tuples always serialize")
}

/// The profile's initial normalization vocabulary; anything else needs
/// another profile/version, never an opaque token.
pub fn is_supported_type(normalized: &str) -> bool {
    matches!(
        normalized,
        "bool"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "unit"
    )
}

pub fn is_native_width(normalized: &str) -> bool {
    matches!(normalized, "isize" | "usize")
}

pub fn is_known_role(role: &str) -> bool {
    matches!(
        role,
        "parameter" | "receiver" | "capture" | "return" | "output"
    )
}

/// Exact Jaro-Winkler convention over Unicode scalar values.
///
/// Orientation by `(length, lexicographic scalar sequence)` makes repeated
/// character greedy matching symmetric. The prefix boost always applies;
/// there is no extra `J > 0.7` condition.
pub fn jaro_winkler(first: &str, second: &str) -> Result<Frac, ArithError> {
    if first == second {
        return Ok(Frac::one());
    }
    let a: Vec<char> = first.chars().collect();
    let b: Vec<char> = second.chars().collect();
    if a.is_empty() || b.is_empty() {
        return Ok(Frac::zero());
    }

    let (short, long) = orient(&a, &b);
    let matches = find_matches(short, long);
    if matches.count == 0 {
        return Ok(Frac::zero());
    }

    let jaro = jaro_score(&matches, short.len(), long.len())?;
    let prefix = common_prefix(&a, &b);

    boost(jaro, prefix)
}

fn orient<'a>(a: &'a [char], b: &'a [char]) -> (&'a [char], &'a [char]) {
    if (a.len(), a) <= (b.len(), b) {
        (a, b)
    } else {
        (b, a)
    }
}

struct MatchState {
    count: usize,
    ordered_short: Vec<char>,
    ordered_long: Vec<char>,
}

fn find_matches(short: &[char], long: &[char]) -> MatchState {
    let window = short_long_window(short.len(), long.len());
    let mut taken_short = vec![false; short.len()];
    let mut hay = Haystack::new(long, window);
    for (i, cell) in short.iter().enumerate() {
        if hay.take_first(cell, i).is_some() {
            taken_short[i] = true;
        }
    }

    let ordered_short = picked(short, &taken_short);
    let ordered_long = picked(long, &hay.used);
    MatchState {
        count: ordered_short.len(),
        ordered_short,
        ordered_long,
    }
}

fn short_long_window(short_len: usize, long_len: usize) -> usize {
    (short_len.max(long_len) / 2).saturating_sub(1)
}

struct Haystack {
    positions: BTreeMap<char, VecDeque<usize>>,
    used: Vec<bool>,
    length: usize,
    radius: usize,
}

impl Haystack {
    fn new(chars: &[char], radius: usize) -> Self {
        let mut positions: BTreeMap<char, VecDeque<usize>> = BTreeMap::new();
        for (index, character) in chars.iter().enumerate() {
            positions.entry(*character).or_default().push_back(index);
        }
        Self {
            positions,
            used: vec![false; chars.len()],
            length: chars.len(),
            radius,
        }
    }

    fn take_first(&mut self, needle: &char, i: usize) -> Option<usize> {
        let lo = i.saturating_sub(self.radius);
        let hi = (i + self.radius).min(self.length.saturating_sub(1));
        let positions = self.positions.get_mut(needle)?;
        // The lower bound only advances. Discarding expired positions and
        // consuming the first remaining one preserves the exact greedy match.
        while positions.front().is_some_and(|position| *position < lo) {
            positions.pop_front();
        }

        let position = positions
            .front()
            .is_some_and(|position| *position <= hi)
            .then(|| positions.pop_front())
            .flatten()?;
        self.used[position] = true;
        Some(position)
    }
}

fn picked(chars: &[char], taken: &[bool]) -> Vec<char> {
    chars
        .iter()
        .zip(taken)
        .filter(|(_, taken)| **taken)
        .map(|(c, _)| *c)
        .collect()
}

fn jaro_score(matches: &MatchState, short_len: usize, long_len: usize) -> Result<Frac, ArithError> {
    let m = matches.count as i128;
    let disagreements = sequence_disagreements(matches) as i128;
    let transpositions = Frac::new(disagreements, 2)?;
    let term = Frac::new(m, 1)?.checked_add(transpositions.checked_mul(Frac::new(-1, 1)?)?)?;
    let sum = Frac::new(m, short_len as i128)?
        .checked_add(Frac::new(m, long_len as i128)?)?
        .checked_add(term.checked_div(Frac::new(m, 1)?)?)?;

    sum.checked_div(Frac::new(3, 1)?)
}

fn sequence_disagreements(matches: &MatchState) -> usize {
    matches
        .ordered_short
        .iter()
        .zip(matches.ordered_long.iter())
        .filter(|(x, y)| x != y)
        .count()
}

fn common_prefix(a: &[char], b: &[char]) -> usize {
    a.iter()
        .zip(b.iter())
        .take(4)
        .take_while(|(x, y)| x == y)
        .count()
}

fn boost(jaro: Frac, prefix: usize) -> Result<Frac, ArithError> {
    let one_minus = Frac::one().checked_add(jaro.checked_mul(Frac::new(-1, 1)?)?)?;
    jaro.checked_add(Frac::new(prefix as i128, 10)?.checked_mul(one_minus)?)
}

/// Arithmetic mean of the three field scores; every field counts equally,
/// including verified empty interface arrays.
pub fn string_mean(return_para: Frac, intro_para: Frac, name: Frac) -> Result<Frac, ArithError> {
    return_para
        .checked_add(intro_para)?
        .checked_add(name)?
        .checked_div(Frac::new(3, 1)?)
}
