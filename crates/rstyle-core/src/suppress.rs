//! `// rstyle-allow(rule_a, go:rule_b): reason` comments above a function. A rule named as
//! `--filter` names a frontend's analysis belongs to that frontend, which reads the comment
//! itself, so core can't call it unknown.

use crate::filter::valid_name;

pub const RULE_IDS: &[&str] = &[
    "fn_length",
    "fn_params",
    "fn_spacing",
    "cyclomatic_complexity",
    "cognitive_complexity",
    "mutation_per_target",
    "mutated_targets",
    "side_effect_density",
    "abstraction",
    "dead_store",
    "ignored_result",
    "deviant_argument",
    "taint_flow",
    "algorithmic_complexity",
    "guard_clause",
    "primitive_obsession",
    "value_object",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Suppression {
    None,
    Allow(Vec<String>),
    Invalid(String),
}

const PREFIX: &str = "// rstyle-allow(";

pub fn parse(comment_line: &str) -> Suppression {
    let Some(rest) = comment_line.trim().strip_prefix(PREFIX) else {
        return Suppression::None;
    };
    let Some((rules, tail)) = rest.split_once(')') else {
        return Suppression::Invalid("malformed rstyle-allow comment".into());
    };
    let reason = tail.strip_prefix(':').map_or("", str::trim);

    if reason.is_empty() {
        return Suppression::Invalid("rstyle-allow requires a reason after `:`".into());
    }
    let names: Vec<String> = rules.split(',').map(|r| r.trim().to_string()).collect();
    match names.iter().find(|name| !known(name)) {
        Some(unknown) => Suppression::Invalid(format!("unknown rule `{unknown}` in rstyle-allow")),
        None => Suppression::Allow(names),
    }
}

fn known(name: &str) -> bool {
    RULE_IDS.contains(&name)
        || name
            .split_once(':')
            .is_some_and(|(frontend, rule)| valid_name(frontend) && valid_name(rule))
}

/// True when the comment is a valid suppression that names `rule`.
pub fn allows(comment_line: &str, rule: &str) -> bool {
    matches!(parse(comment_line), Suppression::Allow(rules) if rules.iter().any(|r| r == rule))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_comments_are_not_suppressions() {
        assert_eq!(parse("// just a note"), Suppression::None);
        assert_eq!(parse(""), Suppression::None);
    }

    #[test]
    fn allows_named_rules_with_reason() {
        assert_eq!(
            parse("// rstyle-allow(fn_params): ffi shim"),
            Suppression::Allow(vec!["fn_params".into()])
        );
        assert_eq!(
            parse("  // rstyle-allow(fn_params, fn_length): generated"),
            Suppression::Allow(vec!["fn_params".into(), "fn_length".into()])
        );
    }

    #[test]
    fn abstraction_is_a_known_rule_for_the_patterns_miner() {
        assert!(allows(
            "// rstyle-allow(abstraction): deliberately parallel",
            "abstraction"
        ));
        assert!(!allows("// rstyle-allow(abstraction): why", "fn_length"));
    }

    #[test]
    fn a_frontends_rule_is_that_frontends_to_judge() {
        assert_eq!(
            parse("// rstyle-allow(go:nilerr, fn_length): deliberate"),
            Suppression::Allow(vec!["go:nilerr".into(), "fn_length".into()])
        );
        assert!(allows("// rstyle-allow(go:nilerr): why", "go:nilerr"));
        assert!(!allows("// rstyle-allow(go:nilerr): why", "fn_length"));
        for name in ["go:", ":nilerr", "go:a:b", "go:../x"] {
            let comment = format!("// rstyle-allow({name}): why");
            assert!(matches!(parse(&comment), Suppression::Invalid(_)), "{name}");
        }
    }

    #[test]
    fn missing_reason_is_invalid() {
        assert!(matches!(
            parse("// rstyle-allow(fn_params)"),
            Suppression::Invalid(_)
        ));
        assert!(matches!(
            parse("// rstyle-allow(fn_params):  "),
            Suppression::Invalid(_)
        ));
    }

    #[test]
    fn unknown_rule_is_invalid() {
        assert!(matches!(
            parse("// rstyle-allow(nope): why"),
            Suppression::Invalid(_)
        ));
    }
}
