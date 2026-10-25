//! Concrete identities are observations, not callback or value-flow permission.
use super::float_kernels::function;
use super::*;

const SOURCE: &str = r#"
use core::num::NonZero;
#[derive(Debug)]
pub struct ParsedItem<'a, T>(&'a [u8], T);
impl<'a, T> ParsedItem<'a, T> {
    fn flat_map<U>(self, f: impl FnOnce(T) -> Option<U>) -> Option<ParsedItem<'a, U>> {
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
    let captured = &inventory(&report, "captured")["callbacks"][0];
    assert_eq!(captured["capture_count"], 1);
    assert_eq!(captured["callable"][0]["identity_status"], "unsupported");
    let pointer = &inventory(&report, "pointer")["callbacks"][0]["calls"][0];
    assert_eq!(pointer["status"], "unsupported");
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
