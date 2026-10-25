//! Real Cargo source-proof composition through exact checked dependency owners.
use super::*;
use rstyle_pdg::{dependency_namespaces, nominal_boundary::census::Census};
use std::sync::Arc;

const MEMBERS: &[&str] = &[
    "Value::new",
    "Value::get",
    "Value::roundtrip",
    "Value::same",
];

fn workspace_with_anonymous_prelude() -> TempDir {
    let directory = workspace();
    std::fs::write(
        directory.path().join("outside/src/lib.rs"),
        "pub trait Extend {}\npub trait Truncate {}\npub mod prelude { pub use crate::{Extend as _, Truncate as _}; }\npub fn selected_value(x: u8) -> u8 { x }\n",
    ).unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "use renamed::prelude::*;\npub fn local(x: u8) -> u8 { x }\n#[derive(Clone, Copy)] pub struct Value { pub value: u8 }\nimpl Value { pub fn new(value: u8) -> Self { Self { value } } pub fn get(&self) -> u8 { self.value } pub fn roundtrip(&self) -> Self { Self::new(self.get()) } pub fn same(&self) -> Self { Self::new(self.get()) } }\npub fn composed(value: u8) -> u8 { Value::new(value).get() }\n",
    )
    .unwrap();
    directory
}

fn proof(report: &Value) {
    let (consumer, _) = data_unit(named(report, "local"));
    let (producer, _) = data_unit(named(report, "selected_value"));
    let captured = dependency_namespaces::unit_capture(consumer.functions())
        .unwrap()
        .unwrap();
    let original = dependency_namespaces::unit_capture(producer.functions())
        .unwrap()
        .unwrap();
    let [import] = captured.imports.as_slice() else {
        panic!("one original module import required")
    };
    assert_eq!(import.site.path, ["renamed", "prelude"]);
    assert!(
        !import.complete,
        "foreign ambiguity is not compiler-observed"
    );
    assert_eq!(import.exports.len(), 2);
    let census = producer.functions()[0]
        .build
        .frontend_options
        .data::<Arc<Census>>()
        .unwrap();
    let receipt = dependency_namespaces::proof::verify_anonymous_prelude(
        Arc::clone(&census),
        &original,
        import,
    )
    .expect("exact original producer namespace proof");
    assert_eq!(
        receipt
            .traits()
            .iter()
            .map(|definition| definition.path.as_str())
            .collect::<Vec<_>>(),
        ["Extend", "Truncate"]
    );
    assert_eq!(receipt.import(), &import.site);
    let mut missing = import.clone();
    missing.exports.pop();
    missing.complete = true;
    assert!(
        dependency_namespaces::proof::verify_anonymous_prelude(
            Arc::clone(&census),
            &original,
            &missing
        )
        .is_err()
    );
}

#[test]
fn anonymous_dependency_exports_preserve_original_routes_through_cargo_and_strict_replay() {
    let directory = workspace_with_anonymous_prelude();
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--capture-dependency",
            "scalar_dependency",
        ],
    );
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        report
    );
    inventory(&report, 1, 2);
    admitted(&report);
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(
            Path::new(report["run_directory"].as_str().unwrap()).join("dependency-closure.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["associations"].as_array().unwrap().len(), 1);
    proof(&report);
    let (output, replayed) =
        replay::replay(directory.path(), report["run_directory"].as_str().unwrap());
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        replayed
    );
    assert_eq!(replayed["verification_policy"], "strict-replay");
    admitted(&replayed);
    proof(&replayed);
}

fn admitted(report: &Value) {
    for name in MEMBERS {
        let function = named(report, name);
        assert_eq!(function["outcome"], "admitted", "{function}");
    }
    assert_eq!(named(report, "composed")["outcome"], "excluded");
    for flag in [
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    let first = &named(report, "Value::roundtrip")["key"];
    let second = &named(report, "Value::same")["key"];
    assert!(
        report["matching"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|pair| {
                ((&pair["first"] == first && &pair["second"] == second)
                    || (&pair["second"] == first && &pair["first"] == second))
                    && pair["decision"] == "Accepted"
            })
    );
}

#[test]
fn anonymous_namespace_observations_require_a_selected_checked_dependency() {
    let directory = workspace_with_anonymous_prelude();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    for name in MEMBERS {
        let function = named(&report, name);
        assert_eq!(function["outcome"], "excluded", "{function}");
    }
}

#[test]
fn neighboring_and_grouped_unresolved_globs_retain_source_ambiguity() {
    for imported in [
        "use renamed::prelude::*; use renamed::neighbor::*;",
        "use renamed::{prelude::*, neighbor::*};",
    ] {
        let directory = workspace_with_anonymous_prelude();
        let source = directory.path().join("src/lib.rs");
        let original = std::fs::read_to_string(&source).unwrap();
        std::fs::write(
            &source,
            original.replace("use renamed::prelude::*;", imported),
        )
        .unwrap();
        let producer = directory.path().join("outside/src/lib.rs");
        let original = std::fs::read_to_string(&producer).unwrap();
        std::fs::write(
            &producer,
            format!("{original}\npub mod neighbor {{ pub use core::ops::Drop as Cleanup; }}"),
        )
        .unwrap();
        let (output, report) = clones(
            directory.path(),
            &[
                "--extraction-profile",
                "hir-data-v3",
                "--capture-dependency",
                "scalar_dependency",
            ],
        );
        assert!(output.status.success(), "{report}");
        for name in MEMBERS {
            let function = named(&report, name);
            assert_eq!(function["outcome"], "excluded", "{function}");
        }
    }
}

#[test]
fn widening_prelude_metadata_composes_through_cargo_and_strict_replay() {
    let directory = workspace_with_anonymous_prelude();
    let source = directory.path().join("outside/src/lib.rs");
    let original = std::fs::read_to_string(&source).unwrap();
    std::fs::write(&source, original
        .replace("pub trait Extend", "#[deprecated(since=\"0.2.2\",note=\"use Widen\")] #[allow(deprecated)] pub trait Extend")
        .replace("pub use crate", "#[allow(deprecated)] pub use crate")
        .replace("Truncate as _", "Truncate as _, Widen as _") + "pub trait Widen {}\n").unwrap();
    let (output, report) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--capture-dependency",
            "scalar_dependency",
        ],
    );
    assert!(output.status.success(), "{report}");
    admitted(&report);
    let (output, replayed) =
        replay::replay(directory.path(), report["run_directory"].as_str().unwrap());
    assert!(output.status.success(), "{replayed}");
    assert_eq!(replayed["verification_policy"], "strict-replay");
    admitted(&replayed);
}
