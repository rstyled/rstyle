//! Equivalence of derived rule facts with a frontend's own facts for the same
//! functions. Event lists are compared as multisets: their order is not part of
//! the rules' input.

use crate::facts::FnFacts;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Difference {
    pub function: String,
    pub field: &'static str,
    pub reference: String,
    pub derived: String,
}

#[derive(Debug, Default, Serialize)]
pub struct Comparison {
    pub functions: usize,
    pub differences: Vec<Difference>,
}

impl Comparison {
    /// Differing functions per field.
    pub fn by_field(&self) -> BTreeMap<&'static str, usize> {
        let mut counts = BTreeMap::new();
        for difference in &self.differences {
            *counts.entry(difference.field).or_default() += 1;
        }
        counts
    }

    pub fn extend(&mut self, other: Comparison) {
        self.functions += other.functions;
        self.differences.extend(other.differences);
    }
}

/// Compares facts paired by function ID; a function present on one side only differs.
pub fn compare(reference: &[FnFacts], derived: &[FnFacts]) -> Comparison {
    let derived: BTreeMap<&str, &FnFacts> = derived
        .iter()
        .map(|facts| (facts.id.as_str(), facts))
        .collect();
    let mut comparison = Comparison {
        functions: reference.len(),
        differences: vec![],
    };
    for expected in reference {
        let difference = |field, reference: String, derived: String| Difference {
            function: expected.id.clone(),
            field,
            reference,
            derived,
        };
        let Some(actual) = derived.get(expected.id.as_str()) else {
            comparison
                .differences
                .push(difference("function", "present".into(), "absent".into()));
            continue;
        };
        for (field, reference, derived) in fields(expected)
            .into_iter()
            .zip(fields(actual))
            .map(|(a, b)| (a.0, a.1, b.1))
        {
            if reference != derived {
                comparison
                    .differences
                    .push(difference(field, reference, derived));
            }
        }
    }
    comparison
}

fn fields(facts: &FnFacts) -> Vec<(&'static str, String)> {
    let complexity = facts.complexity.as_ref().map(|measured| {
        let mut points: Vec<_> = measured
            .contributions
            .iter()
            .map(|p| format!("{p:?}"))
            .collect();
        points.sort();
        (measured.cyclomatic, measured.cognitive, points)
    });
    vec![
        ("implements", format!("{:?}", facts.implements)),
        ("path", facts.path.clone()),
        ("location", format!("{}:{}", facts.line, facts.column)),
        ("name", facts.name.clone()),
        (
            "parameters",
            format!("{} self={}", facts.params, facts.has_self),
        ),
        ("statements", facts.statements.to_string()),
        ("top_statements", facts.top_statements.to_string()),
        ("complexity", format!("{complexity:?}")),
        ("source", facts.source.clone()),
        ("preceding_line", facts.preceding_line.clone()),
        ("mutations", sorted(&facts.mutations)),
        ("calls", sorted(&facts.calls)),
        ("effects", sorted(&facts.effects)),
        ("edges", sorted(&facts.edges)),
        ("param_list", format!("{:?}", facts.param_list)),
        ("mutated_types", format!("{:?}", facts.mutated_types)),
        ("sig", format!("{:?}", facts.sig)),
        ("pdg", format!("{:?}", facts.pdg)),
    ]
}

fn sorted<T: std::fmt::Debug>(events: &[T]) -> String {
    let mut rendered: Vec<String> = events.iter().map(|event| format!("{event:?}")).collect();
    rendered.sort();
    rendered.join("\n")
}

/// Runtime-bound inputs the frontend lowered, against those derived from its document.
/// Local and loop numbers are opaque identities, so both sides are compared after
/// renumbering them in order of appearance.
pub fn compare_bounds(
    reference: &serde_json::Value,
    derived: &[(String, crate::advisory::runtime_bounds::Input)],
) -> Vec<Difference> {
    let mut differences = Vec::new();
    for (function, input) in derived {
        let expected = reference.get(function).map(canonical_bounds);
        let actual = serde_json::to_value(input).map(|value| canonical_bounds(&value));
        let (expected, actual) = match (expected, actual) {
            (Some(expected), Ok(actual)) => (expected, actual),
            (None, _) => ("absent".into(), "present".into()),
            (_, Err(error)) => ("present".into(), error.to_string()),
        };
        if expected != actual {
            differences.push(Difference {
                function: function.clone(),
                field: "runtime_bounds",
                reference: expected,
                derived: actual,
            });
        }
    }
    differences
}

/// The input's JSON with local and loop numbers renumbered by first appearance.
pub fn canonical_bounds(value: &serde_json::Value) -> String {
    let text = value.to_string();
    // Extent labels end `len(...)#N`; `#` alone also occurs in paths such as `{impl#2}`.
    let markers = ["\"Local\":", ")#", "for-index-", "for-limit-"];
    let mut numbers: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    while !rest.is_empty() {
        let found = markers
            .iter()
            .filter_map(|marker| rest.find(marker).map(|at| (at, *marker)))
            .min();
        let Some((at, marker)) = found else {
            out.push_str(rest);
            break;
        };
        let after = at + marker.len();
        out.push_str(&rest[..after]);
        let digits = rest[after..].bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            rest = &rest[after..];
            continue;
        }
        let number = &rest[after..after + digits];
        let next = numbers.len();
        let canonical = *numbers.entry(number.to_string()).or_insert(next);
        out.push_str(&canonical.to_string());
        rest = &rest[after + digits..];
    }

    out
}
