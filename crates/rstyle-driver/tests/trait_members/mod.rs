//! Real compiler owning-member associations, independently specified before capture.
use super::*;
use serde_json::Value;
mod generated;

const KEY: &str = "rstyle.rust.concrete-conversion-member";
const SOURCE: &str = r#"
#[derive(Clone,Copy)] pub struct Payload { name: &'static str, flag: bool }
#[derive(Clone,Copy)] pub enum Wrapped { Empty, Value(Payload) }
pub struct ErrorA;
pub struct ErrorB;
impl From<Payload> for Wrapped { fn from(v: Payload) -> Self { Self::Value(v) } }
impl TryFrom<Wrapped> for Payload {
    type Error = ErrorA;
    fn try_from(v: Wrapped) -> Result<Self, Self::Error> {
        match v { Wrapped::Value(v) => Ok(v), _ => Err(ErrorA) }
    }
}
impl TryFrom<Wrapped> for bool {
    type Error = ErrorB;
    fn try_from(v: Wrapped) -> Result<Self, Self::Error> {
        match v { Wrapped::Value(v) => Ok(v.flag), _ => Err(ErrorB) }
    }
}
pub struct Generic<T>(T);
impl<T> From<T> for Generic<T> { fn from(v: T) -> Self { Self(v) } }
mod lookalike {
    pub trait From<T> { fn from(v: T) -> Self; }
    pub struct Local;
    impl From<u8> for Local { fn from(_: u8) -> Self { Self } }
}
pub fn ordinary(v: u8) -> u8 { v }
"#;

fn captured(profile: &str) -> Vec<HirFnFacts> {
    captured_source(profile, SOURCE)
}

fn captured_source(profile: &str, source: &str) -> Vec<HirFnFacts> {
    let directory = directory();
    let input = directory.join("members.rs");
    let facts = directory.join("facts.json");
    std::fs::write(&input, source).unwrap();
    let output = selected_invocation(&input, &facts, Some(profile))
        .env("RSTYLE_DIAGNOSTIC_CAPTURE_ONLY", "1")
        .env("RSTYLE_DIAGNOSTIC_FACTS_DIR", directory.join("snapshots"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let raw = snapshot(&directory);
    let functions = serde_json::from_value(raw["facts"].clone()).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    functions
}

fn snapshot(directory: &std::path::Path) -> Value {
    let paths: Vec<_> = std::fs::read_dir(directory.join("snapshots"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.to_string_lossy()
                .ends_with("-pre-helper-certification.json")
        })
        .collect();
    assert_eq!(paths.len(), 1);
    serde_json::from_slice(&std::fs::read(&paths[0]).unwrap()).unwrap()
}

fn observations(functions: &[HirFnFacts]) -> Vec<&Value> {
    functions
        .iter()
        .filter_map(|function| function.build.frontend_options.get(KEY))
        .filter(|value| value["status"] == "observed")
        .map(|value| &value["observation"])
        .collect()
}

fn source_range(source: &Value) -> &str {
    let start = source["span"]["start"].as_u64().unwrap() as usize;
    let end = source["span"]["end"].as_u64().unwrap() as usize;
    &SOURCE[start..end]
}

#[test]
fn concrete_members_retain_exact_core_owner_signatures_and_associated_error() {
    let functions = captured("hir-data-v3");
    let observed = observations(&functions);
    assert_eq!(observed.len(), 3);
    for member in &observed {
        assert_eq!(member["admission_authority"], false);
        assert_eq!(member["method"], member["selected"]);
        assert_eq!(member["requested_signature"], member["selected_signature"]);
        assert_eq!(member["trait_definition"]["crate"], "core");
        assert_eq!(member["implementation"]["local"], true);
        assert!(source_range(&member["implementation_source"]).starts_with("impl "));
        assert!(source_range(&member["method_source"]).starts_with("fn "));
        assert!(
            source_range(&member["implementation_source"])
                .contains(source_range(&member["method_source"]))
        );
        assert!(source_range(&member["implementation_source"]).ends_with('}'));
        assert_eq!(member["method_header"]["is_const"], false);
        assert_eq!(member["method_header"]["is_async"], false);
        assert_eq!(member["selected_signature"]["variadic"], false);
        if !member["associated_error"].is_null() {
            assert!(
                source_range(&member["implementation_source"])
                    .contains(source_range(&member["associated_error"]["source"]))
            );
            assert_eq!(member["members"].as_array().unwrap().len(), 2);
        }
    }
    check_errors(&observed);
}

fn check_errors(observed: &[&Value]) {
    let from = observed
        .iter()
        .find(|value| {
            value["trait_method"]["path"]
                .as_str()
                .unwrap()
                .ends_with("::convert::From::from")
        })
        .unwrap();
    assert!(from["associated_error"].is_null());
    assert_eq!(from["members"].as_array().unwrap().len(), 1);
    let errors: Vec<_> = observed
        .iter()
        .filter_map(|value| value["associated_error"].as_object())
        .collect();
    assert_eq!(errors.len(), 2);
    for error in &errors {
        assert_eq!(error["declared_type"], error["normalized_projection"]);
        assert_eq!(error["trait_member"]["crate"], "core");
        assert!(
            error["trait_member"]["path"]
                .as_str()
                .unwrap()
                .ends_with("::convert::TryFrom::Error")
        );
        let expected = if source_range(&error["source"]).contains("ErrorA") {
            "ErrorA"
        } else {
            "ErrorB"
        };
        assert_eq!(
            error["declared_type"][0]["identity"]["definition"]["path"],
            expected
        );
    }
    assert_ne!(
        errors[0]["implementation_member"],
        errors[1]["implementation_member"]
    );
    assert_ne!(errors[0]["declared_type"], errors[1]["declared_type"]);
}

#[test]
fn generic_and_lookalike_members_never_acquire_concrete_core_authority() {
    let functions = captured("hir-data-v3");
    let generic = functions
        .iter()
        .find(|function| function.name.contains("Generic") && function.name.ends_with("::from"))
        .unwrap();
    assert_eq!(generic.build.frontend_options[KEY]["status"], "unsupported");
    for function in functions
        .iter()
        .filter(|function| function.name.contains("lookalike::") || function.name == "ordinary")
    {
        assert!(function.build.frontend_options.get(KEY).is_none());
    }
    assert!(
        captured("hir-data-v2").iter().all(|function| function
            .build
            .frontend_options
            .get(KEY)
            .is_none())
    );
}
