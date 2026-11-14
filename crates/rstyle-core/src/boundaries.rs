//! `[[boundaries]]`: code under `from` may not call code under `deny`, as a domain that may not
//! reach its adapters. A boundary is drawn between files, which every frontend names the same way,
//! and read off the call graph, so a call that names its callee by full path breaks it as one
//! after a `use` does.

use crate::config::Boundary;
use crate::facts::{Edge, FnFacts};
use crate::report::Diagnostic;
use std::collections::HashMap;

pub const RULE: &str = "boundary";

/// Each function's calls across a boundary, by the function's index in `facts`. A function under
/// `deny` may call its own layer.
pub fn add(facts: &[FnFacts], boundaries: &[Boundary], found: &mut [Vec<Diagnostic>]) {
    let paths: HashMap<&str, &str> = facts
        .iter()
        .map(|f| (f.id.as_str(), f.path.as_str()))
        .collect();
    for boundary in boundaries {
        for (index, f) in facts.iter().enumerate() {
            if !boundary.is_from(&f.path) || boundary.is_denied(&f.path) {
                continue;
            }
            for edge in &f.edges {
                let Some(callee) = paths.get(edge.callee.as_str()) else {
                    continue;
                };
                if boundary.is_denied(callee) {
                    found[index].push(crossing(f, edge, callee, boundary));
                }
            }
        }
    }
}

fn crossing(f: &FnFacts, edge: &Edge, path: &str, boundary: &Boundary) -> Diagnostic {
    let name = edge.callee.rsplit("::").next().unwrap_or(&edge.callee);
    Diagnostic {
        path: f.path.clone(),
        line: edge.line,
        column: 1,
        rule_id: RULE.into(),
        message: format!(
            "function `{}` calls `{name}` in `{path}`, which code under {} may not call",
            f.name,
            boundary.from.join(", ")
        ),
        actual: 1,
        limit: 0,
        details: vec![],
        source: f.whole_lines(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::facts::sample;

    fn function(id: &str, path: &str, calls: &[&str]) -> FnFacts {
        let edge = |(at, callee): (usize, &&str)| Edge {
            callee: (*callee).into(),
            line: 2 + at,
            dynamic: false,
            required: false,
            args: vec![],
        };
        FnFacts {
            id: id.into(),
            path: path.into(),
            edges: calls.iter().enumerate().map(edge).collect(),
            ..sample(id.rsplit("::").next().unwrap())
        }
    }

    #[test]
    fn a_call_into_a_denied_layer_is_found_at_the_call_and_the_layer_may_call_itself() {
        let facts = [
            function(
                "app::review::rate",
                "src/review.rs",
                &["app::adapters::fetch", "app::review::pure"],
            ),
            function("app::review::pure", "src/review.rs", &[]),
            function(
                "app::adapters::fetch",
                "src/adapters/github.rs",
                &["app::adapters::send"],
            ),
            function("app::adapters::send", "src/adapters/http.rs", &[]),
        ];
        let boundary: Boundary =
            toml::from_str("from = [\"src/review.rs\"]\ndeny = [\"adapters\"]").unwrap();
        let mut found = vec![Vec::new(); facts.len()];
        add(&facts, &[boundary], &mut found);

        assert_eq!(found[0].len(), 1, "{found:?}");
        let crossing = &found[0][0];
        assert_eq!((crossing.rule_id.as_str(), crossing.line), (RULE, 2));
        assert!(
            crossing
                .message
                .contains("`fetch` in `src/adapters/github.rs`"),
            "{}",
            crossing.message
        );
        assert!(found[1..].iter().all(Vec::is_empty), "{found:?}");
    }

    #[test]
    fn a_boundary_needs_both_sides_and_only_them() {
        let read = |text: &str| {
            toml::from_str::<Boundary>(text)
                .map(|_| ())
                .map_err(|e| e.to_string())
        };
        assert!(
            read("from = [\"domain\"]\ndeny = []")
                .unwrap_err()
                .contains("`from` and `deny`")
        );
        assert!(read("from = [\"domain\"]\ndeny = [\"adapters\"]\nallow = []").is_err());
        assert!(read("from = [\"domain\"]\ndeny = [\"adapters\"]").is_ok());
    }
}
