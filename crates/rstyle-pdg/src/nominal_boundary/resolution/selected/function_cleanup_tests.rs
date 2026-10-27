//! Source-boundary kernels; these synthetic facts do not certify extraction.
use super::*;
use crate::concrete_types::{Field, MixedVariant, Nominal, VariantShape};

const ERROR: &str = "pub enum Error { Missing, Size { needed: usize } }";
const SOURCE: &str = r#"
mod block {
    mod compress { pub fn compress() {} }
    mod decompress_safe { pub fn decompress() {} }
    use decompress_safe as decompress;
    pub use compress::*;
    pub use decompress::*;
    pub enum Error { Missing, Size { needed: usize } }
    fn f() { unknown!(); }
}
"#;

fn environment(source: &str) -> Environment {
    let mut census = (*super::super::environment::tests::census()).clone();
    census.files[0].bytes = source.into();
    census.files[0].digest = concrete_types::digest(source.as_bytes());
    Environment::with_options(std::sync::Arc::new(census), true, Some(&[]), false).unwrap()
}

fn error(environment: &Environment) -> TypeTree {
    let package = &environment.census().context.package;
    TypeTree::MixedEnum {
        nominal: Nominal {
            source_identity: Some(package.package_id.clone()),
            package_name: package.name.clone(),
            package_version: package.version.clone(),
            manifest_digest: package.manifest_digest.clone(),
            definition_path: "block::Error".into(),
            declaration_digest: concrete_types::digest(ERROR.as_bytes()),
        },
        source: ERROR.into(),
        variants: vec![
            MixedVariant {
                position: 0,
                name: "Missing".into(),
                shape: VariantShape::Unit {},
            },
            MixedVariant {
                position: 1,
                name: "Size".into(),
                shape: VariantShape::Named {
                    fields: vec![(
                        "needed".into(),
                        Field {
                            position: 0,
                            name: "needed".into(),
                            declaration: "usize".into(),
                            ty: TypeTree::Scalar("usize".into()),
                        },
                    )],
                },
            },
        ],
    }
}

fn facts(environment: &Environment) -> HirFnFacts {
    let mut facts: HirFnFacts =
        serde_json::from_str(include_str!("function_cleanup_tests/facts.json")).unwrap();
    let result = TypeTree::Result {
        ok: Box::new(TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(
            TypeTree::Scalar("u8".into()),
        ))))),
        err: Box::new(error(environment)),
    };
    let observations = InterfaceObservations {
        identity: concrete_types::IDENTITY.into(),
        parameters: vec![],
        result: Observation::Captured { ty: result.clone() },
    };
    facts.build.frontend_options.insert(
        concrete_types::KEY.into(),
        serde_json::to_value(observations).unwrap(),
    );
    facts.signature.return_ty.normalized = Some(crate::data_types::token(&result).unwrap());
    facts
}

#[test]
fn function_scope_reuses_only_checked_whole_unit_cleanup_certificate() {
    let safe = SOURCE.replace("fn f() { unknown!(); }", "fn f() {} ");
    let environment = environment(&safe);
    let mut facts = facts(&environment);
    let namespace = vec!["block".into()];
    assert!(check_namespace_with_closure(&facts, &environment, &namespace, None).is_err());
    check_namespace(&facts, &environment, &namespace).unwrap();
    facts
        .build
        .frontend_options
        .insert("extraction_profile".into(), "hir-data-v2".into());
    assert!(check_namespace(&facts, &environment, &namespace).is_err());
}

#[test]
fn certificate_recovery_keeps_unknown_expansions_and_destructors_closed() {
    for source in [
        SOURCE.to_string(),
        SOURCE.replace(
            "fn f() { unknown!(); }",
            "impl Drop for Error { fn drop(&mut self) {} }",
        ),
        SOURCE.replace(
            "fn f() { unknown!(); }",
            "use external::Drop as Cleanup; impl Cleanup for Error { fn drop(&mut self) {} }",
        ),
        SOURCE.replace(
            "mod compress { pub fn compress() {} }",
            "mod compress { pub use external::Debug; }",
        ),
    ] {
        let environment = environment(&source);
        assert!(check_namespace(&facts(&environment), &environment, &["block".into()]).is_err());
    }
}

#[test]
fn original_lexical_glob_targets_preserve_alias_and_requested_name_evidence() {
    let safe = SOURCE.replace("fn f() { unknown!(); }", "fn f() {} ");
    let environment = environment(&safe);
    let mut facts = facts(&environment);
    for prefix in ["compress", "decompress"] {
        for name in ["Result", "Ok", "Err", "core", "std"] {
            assert!(
                lexical_glob_excludes(
                    &facts,
                    &environment,
                    LexicalGlob {
                        namespace: &vec!["block".into()],
                        path: &[prefix.into()]
                    },
                    name,
                )
                .unwrap()
            );
        }
    }
    facts
        .build
        .frontend_options
        .insert("extraction_profile".into(), "hir-data-v2".into());
    assert!(
        !lexical_glob_excludes(
            &facts,
            &environment,
            LexicalGlob {
                namespace: &vec!["block".into()],
                path: &["compress".into()]
            },
            "Result"
        )
        .unwrap()
    );
}

#[test]
fn lexical_globs_retain_unknown_roots_aliases_and_exact_name_collisions() {
    let safe = SOURCE.replace("fn f() { unknown!(); }", "fn f() {} ");
    for changed in [
        safe.replace("pub fn compress() {}", "pub fn Result() {}"),
        safe.replace("pub fn compress() {}", "pub struct Result;"),
        safe.replace("pub fn compress() {}", "struct Result;"),
        safe.replace(
            "use decompress_safe as decompress;",
            "use external::decompress_safe as compress;",
        ),
        safe.replace("pub use compress::*;", "pub use ::external::*;"),
        safe.replace(
            "mod compress { pub fn compress() {} }",
            "mod compress { pub use external::*; }",
        ),
    ] {
        let environment = environment(&changed);
        assert!(
            lexical_glob_excludes(
                &facts(&environment),
                &environment,
                LexicalGlob {
                    namespace: &vec!["block".into()],
                    path: &["compress".into()]
                },
                "Result"
            )
            .is_err(),
            "{changed}"
        );
    }
}

#[test]
fn unrelated_option_functions_keep_historical_glob_route_applicable() {
    let safe = SOURCE.replace("fn f() { unknown!(); }", "fn f() {} ");
    let environment = environment(&safe);
    let mut facts = facts(&environment);
    let observations = InterfaceObservations {
        identity: concrete_types::IDENTITY.into(),
        parameters: vec![],
        result: Observation::Captured {
            ty: TypeTree::Option(Box::new(TypeTree::Scalar("u8".into()))),
        },
    };
    facts.build.frontend_options.insert(
        concrete_types::KEY.into(),
        serde_json::to_value(observations).unwrap(),
    );
    imports::glob_excludes(
        &environment,
        &["block".into(), "compress".into()],
        "Option",
        &crate::conditional_source::Configuration::new(&facts.build.cfgs),
    )
    .unwrap();
    assert!(
        !lexical_glob_excludes(
            &facts,
            &environment,
            LexicalGlob {
                namespace: &vec!["block".into()],
                path: &["compress".into()]
            },
            "Option"
        )
        .unwrap()
    );
}
