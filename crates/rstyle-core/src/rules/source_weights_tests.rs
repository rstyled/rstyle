use super::*;
use crate::facts::{CallEvent, EffectEvent, sample};

fn writing_function(path: &str) -> FnFacts {
    FnFacts {
        path: path.into(),
        source: format!("fn write() {{\n{}}}", "write();\n".repeat(10)),
        statements: 4,
        calls: vec![CallEvent {
            callee: "std::fs::write".into(),
            line: 2,
            dynamic: false,
        }],
        ..sample("write")
    }
}

#[test]
fn scoped_weights_can_lower_and_raise_density_without_relaxing_other_paths() {
    let config = Config::parse(
        r#"
[[side_effects.overrides]]
paths = ["src/adapters/**"]
weights = { io = 1 }
[[side_effects.overrides]]
paths = ["src/adapters/critical/**"]
weights = { io = 5 }
"#,
    )
    .unwrap();
    for (path, density, findings) in [
        ("src/adapters/store.rs", 250, 0),
        ("src/domain/rules.rs", 750, 1),
        ("src/application/check.rs", 750, 1),
        ("src/adapters/critical/store.rs", 1250, 1),
    ] {
        let facts = writing_function(path);
        let effects = effects_of(&facts, &config);
        assert_eq!(density_milli(&facts, &effects, &config), density, "{path}");
        assert_eq!(
            side_effect_density(&facts, &effects, &config).len(),
            findings,
            "{path}"
        );
    }
}

#[test]
fn zero_io_weight_keeps_unsafe_findings_and_reports_only_weighted_details() {
    let config = Config::parse(
        r#"
[[side_effects.overrides]]
paths = ["src/adapters/**"]
weights = { io = 0 }
"#,
    )
    .unwrap();
    let mut facts = writing_function("src/adapters/store.rs");
    facts.effects = vec![EffectEvent {
        kind: EffectKind::Unsafe,
        detail: "unsafe block".into(),
        line: 3,
    }];
    let effects = effects_of(&facts, &config);
    let findings = side_effect_density(&facts, &effects, &config);
    assert_eq!(findings[0].actual, 1000);
    assert_eq!(findings[0].details.len(), 1);
    assert_eq!(findings[0].details[0].label, "unsafe: unsafe block");
}
