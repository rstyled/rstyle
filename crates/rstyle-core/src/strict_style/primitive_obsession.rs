//! A parameter whose name says what it means, held as plain text or an integer in more than one
//! function. A named type would say what it holds, and stop two of them being swapped.

use crate::facts::{FnFacts, Scalar};
use crate::report::Diagnostic;
use crate::rules::{Measure, diagnostic};
use std::collections::BTreeMap;

pub(super) const RULE: &str = "primitive_obsession";

/// Fewer functions than this holding a meaning as a plain value say nothing about a codebase.
const MIN_FUNCTIONS: usize = 2;

/// Words in a name that mean something a type could say.
const MEANINGS: [&str; 32] = [
    "email",
    "phone",
    "url",
    "uri",
    "uuid",
    "guid",
    "id",
    "name",
    "address",
    "street",
    "city",
    "state",
    "zip",
    "postal",
    "country",
    "currency",
    "price",
    "amount",
    "quantity",
    "ssn",
    "passport",
    "license",
    "username",
    "password",
    "token",
    "apikey",
    "secret",
    "creditcard",
    "iban",
    "isbn",
    "sku",
    "serial",
];

/// The words of a name, lowercase, split at `_`, `-` and each lower-to-upper change: `userEmail`
/// and `user_email` both hold `email`, and `valid` doesn't hold `id`.
fn words(name: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut word = String::new();
    let mut after_lower = false;

    for letter in name.chars() {
        let separated = letter == '_' || letter == '-';
        if (separated || (letter.is_uppercase() && after_lower)) && !word.is_empty() {
            found.push(std::mem::take(&mut word));
        }
        if !separated {
            word.extend(letter.to_lowercase());
        }
        after_lower = letter.is_lowercase() || letter.is_ascii_digit();
    }
    if !word.is_empty() {
        found.push(word);
    }
    found
}

fn meaning(name: &str) -> Option<&'static str> {
    words(name)
        .iter()
        .find_map(|word| MEANINGS.iter().find(|meaning| *meaning == word))
        .copied()
}

/// Each function holding a meaning as a plain value, by that meaning and value: the function and
/// the first such parameter's name.
type Holders<'f> = BTreeMap<(&'static str, Scalar), BTreeMap<usize, &'f str>>;

fn holders(facts: &[FnFacts]) -> Holders<'_> {
    let mut found = Holders::new();
    for (index, function) in facts.iter().enumerate() {
        for parameter in &function.param_list {
            let (Some(scalar @ (Scalar::Text | Scalar::Integer)), Some(meaning)) =
                (parameter.scalar, meaning(&parameter.name))
            else {
                continue;
            };
            let functions: &mut BTreeMap<usize, &str> = found.entry((meaning, scalar)).or_default();
            functions.entry(index).or_insert(parameter.name.as_str());
        }
    }
    found
}

pub(super) fn add(facts: &[FnFacts], found: &mut [Vec<Diagnostic>]) {
    for ((meaning, scalar), functions) in holders(facts) {
        let actual = functions.len();
        if actual < MIN_FUNCTIONS {
            continue;
        }
        for (index, parameter) in functions {
            let message = format!(
                "parameter `{parameter}` holds `{meaning}` as plain {}, as {} other functions do; \
                 a named type would say what it is",
                scalar.name(),
                actual - 1,
            );
            let measure = Measure {
                actual,
                limit: MIN_FUNCTIONS - 1,
            };
            found[index].push(diagnostic(&facts[index], RULE, message, measure));
        }
    }
}

#[cfg(test)]
mod tests;
