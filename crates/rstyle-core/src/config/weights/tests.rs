use crate::{config::Config, facts::EffectKind};

#[test]
fn source_weights_only_change_matching_paths_and_supplied_kinds() {
    let config = Config::parse(
        r#"
[[side_effects.overrides]]
paths = ["src/adapters/**"]
weights = { io = 1, network = 2 }
"#,
    )
    .unwrap();
    let effects = &config.side_effects;
    assert_eq!(
        effects.weight_for("src/adapters/files.rs", EffectKind::Io),
        1
    );
    assert_eq!(effects.weight_for("src/domain/rules.rs", EffectKind::Io), 3);
    assert_eq!(
        effects.weight_for("src/adapters_extra/files.rs", EffectKind::Io),
        3
    );
    assert_eq!(
        effects.weight_for("src/adapters/files.rs", EffectKind::Unsafe),
        4
    );
}

#[test]
fn later_matching_rules_override_only_their_supplied_weights() {
    let config = Config::parse(
        r#"
[[side_effects.overrides]]
paths = ["src/adapters/**"]
weights = { io = 1, network = 2 }
[[side_effects.overrides]]
paths = ["src/adapters/critical/**"]
weights = { io = 6 }
"#,
    )
    .unwrap();
    let effects = &config.side_effects;
    assert_eq!(
        effects.weight_for("src/adapters/critical/store.rs", EffectKind::Io),
        6
    );
    assert_eq!(
        effects.weight_for("src/adapters/critical/store.rs", EffectKind::Network),
        2
    );
    assert_eq!(
        effects.weight_for("src/adapters/store.rs", EffectKind::Io),
        1
    );
}

#[test]
fn invalid_source_override_paths_and_weight_names_are_rejected() {
    for input in [
        "[[side_effects.overrides]]\npaths = []\nweights = { io = 1 }",
        "[[side_effects.overrides]]\npaths = ['[']\nweights = { io = 1 }",
        "[[side_effects.overrides]]\npaths = ['src/**']\nweights = { ioo = 1 }",
        "[[side_effects.overrides]]\npath = ['src/**']\nweights = { io = 1 }",
    ] {
        assert!(Config::parse(input).is_err(), "accepted {input}");
    }
}
