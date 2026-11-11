use crate::config::Config;

const LAYERED: &str = r#"
[side_effects.weights]
network = 4

[[overrides]]
paths = ["**/adapters/**"]
strict_style.fn_length = { max_lines = 80 }
side_effects.weights = { io = 1, network = 2 }

[[overrides]]
paths = ["**/adapters/critical/**"]
side_effects.weights = { io = 6 }
"#;

#[test]
fn unmatched_paths_borrow_the_base() {
    let config = Config::parse(LAYERED).unwrap();
    let policy = config.for_path("src/domain/rules.rs");
    assert!(matches!(policy, std::borrow::Cow::Borrowed(_)));
    assert_eq!(policy.side_effects.weights.network, 4);
    assert_eq!(policy.strict_style.fn_length.max_lines, 50);
}

#[test]
fn matching_overrides_merge_in_order_over_the_base() {
    let config = Config::parse(LAYERED).unwrap();
    let adapter = config.for_path("src/adapters/store.rs");
    assert_eq!(adapter.side_effects.weights.io, 1);
    assert_eq!(adapter.side_effects.weights.network, 2);
    assert_eq!(adapter.side_effects.weights.r#unsafe, 4);
    assert_eq!(adapter.strict_style.fn_length.max_lines, 80);

    let critical = config.for_path("src/adapters/critical/store.rs");
    assert_eq!(critical.side_effects.weights.io, 6);
    assert_eq!(critical.side_effects.weights.network, 2);
    assert_eq!(critical.strict_style.fn_length.max_lines, 80);
}

#[test]
fn invalid_overrides_are_rejected() {
    for input in [
        "[[overrides]]\nstrict_style.fn_length = { max_lines = 1 }",
        "[[overrides]]\npaths = []\nstrict_style.fn_length = { max_lines = 1 }",
        "[[overrides]]\npaths = ['[']\nstrict_style.fn_length = { max_lines = 1 }",
        "[[overrides]]\npaths = ['src/**']\nside_effects.weights = { ioo = 1 }",
        "[[overrides]]\npaths = ['src/**']\nstrict_style.fn_length = { max_lines = 'x' }",
        "[[overrides]]\npaths = ['src/**']\nexclude = ['x']",
        "[[overrides]]\npaths = ['src/**']\ngrouping = { caller_hops = 2 }",
        "[[overrides]]\npaths = ['src/**']\nside_effects = { transitive_depth = 0 }",
        "[[overrides]]\npaths = ['src/**']\nside_effects.paths = { io = ['x::'] }",
        "[[overrides]]\npaths = ['src/**']\nstrict_style.value_object = { enabled = true }",
        "[[overrides]]\npaths = ['src/**']\nstrict_style.fn_spacing = { min = 3 }",
    ] {
        assert!(Config::parse(input).is_err(), "accepted {input}");
    }
}

#[test]
fn plain_names_match_a_module_anywhere() {
    let config = Config::parse(
        "[[overrides]]\npaths = ['adapters', 'bin/main.rs']\nstrict_style.fn_length = { max_lines = 80 }",
    )
    .unwrap();
    let max_lines = |path| config.for_path(path).strict_style.fn_length.max_lines;
    assert_eq!(max_lines("adapters/store.rs"), 80);
    assert_eq!(max_lines("crates/app/src/adapters/db/store.rs"), 80);
    assert_eq!(max_lines("crates/app/src/adapters.rs"), 80);
    assert_eq!(max_lines("crates/app/src/bin/main.rs"), 80);
    assert_eq!(max_lines("crates/app/src/adapters_extra/store.rs"), 50);
    assert_eq!(max_lines("crates/app/src/my_adapters.rs"), 50);
    assert_eq!(max_lines("crates/app/src/main.rs"), 50);
}

#[test]
fn slashes_around_a_plain_name_are_ignored() {
    for name in ["adapters/", "/adapters", "/adapters/"] {
        let input = format!(
            "[[overrides]]\npaths = ['{name}']\nstrict_style.fn_length = {{ max_lines = 80 }}"
        );
        let config = Config::parse(&input).unwrap();
        let max_lines = |path| config.for_path(path).strict_style.fn_length.max_lines;
        assert_eq!(max_lines("src/adapters/store.rs"), 80, "{name}");
        assert_eq!(max_lines("src/adapters.rs"), 80, "{name}");
        assert_eq!(max_lines("src/domain/rules.rs"), 50, "{name}");
    }
}

#[test]
fn overrides_serialize_back_to_their_table_and_only_when_present() {
    let plain = toml::to_string(&Config::default()).unwrap();
    assert!(!plain.contains("overrides"));

    let config = Config::parse(LAYERED).unwrap();
    let round_trip = Config::parse(&toml::to_string(&config).unwrap()).unwrap();
    let policy = round_trip.for_path("src/adapters/critical/store.rs");
    assert_eq!(policy.side_effects.weights.io, 6);
    assert_eq!(policy.strict_style.fn_length.max_lines, 80);
}

#[test]
fn per_function_strict_style_rules_can_be_turned_on_or_off_by_path() {
    let config = Config::parse(
        "[strict_style.fn_spacing]\nenabled = true\n\n\
         [[overrides]]\npaths = ['generated']\n\
         strict_style.fn_spacing = { enabled = false }\n\
         strict_style.fn_params = { enabled = false }\n\n\
         [[overrides]]\npaths = ['domain']\nstrict_style.fn_spacing = { min_statements = 3 }",
    )
    .unwrap();
    let spacing = |path| config.for_path(path).strict_style.fn_spacing.clone();
    assert!(!spacing("src/generated/api.rs").enabled);
    assert!(spacing("src/domain/order.rs").enabled);
    assert_eq!(spacing("src/domain/order.rs").min_statements, 3);
    assert_eq!(spacing("src/main.rs").min_statements, 5);

    let params = |path| config.for_path(path).strict_style.fn_params.enabled;
    assert!(!params("src/generated/api.rs"));
    assert!(params("src/domain/order.rs"));
}
