//! Concrete identities are observations, not callback or value-flow permission.
use super::float_kernels::function;
use super::*;

const SOURCE: &str = r#"
use core::num::NonZero;
#[derive(Debug)]
pub struct ParsedItem<'a, T>(&'a [u8], T);
#[derive(Debug)]
pub struct Neighbor { pub value: Option<NonZero<u8>> }
impl<'a, T> ParsedItem<'a, T> {
    pub(crate) fn flat_map<U>(self, f: impl FnOnce(T) -> Option<U>) -> Option<ParsedItem<'a, U>> {
        Some(ParsedItem(self.0, f(self.1)?))
    }
}
pub fn byte(input: &[u8]) -> Option<ParsedItem<'_, NonZero<u8>>> {
    Some(ParsedItem(input, 1u8)).and_then(|parsed| parsed.flat_map(NonZero::new))
}
pub fn word(input: &[u8]) -> Option<ParsedItem<'_, NonZero<u16>>> {
    Some(ParsedItem(input, 1u16)).and_then(|parsed| parsed.flat_map(NonZero::new))
}
pub fn captured(input: &[u8], offset: u8) -> Option<ParsedItem<'_, NonZero<u8>>> {
    Some(ParsedItem(input, 1u8)).and_then(|parsed| {
        if offset == 0 { return None; } parsed.flat_map(NonZero::new)
    })
}
pub fn pointer(input: &[u8]) -> Option<ParsedItem<'_, NonZero<u8>>> {
    Some(ParsedItem(input, 1u8)).and_then(|parsed| {
        parsed.flat_map(NonZero::new as fn(u8) -> Option<NonZero<u8>>)
    })
}
"#;

#[test]
fn callable_observations_keep_nested_owners_and_unsupported_environments_distinct() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for (name, scalar) in [("byte", "u8"), ("word", "u16")] {
        check_callback(&report, name, scalar);
    }
    check_original_members(&report);
    check_neighbor(&report);
    let captured = &inventory(&report, "captured")["callbacks"][0];
    assert_eq!(captured["capture_count"], 1);
    assert_eq!(captured["callable"][0]["identity_status"], "unsupported");
    let pointer = &inventory(&report, "pointer")["callbacks"][0]["calls"][0];
    assert_eq!(pointer["status"], "unsupported");
}

fn check_neighbor(report: &Value) {
    let generated = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| {
            entry["facts"]["name"].as_str().is_some_and(|name| {
                name.starts_with("<Neighbor as ") && name.ends_with("Debug>::fmt")
            })
        })
        .collect::<Vec<_>>();
    let [entry] = generated.as_slice() else {
        panic!("the complete derived neighbor remains in the capture inventory");
    };

    assert_eq!(
        entry["outcome"], "excluded",
        "raw NonZero shape grants no generated-source authority"
    );
}

fn check_original_members(report: &Value) {
    let facts: Vec<rstyle_pdg::HirFnFacts> = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| serde_json::from_value(entry["facts"].clone()).unwrap())
        .collect();
    let store = rstyle_pdg::unit_store::UnitStore::capture(facts).unwrap();
    let analysis = store.analysis();

    for name in ["byte", "word"] {
        let index = store
            .functions()
            .iter()
            .position(|facts| facts.name == name)
            .unwrap();
        check_original_member(&analysis.function(index).unwrap());
    }
}

fn check_original_member(input: &rstyle_pdg::unit_store::FunctionView<'_>) {
    use rstyle_pdg::{generated_sources::capture::Definition, parsed_item_callbacks};
    let callback = &input.build.frontend_options["rstyle.rust.call-instances"]["callbacks"][0];
    let call = &callback["calls"][0];
    let item = &call["selected_arguments"][3]["identity"];
    let definition =
        |value: &Value| -> Definition { serde_json::from_value(value.clone()).unwrap() };
    let member = definition(&call["selected"]);
    let product = definition(
        &callback["callable"][0]["identity"]["signature"]["inputs"][0]["elements"][0]["definition"],
    );
    let option = definition(&item["signature"]["output"]["definition"]);
    let nonzero = definition(&call["selected_arguments"][2]["identity"]["definition"]);
    let constructor = definition(&item["definition"]);
    let context = parsed_item_callbacks::Context {
        member: &member,
        product: &product,
        option: &option,
        nonzero: &nonzero,
        constructor: &constructor,
    };
    let checked = parsed_item_callbacks::verify_in(input, call, &context)
        .map_err(|error| format!("{error}; selected callback: {item}"));
    check_receipt(input, checked);
}

fn check_receipt(
    input: &rstyle_pdg::HirFnFacts,
    checked: Result<rstyle_pdg::parsed_item_callbacks::Checked, String>,
) {
    let native = input
        .build
        .target_triple
        .known_value()
        .is_some_and(|target| target == "aarch64-unknown-linux-gnu");
    if native {
        assert_eq!(checked.unwrap().ordered_fields(), [0, 1]);
        assert!(
            input
                .signature
                .return_ty
                .normalized
                .as_ref()
                .unwrap()
                .contains("non_zero")
        );
    } else {
        assert!(
            checked.is_err(),
            "foreign core authority requires independent pins"
        );
    }
}

fn check_callback(report: &Value, name: &str, scalar: &str) {
    let inventory = inventory(report, name);
    let [callback] = inventory["callbacks"].as_array().unwrap().as_slice() else {
        panic!("one separately scoped callback");
    };
    assert_eq!(callback["capture_count"], 0);
    assert_eq!(
        callback["callable"][0]["identity"]["kind"],
        "noncapturing_closure"
    );
    assert_eq!(
        callback["callable"][0]["identity"]["upvars"]["elements"],
        serde_json::json!([])
    );
    let [call] = callback["calls"].as_array().unwrap().as_slice() else {
        panic!("one nested flat_map call");
    };
    assert_eq!(call["status"], "resolved", "{call}");
    let entry = function(report, name);
    let source = entry["facts"]["source"]["bytes"].as_str().unwrap();
    let start = call["span"]["start"].as_u64().unwrap() as usize;
    let end = call["span"]["end"].as_u64().unwrap() as usize;
    assert_eq!(&source[start..end], "parsed.flat_map(NonZero::new)");
    assert!(
        call["selected"]["path"]
            .as_str()
            .unwrap()
            .ends_with("::flat_map")
    );
    check_function_item(call, scalar);
    assert!(
        inventory["calls"]
            .as_array()
            .unwrap()
            .iter()
            .all(|root| root["hir"] != call["hir"])
    );
}

fn check_function_item(call: &Value, scalar: &str) {
    let item = call["selected_arguments"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(item["identity"]["kind"], "function_item");
    assert!(
        item["identity"]["definition"]["path"]
            .as_str()
            .unwrap()
            .ends_with("::new")
    );
    assert_eq!(item["identity"]["definition"]["crate"], "core");
    assert_eq!(item["identity"]["signature"]["inputs"][0]["name"], scalar);
    assert_eq!(item["identity"]["signature"]["bound_variables"], 0);
}

fn inventory<'a>(report: &'a Value, name: &str) -> &'a Value {
    let entry = function(report, name);
    assert_eq!(
        entry["outcome"], "excluded",
        "observations cannot certify a caller"
    );
    &entry["facts"]["build"]["frontend_options"]["rstyle.rust.call-instances"]
}
