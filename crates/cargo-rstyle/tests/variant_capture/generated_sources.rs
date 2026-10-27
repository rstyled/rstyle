//! Raw expansion and constructor identities never grant generated callable admission.
use super::float_kernels::function;
use super::*;
use rstyle_pdg::generated_sources::capture::{KEY, Observation, Outcome, UnitCapture};

const SOURCE: &str = r#"
#![no_std]
#[derive(Copy, Clone)]
pub struct Inner(u32);
#[derive(Clone, Debug)]
pub struct InnerError { attempted: u64 }
pub fn checked(value: usize) -> Result<Inner, InnerError> {
    if value > 100 { Err(InnerError { attempted: value as u64 }) }
    else { Ok(Inner(value as u32)) }
}
#[derive(Copy, Clone)]
pub struct First(Inner);
#[derive(Copy, Clone)]
pub struct Second(Inner);
macro_rules! identifiers {
    ($name:ident, $err:ident, $iter:ident, $withiter:ident) => {
        impl $name {
            pub fn new(value: usize) -> Result<$name, $err> {
                checked(value).map($name).map_err($err)
            }
            pub fn value(&self) -> u32 { self.0.0 }
        }
        #[derive(Clone, Debug)]
        pub struct $err(InnerError);
    };
}
identifiers!(First, FirstError, FirstIter, WithFirstIter);
identifiers!(Second, SecondError, SecondIter, WithSecondIter);
pub fn ordinary(value: u8) -> u16 { value as u16 }
pub fn function_callback(value: Option<u8>) -> Option<u16> { value.map(ordinary) }
pub enum Choice { Byte(u8) }
pub fn enum_callback(value: Option<u8>) -> Option<Choice> { value.map(Choice::Byte) }
pub struct Pair(u8, u16);
fn inspect<F>(_: F) {}
pub fn other_callbacks() {
    inspect(Pair);
    inspect(|byte: u8| byte);
    inspect(ordinary as fn(u8) -> u16);
}
"#;

#[test]
fn generated_source_diagnostics_distinguish_members_and_constructor_instances() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let output = diagnostic_process::output(
        Command::new(cargo_bin("cargo-rstyle"))
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-data-v3",
            ])
            .current_dir(directory.path())
            .env("RSTYLE_DRIVER", driver())
            .env("RSTYLE_DIAGNOSTIC_GENERATED_SOURCES", "1")
            .env_remove("CARGO_TARGET_DIR"),
    )
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    check_constructor_observations(&report);
    check_callback_observations(&report);
    check_raw_origins(&report);
}

fn check_constructor_observations(report: &Value) {
    for name in ["First::new", "Second::new"] {
        let entry = function(report, name);
        assert_eq!(
            entry["outcome"], "excluded",
            "raw observations cannot grant admission"
        );
        assert_eq!(entry["facts"]["source_scan_complete"], false);
        let facts: rstyle_pdg::HirFnFacts = serde_json::from_value(entry["facts"].clone()).unwrap();
        assert!(rstyle_pdg::producer::produce(&facts).is_err());
        let calls =
            entry["facts"]["build"]["frontend_options"]["rstyle.rust.call-instances"]["calls"]
                .as_array()
                .unwrap();
        for method in ["map", "map_err"] {
            let call = calls
                .iter()
                .find(|call| {
                    call["requested"]["path"]
                        .as_str()
                        .is_some_and(|path| path.ends_with(method))
                })
                .expect("selected core Result mapping call");
            assert_eq!(call["status"], "resolved", "{}", call["reason"]);
            let text = serde_json::to_string(&call["requested_arguments"]).unwrap();
            assert!(text.contains("tuple_constructor"));
            assert!(text.contains(name.split("::").next().unwrap()));
        }
    }
}

fn check_callback_observations(report: &Value) {
    let function_calls = function(report, "function_callback")["facts"]["build"]
        ["frontend_options"]["rstyle.rust.call-instances"]["calls"].as_array().unwrap();
    assert_eq!(function_calls.len(), 1);
    assert_eq!(function_calls[0]["status"], "resolved");
    assert!(
        function_calls[0]["requested_arguments"]
            .as_array()
            .unwrap()
            .iter()
            .any(|argument| argument["identity"]["kind"] == "function_item")
    );
    assert_eq!(function(report, "function_callback")["outcome"], "excluded");
    for (name, expected) in [
        ("enum_callback", &["unsupported"][..]),
        (
            "other_callbacks",
            &["unsupported", "resolved", "unsupported"][..],
        ),
    ] {
        let calls = function(report, name)["facts"]["build"]["frontend_options"]
            ["rstyle.rust.call-instances"]["calls"].as_array().unwrap();
        let statuses: Vec<_> = calls
            .iter()
            .map(|call| call["status"].as_str().unwrap())
            .collect();
        assert_eq!(statuses, expected, "{name}");
    }
}

fn raw_capture(report: &Value) -> UnitCapture {
    let records: Vec<_> = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|entry| entry["facts"]["build"]["frontend_options"].get(KEY))
        .collect();
    assert_eq!(records.len(), 1, "one diagnostic table per unit");
    serde_json::from_value(records[0].clone()).unwrap()
}

fn observed<'a>(captured: &'a UnitCapture, name: &str) -> &'a Observation {
    captured
        .outcomes
        .iter()
        .find_map(|outcome| match outcome {
            Outcome::Observed { observation } if observation.owner.path == name => {
                Some(observation.as_ref())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing complete raw observation for {name}"))
}

fn check_raw_origins(report: &Value) {
    let captured = raw_capture(report);
    assert!(captured.complete);
    let first = observed(&captured, "First::new");
    let second = observed(&captured, "Second::new");
    let getter = observed(&captured, "First::value");
    assert_ne!(
        first.owner.definition_identity,
        getter.owner.definition_identity
    );
    assert_eq!(first.defining_expansion, getter.defining_expansion);
    assert_ne!(first.defining_expansion, second.defining_expansion);
    assert_ne!(first.signature.raw.span, getter.signature.raw.span);
    assert!(!first.nodes.is_empty());
    for node in &first.nodes {
        assert!(node.origin.raw.digest.starts_with("xxh3-64:"));
        assert!(node.origin.raw.span.end as usize <= SOURCE.len());
    }
    assert!(
        captured
            .emitted
            .iter()
            .any(|item| item.definition.path == "FirstError")
    );
    assert!(
        captured
            .emitted
            .iter()
            .any(|item| item.definition.path == "SecondError")
    );
    assert!(captured.expansions.iter().any(|expansion| {
        expansion
            .macro_definition
            .as_ref()
            .is_some_and(|definition| definition.path == "identifiers")
    }));
}
