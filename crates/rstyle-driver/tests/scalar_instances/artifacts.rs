//! Loaded artifact observations do not turn generic calls into verified helpers.
use super::*;

fn dependency(directory: &std::path::Path) -> Vec<HirFnFacts> {
    let input = directory.join("libscalar_dependency.rs");
    let facts = directory.join("dependency.json");
    std::fs::write(
        &input,
        "pub fn identity<T>(value:T)->T { value }\n\
         pub fn scalar(value:i32)->i32 { value }\n\
         pub fn predicate<T: PartialOrd>(a:T,b:T)->bool { a < b }",
    )
    .unwrap();
    let output = selected_invocation(&input, &facts, Some("hir-data-v3"))
        .args(["-C", "metadata=scalar_dependency"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    serde_json::from_slice(&std::fs::read(facts).unwrap()).unwrap()
}

fn callers(directory: &std::path::Path, profile: &str) -> Vec<HirFnFacts> {
    let input = directory.join("scalar_callers.rs");
    let facts = directory.join("callers.json");
    std::fs::write(
        &input,
        "pub fn signed(x:i32)->i32 { renamed::identity::<i32>(x) }\n\
         pub fn unsigned(x:u32)->u32 { renamed::identity::<u32>(x) }\n\
         pub fn small(x:u8)->u8 { renamed::identity::<u8>(x) }\n\
         pub fn local<T>(x:T)->T { x }\n\
         pub fn local_call(x:i32)->i32 { local(x) }\n\
         pub fn ordinary(x:i32)->i32 { renamed::scalar(x) }\n\
         pub fn predicate(x:i32,y:i32)->bool { renamed::predicate(x,y) }",
    )
    .unwrap();
    let output = selected_invocation(&input, &facts, Some(profile))
        .args(["-C", "metadata=scalar_caller", "--extern"])
        .arg(format!(
            "renamed={}",
            directory.join("libscalar_dependency.rmeta").display()
        ))
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output);
    serde_json::from_slice(&std::fs::read(facts).unwrap()).unwrap()
}

fn observation(functions: &[HirFnFacts], name: &str) -> Value {
    let facts = functions.iter().find(|facts| facts.name == name).unwrap();
    facts.build.frontend_options["rstyle.rust.call-instances"]["calls"][0].clone()
}

fn check_artifact(call: &Value, local: &Value, directory: &std::path::Path, scalar: &str) {
    assert_eq!(call["status"], "resolved");
    assert_eq!(call["requested"], call["selected"]);
    assert_eq!(call["requested_arguments"], call["selected_arguments"]);
    assert_eq!(call["selected_arguments"][0]["identity"]["name"], scalar);
    let artifact = &call["compiled_dependency"];
    for key in ["identity", "crate_name", "stable_crate_id"] {
        assert_eq!(artifact[key], local[key]);
    }
    assert_eq!(
        artifact["rmeta"],
        directory
            .join("libscalar_dependency.rmeta")
            .to_str()
            .unwrap()
    );
    assert!(!artifact["crate_hash"].as_str().unwrap().is_empty());
    assert_eq!(artifact["instance"]["type_arguments"], json!([]));
    assert_eq!(artifact["instance"]["const_arguments"], json!([]));
}

fn check_call_node(facts: &HirFnFacts, scalar: &str) {
    let body_json = serde_json::to_value(&facts.body).unwrap();
    let mut body = Vec::new();
    nodes(&body_json, &mut body);
    let node = body
        .iter()
        .find(|node| node["kind"] == "expr_call")
        .unwrap();
    assert_eq!(node["res"]["kind"], "resolved_scalar_instance_callee");
    assert_eq!(node["dispatch"]["kind"], "resolved_scalar_instance");
    assert_eq!(node["dispatch"]["operand_tys"], json!([scalar]));
    assert_eq!(node["dispatch"]["result_ty"], scalar);
    assert!(
        body.iter()
            .all(|node| node["res"]["kind"] != "resolved_direct_callee")
    );
    assert_ne!(classify(facts).status, EligibilityVerdict::Eligible);
}

#[test]
fn scalar_instance_calls_observe_loaded_artifacts_without_helper_authority() {
    let directory = directory();
    let owner = dependency(&directory);
    let functions = callers(&directory, "hir-data-v3");
    let local = &owner[0].build.frontend_options["rstyle.rust.compiled-crate"];
    for (name, scalar) in [("signed", "i32"), ("unsigned", "u32")] {
        let call = observation(&functions, name);
        check_artifact(&call, local, &directory, scalar);
        let facts = functions.iter().find(|facts| facts.name == name).unwrap();
        check_call_node(facts, scalar);
    }
    for name in ["small", "local_call"] {
        assert!(
            observation(&functions, name)
                .get("compiled_dependency")
                .is_none()
        );
    }
    let predicate = functions
        .iter()
        .find(|facts| facts.name == "predicate")
        .unwrap();
    let body = serde_json::to_value(&predicate.body).unwrap();
    let mut predicate_nodes = Vec::new();
    nodes(&body, &mut predicate_nodes);
    assert!(
        predicate_nodes
            .iter()
            .all(|node| { node["res"]["kind"] != "resolved_scalar_instance_callee" })
    );
    let legacy = callers(&directory, "hir-data-v2");
    assert!(legacy.iter().all(|facts| {
        !facts
            .build
            .frontend_options
            .contains_key("rstyle.rust.call-instances")
    }));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn original_owner_publication_keeps_complete_candidate_outcomes_without_requests() {
    let capture = capture(requests(json!([])));
    assert!(capture.output.status.success(), "{:?}", capture.output);
    let raw = capture.snapshot("-pre-helper-certification.json");
    let functions: Vec<HirFnFacts> = serde_json::from_value(raw["facts"].clone()).unwrap();
    let key = "rstyle.rust.scalar-instance-projections";
    for name in ["clamp", "minimum", "tuple", "nested"] {
        let original = functions.iter().find(|facts| facts.name == name).unwrap();
        let inventory = &original.build.frontend_options[key];
        assert_eq!(
            inventory["identity"],
            "rstyle.rust.scalar-instance-projections/1"
        );
        assert_eq!(inventory["complete"], true);
        let entries = inventory["requests"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        for (entry, scalar) in entries.iter().zip(["i32", "u32"]) {
            assert_eq!(entry["request"]["function"], name);
            assert_eq!(entry["request"]["arguments"], json!([scalar]));
            assert!(entry.get("build").is_none());
            if name == "clamp" || name == "minimum" {
                assert_eq!(entry["status"], "projected");
                assert_eq!(entry["source"]["bytes"], &original.source.bytes[..]);
                assert_eq!(entry["signature"]["generics"].as_array().unwrap().len(), 1);
                assert_eq!(entry["signature"]["params"][0]["ty"]["spelling"], "T");
            } else {
                assert_eq!(entry["status"], "unsupported");
                assert!(
                    entry["reason"]
                        .as_str()
                        .is_some_and(|reason| !reason.is_empty())
                );
                assert!(entry.get("body").is_none());
            }
        }
        assert_eq!(original.signature.return_ty.observed, "T");
        assert_ne!(classify(original).status, EligibilityVerdict::Eligible);
    }
    for name in ["relations", "konst"] {
        let original = functions.iter().find(|facts| facts.name == name).unwrap();
        assert!(!original.build.frontend_options.contains_key(key));
    }
}
