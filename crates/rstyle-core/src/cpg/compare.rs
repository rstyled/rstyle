//! Equivalence of derived rule facts with a frontend's own facts for the same
//! functions. Event lists are compared as multisets: their order is not part of
//! the rules' input.

use crate::facts::FnFacts;
use serde::Serialize;
use std::collections::BTreeMap;

/// Facts the rule derivation does not produce yet, so they are not compared.
pub const NOT_DERIVED: [&str; 4] = ["param_list", "mutated_types", "sig", "pdg"];

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
    ]
}

fn sorted<T: std::fmt::Debug>(events: &[T]) -> String {
    let mut rendered: Vec<String> = events.iter().map(|event| format!("{event:?}")).collect();
    rendered.sort();
    rendered.join("\n")
}
