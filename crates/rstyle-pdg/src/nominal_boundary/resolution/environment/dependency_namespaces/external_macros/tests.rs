use super::*;
use crate::{
    dependency_namespaces::capture::{Declaration, Export},
    generated_sources::capture::Definition,
    nominal_boundary::census::{Census, UnitFile, expansions::FileSpan},
};

const IMPORT: &str =
    "use powerfmt::smart_display::{self, FormatterOptions, Metadata, SmartDisplay};";
const ROOT: &str = include_str!("../../../selected/no_drop/external_expression_macros/lib.rs.txt");
const MODULE: &str =
    include_str!("../../../selected/no_drop/external_expression_macros/smart_display.rs.txt");

struct Fixture {
    environment: Environment,
    import: Import,
    census: Arc<Census>,
    namespaces: UnitCapture,
}

fn file(path: &str, text: &str) -> UnitFile {
    let bytes: crate::SourceSnapshot = text.into();
    UnitFile {
        path: path.into(),
        digest: bytes.checksum().into(),
        bytes,
    }
}

fn census(files: Vec<UnitFile>, name: &str) -> Arc<Census> {
    let mut census = (*crate::nominal_boundary::resolution::environment::tests::census()).clone();
    census.context.crate_name = name.into();
    census.context.root_source = files[0].path.clone();
    census.files = files;
    Arc::new(census)
}

fn location(file: &UnitFile, text: &str) -> FileSpan {
    let start = file.bytes.find(text).unwrap() as u32;
    FileSpan {
        path: file.path.clone(),
        digest: file.digest.clone(),
        span: ByteSpan {
            start,
            end: start + text.len() as u32,
        },
    }
}

fn local(path: &str) -> Definition {
    Definition {
        path: path.into(),
        definition_identity: format!("consumer:{path}"),
        stable_crate_id: "consumer".into(),
        crate_name: "consumer".into(),
        local: true,
    }
}

impl Fixture {
    fn new(source: &str) -> Self {
        let consumer = file("/sample/src/lib.rs", source);
        let environment = consumer_environment(&consumer);
        // Retained actual run-sQLgrr producer observation, with coordinates
        // rebased onto the same reviewed immutable buffers for this source test.
        let mut declaration: Declaration =
            serde_json::from_str(include_str!("producer-reexport.json")).unwrap();
        let producer_root = file("/original/powerfmt/src/lib.rs", ROOT);
        let producer_module = file("/original/powerfmt/src/smart_display.rs", MODULE);
        declaration.site.declaration.path = producer_module.path.clone();
        declaration.site.leaf.path = producer_module.path.clone();
        let mut module = declaration.site.owner.clone();
        module.local = false;
        module.path = "powerfmt::smart_display".into();
        let export: Export = serde_json::from_str(include_str!(
            "../../../selected/no_drop/external_expression_macros/tests/export.json"
        ))
        .unwrap();
        let import = Import {
            site: Site {
                definition: local("{use#0}::{use#0}"),
                owner: local(""),
                scope: vec![],
                path: ["powerfmt", "smart_display", "self"]
                    .map(String::from)
                    .to_vec(),
                binding: Binding::Named("self".into()),
                target: module,
                target_kind: "Mod".into(),
                public: false,
                declaration: location(&consumer, IMPORT),
                leaf: location(&consumer, "self"),
            },
            compiled_dependency: serde_json::json!({
                "identity":"rstyle.rust.compiled-crate/1", "crate_name":"powerfmt",
                "stable_crate_id": export.target.stable_crate_id,
            }),
            exports: vec![export],
            complete: false,
        };
        Self {
            environment,
            import,
            census: census(vec![producer_root, producer_module], "powerfmt"),
            namespaces: UnitCapture {
                identity: crate::dependency_namespaces::capture::IDENTITY.into(),
                imports: vec![],
                declarations: vec![declaration],
                unsupported: vec![],
            },
        }
    }

    fn receipt(&self) -> Result<OwnedMacro, String> {
        prove(
            &self.import,
            Arc::clone(&self.census),
            &self.namespaces,
            &[],
        )
    }

    fn install(&mut self) {
        let receipt = self.receipt().unwrap();
        self.environment
            .dependency_namespaces
            .external_macros
            .push(receipt);
    }

    fn facts(&self) -> HirFnFacts {
        let mut facts: HirFnFacts = serde_json::from_str(include_str!(
            "../../../selected/function_cleanup_tests/facts.json"
        ))
        .unwrap();
        facts.build.frontend_options.clear();
        facts.build.cfgs.clear();
        facts
            .build
            .frontend_options
            .insert_data(Arc::clone(&self.census));
        facts
            .build
            .frontend_options
            .insert_data(Arc::new(self.namespaces.clone()));
        facts.build.frontend_options.insert(
            "rstyle.rust.compiled-crate".into(),
            self.import.compiled_dependency.clone(),
        );
        facts
    }
}

fn invocation() -> syn::Macro {
    syn::parse_str::<syn::ExprMacro>(
        "smart_display::padded_width_of!(1, 2 => width({ struct Hidden; 3 }) fill('0'))",
    )
    .unwrap()
    .mac
}

#[test]
fn associated_source_receipt_returns_every_expression_for_the_original_module_binding() {
    let mut fixture = Fixture::new(IMPORT);
    assert!(producer(&fixture.import, &[fixture.facts()]).is_ok());
    let names = ["smart_display", "padded_width_of"].map(String::from);
    assert!(
        fixture
            .environment
            .external_macro_expressions(&[], &names, &invocation())
            .unwrap()
            .is_none()
    );
    fixture.install();
    assert_eq!(
        fixture
            .environment
            .external_macro_expressions(&[], &names, &invocation())
            .unwrap()
            .unwrap()
            .len(),
        4
    );
    assert!(
        fixture
            .environment
            .external_macro_expressions(&["other".into()], &names, &invocation())
            .unwrap()
            .is_none()
    );
    assert!(
        fixture
            .environment
            .external_macro_expressions(&[], &["other".into()], &invocation())
            .unwrap()
            .is_none()
    );
}

#[test]
fn producer_source_export_owner_and_route_mutations_grant_nothing() {
    let attacks: &[fn(&mut Fixture)] = &[
        |f| Arc::make_mut(&mut f.census).files[1].digest.push('x'),
        |f| f.import.site.target.definition_identity.push('x'),
        |f| f.import.site.target.stable_crate_id.push('x'),
        |f| f.import.exports[0].target.definition_identity.push('x'),
        |f| f.import.exports[0].target.path = "powerfmt::other".into(),
        |f| f.import.exports[0].target.local = true,
        |f| f.import.exports[0].public = false,
        |f| f.import.exports[0].route.clear(),
        |f| f.import.exports[0].route[0].declaration.local = true,
        |f| {
            f.import.exports[0].route[0].kind =
                crate::dependency_namespaces::capture::RouteKind::Glob
        },
        |f| {
            f.import.exports[0].route[0]
                .declaration
                .definition_identity
                .push('x')
        },
        |f| {
            f.namespaces.declarations[0]
                .site
                .owner
                .definition_identity
                .push('x')
        },
        |f| {
            f.namespaces.declarations[0].site.target.path =
                "other::__not_public_at_root__padded_width_of".into()
        },
        |f| f.namespaces.declarations[0].site.leaf.span.start += 1,
        |f| f.namespaces.declarations.clear(),
    ];
    for attack in attacks {
        let mut fixture = Fixture::new(IMPORT);
        attack(&mut fixture);
        assert!(fixture.receipt().is_err());
    }
}

#[test]
fn producer_build_identity_cfg_and_missing_evidence_are_barriers() {
    let fixture = Fixture::new(IMPORT);
    let mut facts = fixture.facts();
    facts.build.cfgs.push("doc".into());
    assert!(producer(&fixture.import, &[facts]).is_err());
    let mut facts = fixture.facts();
    facts.build.frontend_options.insert(
        "rstyle.rust.compiled-crate".into(),
        serde_json::json!({"identity":"rstyle.rust.compiled-crate/1",
            "crate_name":"powerfmt", "stable_crate_id":"wrong"}),
    );
    assert!(producer(&fixture.import, &[facts]).is_err());
    assert!(producer(&fixture.import, &[]).is_err());
}

#[test]
fn unknown_globs_and_original_module_or_external_root_conflicts_are_barriers() {
    for extra in [
        "mod smart_display {}",
        "mod powerfmt {}",
        "type smart_display = u32;",
        "type powerfmt = u32;",
        "use unknown::*;",
        "use powerfmt::*;",
    ] {
        let mut fixture = Fixture::new(&format!("{IMPORT} {extra}"));
        fixture.install();
        assert!(
            fixture
                .environment
                .external_macro_expressions(
                    &[],
                    &["smart_display".into(), "padded_width_of".into()],
                    &invocation(),
                )
                .is_err(),
            "{extra}"
        );
    }
}

#[test]
fn external_arguments_require_finite_token_depth_and_work_before_parsing() {
    let mut fixture = Fixture::new(IMPORT);
    fixture.install();
    let names = ["smart_display", "padded_width_of"].map(String::from);
    let mut mac = invocation();
    let mut deep = "1".parse::<proc_macro2::TokenStream>().unwrap();
    for _ in 0..49 {
        deep = proc_macro2::TokenTree::Group(proc_macro2::Group::new(
            proc_macro2::Delimiter::Parenthesis,
            deep,
        ))
        .into();
    }
    mac.tokens = deep;
    assert!(
        fixture
            .environment
            .external_macro_expressions(&[], &names, &mac)
            .err()
            .unwrap()
            .contains("nesting budget")
    );
    mac.tokens = std::iter::repeat_n(
        proc_macro2::TokenTree::Literal(proc_macro2::Literal::u8_unsuffixed(1)),
        131_073,
    )
    .collect();
    assert!(
        fixture
            .environment
            .external_macro_expressions(&[], &names, &mac)
            .err()
            .unwrap()
            .contains("work budget")
    );
}

#[test]
fn closed_local_glob_excludes_both_module_binding_and_external_root() {
    let mut fixture = Fixture::new(&format!(
        "{IMPORT} mod safe {{ pub struct Other; }} use crate::safe::*;"
    ));
    fixture.install();
    assert!(
        fixture
            .environment
            .external_macro_expressions(
                &[],
                &["smart_display".into(), "padded_width_of".into()],
                &invocation(),
            )
            .unwrap()
            .is_some()
    );
}

#[test]
fn original_glob_route_without_its_generated_receipt_is_not_evidence() {
    let fixture = Fixture::new(&format!(
        "{IMPORT} mod safe {{ pub struct Other; }} use crate::safe::*;"
    ));
    let environment = &fixture.environment;
    for key in ["safe", "*::safe", "*::crate::safe", "*::super::safe"] {
        assert!(
            !environment.external_generated_glob_clear(&[], key),
            "{key}"
        );
    }
    assert!(!environment.external_generated_glob_clear(&["other".into()], "*::crate::safe"));
}

fn consumer_environment(consumer: &UnitFile) -> Environment {
    Environment::with_options(
        census(vec![consumer.clone()], "consumer"),
        false,
        Some(&[]),
        false,
    )
    .unwrap()
}
