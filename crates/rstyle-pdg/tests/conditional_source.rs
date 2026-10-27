use rstyle_pdg::conditional_source::Configuration;
use syn::parse::Parser;

#[test]
fn cfg_predicates_use_the_exact_captured_compiler_set() {
    let values = vec![
        "feature=\"extra\"".into(),
        "unix".into(),
        "target_pointer_width=\"64\"".into(),
    ];
    let configuration = Configuration::new(&values);
    for (source, expected) in [
        ("all()", true),
        ("any()", false),
        ("unix", true),
        ("r#unix", true),
        ("windows", false),
        ("feature = \"extra\"", true),
        ("r#feature = \"extra\"", true),
        ("feature = \"other\"", false),
        ("target_pointer_width = \"64\"", true),
        ("all(unix, any(windows, feature = \"extra\"))", true),
        ("not(all(unix, feature = \"extra\"))", false),
    ] {
        assert_eq!(
            configuration
                .predicate(&syn::parse_str(source).unwrap())
                .unwrap(),
            expected,
            "{source}"
        );
    }
    for source in [
        "not()",
        "not(unix, windows)",
        "version(\"1.90\")",
        "feature = 1",
        "custom(unix)",
    ] {
        assert!(
            configuration
                .predicate(&syn::parse_str(source).unwrap())
                .is_err(),
            "{source}"
        );
    }
}

#[test]
fn nested_cfg_attributes_only_remove_source_when_a_guard_is_proven_false() {
    let values = vec!["feature=\"extra\"".into(), "unix".into()];
    let configuration = Configuration::new(&values);
    for (source, expected) in [
        ("#[cfg(unix)]", true),
        ("#[cfg_attr(feature = \"extra\", cfg(not(unix)))]", false),
        ("#[cfg_attr(any(), cfg(any()))]", true),
        ("#[cfg_attr(all(), cfg_attr(all(), cfg(any())))]", false),
        ("#[cfg(any())] #[cfg(version(\"1.90\"))]", false),
        ("#[cfg(version(\"1.90\"))] #[cfg(any())]", false),
        ("#[inline] #[cfg_attr(unix, allow(unused))]", true),
    ] {
        let attributes = syn::Attribute::parse_outer.parse_str(source).unwrap();
        assert_eq!(
            configuration.active(&attributes).unwrap(),
            expected,
            "{source}"
        );
    }
    let attributes = syn::Attribute::parse_outer
        .parse_str("#[cfg(version(\"1.90\"))]")
        .unwrap();
    assert!(configuration.active(&attributes).is_err());
}
