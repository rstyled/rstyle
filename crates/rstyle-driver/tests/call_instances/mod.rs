//! Actual rustc selection is observational; generic bodies remain unsupported.
use super::*;
use serde_json::Value;

mod usize_readers;

const KEY: &str = "rstyle.rust.call-instances";
const SOURCE: &str = r#"
pub fn identity<T>(value: T) -> T { value }
pub fn plain(value: u8) -> u8 { value }
pub fn forward() { identity(1u8); identity(2u16); identity(3u8); plain(4); }
pub fn reversed() { identity(2u16); identity(1u8); }
pub struct Wrap<T>(T);
impl<T> From<T> for Wrap<T> { fn from(value: T) -> Self { Wrap(value) } }
pub fn converted() -> Wrap<u8> { Wrap::from(7u8) }
pub struct First([u8; 1]);
pub struct Second([u8; 1]);
impl AsRef<[u8]> for First { fn as_ref(&self) -> &[u8] { &self.0 } }
impl AsRef<[u8]> for Second { fn as_ref(&self) -> &[u8] { &self.0 } }
pub fn first(value: &First) -> &[u8] { value.as_ref() }
pub fn second(value: &Second) -> &[u8] { value.as_ref() }
pub fn unresolved<T: AsRef<[u8]> + ?Sized>(value: &T) -> &[u8] { value.as_ref() }
pub fn dynamic(value: &dyn AsRef<[u8]>) -> &[u8] { value.as_ref() }
pub fn nested() { let _callback = || identity(1u32); }
pub mod left { pub struct Twin; }
pub mod right { pub struct Twin; }
pub fn distinct_owners() { identity(left::Twin); identity(right::Twin); }
pub fn array_identity() { identity([1u8; 2]); }
pub fn const_argument<const N: usize>() -> usize { N }
pub fn const_caller() -> usize { const_argument::<4>() }
pub fn core_slice(value: &[u8]) -> &[u8] { <[u8] as AsRef<[u8]>>::as_ref(value) }
pub fn core_str(value: &str) -> &[u8] { <str as AsRef<[u8]>>::as_ref(value) }
"#;

fn captured(profile: &str) -> Vec<HirFnFacts> {
    captured_source(profile, SOURCE)
}

fn captured_source(profile: &str, source: &str) -> Vec<HirFnFacts> {
    let directory = directory();
    let input = directory.join("instances.rs");
    let facts = directory.join("instances.json");
    std::fs::write(&input, source).unwrap();
    let output = selected_invocation(&input, &facts, Some(profile))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let functions = serde_json::from_slice(&std::fs::read(&facts).unwrap()).unwrap();
    std::fs::remove_dir_all(directory).unwrap();
    functions
}

#[test]
fn oversized_and_deep_structural_arguments_remain_explicitly_unsupported() {
    let deep = (0..40).fold("u8".to_string(), |inner, _| format!("({inner},)"));
    let wide = format!("({})", vec!["u8"; 300].join(","));
    let source = format!(
        "pub fn identity<T>() {{}}\n\
         type Deep = {deep}; type Wide = {wide};\n\
         pub fn deep() {{ identity::<Deep>(); }}\n\
         pub fn wide() {{ identity::<Wide>(); }}"
    );
    let functions = captured_source("hir-data-v3", &source);
    for name in ["deep", "wide"] {
        let call = &calls(&functions, name)[0];
        assert_eq!(call["status"], "unsupported");
        assert_eq!(
            call["requested_arguments"][0]["identity_status"],
            "unsupported"
        );
        assert!(call.get("signature").is_none());
    }
}

fn function<'a>(functions: &'a [HirFnFacts], name: &str) -> &'a HirFnFacts {
    functions
        .iter()
        .find(|function| function.name == name)
        .unwrap()
}

fn calls<'a>(functions: &'a [HirFnFacts], name: &str) -> &'a [Value] {
    let inventory = &function(functions, name).build.frontend_options[KEY];
    assert_eq!(
        inventory["identity"],
        "rstyle.rust.call-instance-observations/1"
    );
    assert_eq!(inventory["complete"], true);
    inventory["calls"].as_array().unwrap()
}

fn types(arguments: &Value) -> Vec<&str> {
    arguments
        .as_array()
        .unwrap()
        .iter()
        .filter(|argument| argument["kind"] == "type")
        .map(|argument| argument["value"].as_str().unwrap())
        .collect()
}

#[test]
fn concrete_generic_instances_preserve_arguments_repeats_order_and_source_anchors() {
    let functions = captured("hir-data-v3");
    let forward = calls(&functions, "forward");
    assert_eq!(
        forward.len(),
        3,
        "ordinary nongeneric call is not duplicated"
    );
    let reversed = calls(&functions, "reversed");
    for (call, ty) in forward.iter().zip(["u8", "u16", "u8"]) {
        assert_eq!(call["status"], "resolved");
        assert_eq!(types(&call["requested_arguments"]), [ty]);
        assert_eq!(types(&call["selected_arguments"]), [ty]);
        assert_eq!(call["signature"]["inputs"], serde_json::json!([ty]));
        assert_eq!(call["signature"]["output"], ty);
        let start = call["span"]["start"].as_u64().unwrap() as usize;
        let end = call["span"]["end"].as_u64().unwrap() as usize;
        assert!(function(&functions, "forward").source.bytes[start..end].starts_with("identity("));
    }
    assert_eq!(forward[0]["selected"], forward[2]["selected"]);
    assert_eq!(forward[0]["selected"], reversed[1]["selected"]);
    assert_eq!(
        forward[1]["selected_arguments"],
        reversed[0]["selected_arguments"]
    );
    assert_ne!(forward[0]["hir"], forward[2]["hir"]);
    assert_eq!(function(&functions, "identity").signature.generics.len(), 1);
    assert!(
        calls(&functions, "nested").is_empty(),
        "nested body has another owner"
    );
}

#[test]
fn trait_instances_distinguish_requested_and_selected_substitutions_and_unknowns() {
    let functions = captured("hir-data-v3");
    let conversion = &calls(&functions, "converted")[0];
    assert_eq!(conversion["status"], "resolved");
    assert_eq!(types(&conversion["requested_arguments"]).len(), 2);
    assert_eq!(types(&conversion["selected_arguments"]), ["u8"]);
    assert_ne!(conversion["requested"], conversion["selected"]);
    let first = &calls(&functions, "first")[0];
    let second = &calls(&functions, "second")[0];
    assert_eq!(first["status"], "resolved");
    assert_eq!(second["status"], "resolved");
    assert_ne!(first["selected"], second["selected"]);
    assert_eq!(first["signature"]["output"], "&[u8]");
    assert_eq!(calls(&functions, "unresolved")[0]["status"], "unresolved");
    assert_eq!(calls(&functions, "dynamic")[0]["status"], "unsupported");
    assert!(calls(&functions, "unresolved")[0].get("selected").is_none());
}

#[test]
fn historical_profile_has_no_instance_observations() {
    for function in captured("hir-data-v2") {
        assert!(function.build.frontend_options.get(KEY).is_none());
    }
}

#[test]
fn structural_instance_identity_distinguishes_nominal_owners_and_closes_unknown_forms() {
    let functions = captured("hir-data-v3");
    let owners = calls(&functions, "distinct_owners");
    assert_eq!(owners.len(), 2);
    let left = &owners[0]["selected_arguments"][0]["identity"];
    let right = &owners[1]["selected_arguments"][0]["identity"];
    for call in owners {
        assert_eq!(call["status"], "resolved");
        assert_eq!(call["signature"]["display_only"], true);
    }
    assert_eq!(left["kind"], "adt");
    assert_eq!(right["kind"], "adt");
    assert_ne!(
        left["definition"]["definition_identity"],
        right["definition"]["definition_identity"]
    );
    assert_eq!(
        left["definition"]["stable_crate_id"],
        right["definition"]["stable_crate_id"]
    );
    assert!(
        left["definition"]["path"]
            .as_str()
            .unwrap()
            .ends_with("left::Twin")
    );
    assert!(
        right["definition"]["path"]
            .as_str()
            .unwrap()
            .ends_with("right::Twin")
    );
    for name in ["array_identity", "const_caller"] {
        let call = &calls(&functions, name)[0];
        assert_eq!(call["status"], "unsupported");
        assert_eq!(
            call["selected_arguments"][0]["identity_status"],
            "unsupported"
        );
        assert!(call.get("signature").is_none());
    }
}

#[test]
fn concrete_core_as_ref_selects_distinct_slice_and_str_implementations() {
    let functions = captured("hir-data-v3");
    let slice = &calls(&functions, "core_slice")[0];
    let string = &calls(&functions, "core_str")[0];
    for call in [slice, string] {
        assert_eq!(call["status"], "resolved");
        assert_eq!(call["selected"]["crate"], "core");
        assert_eq!(call["selected"]["local"], false);
        assert_eq!(call["signature"]["output"], "&[u8]");
        assert_eq!(call["requested_arguments"][1]["identity"]["kind"], "slice");
        assert_eq!(
            call["requested_arguments"][1]["identity"]["element"]["name"],
            "u8"
        );
    }
    assert_ne!(
        slice["selected"]["definition_identity"],
        string["selected"]["definition_identity"]
    );
    assert_eq!(slice["requested_arguments"][0]["identity"]["kind"], "slice");
    assert_eq!(string["requested_arguments"][0]["identity"]["kind"], "str");
}
