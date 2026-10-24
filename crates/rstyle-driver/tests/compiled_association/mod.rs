//! Real compiler observations distinguish loaded dependencies from local helpers.
use super::*;
use serde_json::Value;

fn records(path: &std::path::Path) -> Vec<HirFnFacts> {
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn compile_dependency(directory: &std::path::Path) -> Vec<HirFnFacts> {
    let input = directory.join("libdependency.rs");
    let facts = directory.join("dependency.json");
    std::fs::write(
        &input,
        "pub fn scalar(x: u8) -> u8 { x }\npub fn generic<T>(x: T) -> T { x }\n",
    )
    .unwrap();
    let output = selected_invocation(&input, &facts, Some("hir-data-v3"))
        .args(["-C", "metadata=dependency"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    records(&facts)
}

fn compile_caller(directory: &std::path::Path, profile: &str) -> Vec<HirFnFacts> {
    let input = directory.join("caller.rs");
    let facts = directory.join("caller.json");
    std::fs::write(
        &input,
        "pub fn local(x: u8) -> u8 { x }\npub fn caller(x: u8) -> u8 { renamed::scalar(x) }\npub fn local_caller(x: u8) -> u8 { local(x) }\npub fn generic_caller(x: u8) -> u8 { renamed::generic::<u8>(x) }\n",
    )
    .unwrap();
    let output = selected_invocation(&input, &facts, Some(profile))
        .args(["-C", "metadata=caller", "--extern"])
        .arg(format!(
            "renamed={}",
            directory.join("libdependency.rmeta").display()
        ))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    records(&facts)
}

fn resolution(functions: &[HirFnFacts], name: &str) -> Value {
    let function = functions
        .iter()
        .find(|function| function.name == name)
        .unwrap();
    find_resolution(&function.body).unwrap_or(Value::Null)
}

fn find_resolution(node: &rstyle_pdg::frontend::HirNode) -> Option<Value> {
    rstyle_pdg::helper_calls::direct_resolution(node)
        .or_else(|| node.children.iter().find_map(find_resolution))
}

fn check_local_and_generic(caller: &[HirFnFacts]) {
    let local_call = resolution(caller, "local_caller");
    assert_eq!(local_call["local"], true);
    assert!(local_call.get("compiled_dependency").is_none());
    assert!(resolution(caller, "generic_caller")["compiled_dependency"].is_null());
}

#[test]
fn compiled_dependency_observations_bind_actual_rmeta_and_keep_generic_calls_closed() {
    let directory = directory();
    let dependency = compile_dependency(&directory);
    let caller = compile_caller(&directory, "hir-data-v3");
    let local = &dependency[0].build.frontend_options["rstyle.rust.compiled-crate"];
    let foreign = resolution(&caller, "caller");
    let observed = &foreign["compiled_dependency"];
    assert_eq!(local["identity"], "rstyle.rust.compiled-crate/1");
    assert!(local.get("crate_hash").is_none());
    for key in ["identity", "crate_name", "stable_crate_id"] {
        assert_eq!(observed[key], local[key], "{key}");
    }
    assert_eq!(foreign["local"], false);
    assert_eq!(
        observed["rmeta"],
        directory.join("libdependency.rmeta").to_str().unwrap()
    );
    assert!(!observed["crate_hash"].as_str().unwrap().is_empty());
    assert_eq!(
        observed["instance"]["type_arguments"],
        serde_json::json!([])
    );
    assert_eq!(
        observed["instance"]["const_arguments"],
        serde_json::json!([])
    );
    assert_ne!(
        observed["stable_crate_id"],
        caller[0].build.frontend_options["rstyle.rust.compiled-crate"]["stable_crate_id"]
    );
    check_local_and_generic(&caller);
    let legacy = compile_caller(&directory, "hir-data-v2");
    assert!(
        legacy[0]
            .build
            .frontend_options
            .get("rstyle.rust.compiled-crate")
            .is_none()
    );
    assert!(resolution(&legacy, "caller")["compiled_dependency"].is_null());
    std::fs::remove_dir_all(directory).unwrap();
}
