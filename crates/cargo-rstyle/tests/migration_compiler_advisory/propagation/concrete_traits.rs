use super::*;

const LEAF: &str = r#"
pub struct Reader;
pub struct Other;
pub trait Action {
    fn act(&self);
    fn fallback(&self) { std::fs::write("/unused", b"default").ok(); }
}
impl Action for Reader {
    fn act(&self) { std::fs::write("/unused", b"reader").ok(); }
}
impl Action for Other { fn act(&self) {} }
"#;

const CALLERS: &str = r#"
use renamed::Action;
pub fn method() { renamed::Reader.act(); }
pub fn ufcs() { <renamed::Reader as Action>::act(&renamed::Reader); }
pub fn namesake() { renamed::Other.act(); }
pub fn default_body() { renamed::Reader.fallback(); }
"#;

fn fixture(leaf: &str, callers: &str) -> TempDir {
    let directory = workspace(leaf);
    std::fs::write(directory.path().join("caller/src/lib.rs"), callers).unwrap();
    directory
}

#[test]
fn concrete_method_ufcs_and_default_calls_select_original_implementation_effects() {
    let directory = fixture(LEAF, CALLERS);
    let (output, report) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 0);
    assert_eq!(
        report["compiler_advisory"]["metrics"],
        "rstyle.compiler-style-metrics/6"
    );
    let caller = unit(&report, "caller");
    let leaf = unit(&report, "leaf");
    assert_eq!(io(caller), 3);
    assert_eq!(io(leaf), 2);
    assert_eq!(caller["unresolved_callees"], serde_json::json!([]));
    let resolved = caller["resolved_dependency_callees"].as_object().unwrap();
    assert_eq!(resolved.len(), 3);
    assert!(resolved.values().all(|target| target == &leaf["unit"]));
    assert!(!resolved.contains_key("leaf::Action::act"));
    assert!(resolved.contains_key("leaf::Action::fallback"));
    assert_eq!(
        resolved
            .keys()
            .filter(|name| name.ends_with("::act"))
            .count(),
        2
    );
    assert_eq!(report["compiler_advisory"]["complete"], true);
    assert_eq!(
        report["compiler_advisory"]["planning"]["input_complete"],
        true
    );
}

#[test]
fn trait_receiver_plans_exclude_ufcs_self_and_replay_without_sources_or_tools() {
    let leaf = r#"
pub struct Reader;
pub trait Action { fn act(&self,a:u128,b:u128,c:u128,d:u128,e:u128); }
impl Action for Reader {
    fn act(&self,a:u128,b:u128,c:u128,d:u128,e:u128) {
        std::fs::write("/unused",b"reader").ok();
    }
}
"#;
    let callers = r#"
use renamed::Action;
pub fn method(a:u128,b:u128,c:u128,d:u128,e:u128) { renamed::Reader.act(a,b,c,d,e); }
pub fn ufcs(a:u128,b:u128,c:u128,d:u128,e:u128) { <renamed::Reader as Action>::act(&renamed::Reader,a,b,c,d,e); }
"#;
    let directory = fixture(leaf, callers);
    let (output, original) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 0);
    assert_eq!(io(unit(&original, "caller")), 2);
    let groups = original["compiler_advisory"]["planning"]["groups"]
        .as_array()
        .unwrap();
    assert_eq!(groups.len(), 1);
    let steps = groups[0]["evaluation"]["plan"]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 3);
    assert_eq!(steps[0]["incoming_sites"], 2);
    for step in &steps[1..] {
        assert_eq!(step["outgoing_sites"], 1);
        assert_eq!(
            step["pass_through"],
            serde_json::json!(["a", "b", "c", "d", "e"])
        );
    }
    source_free_replay(directory.path(), &original);
}

pub(super) fn source_free_replay(directory: &Path, original: &Value) {
    for path in ["caller", "leaf"] {
        std::fs::remove_dir_all(directory.join(path)).unwrap();
    }
    for path in ["Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.join(path)).unwrap();
    }
    let mut command = Command::new(cargo_bin("cargo-rstyle").canonicalize().unwrap());
    command
        .current_dir(directory)
        .args([
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-plans",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
        ])
        .env("PATH", directory.join("no-tools"));
    let output = diagnostic_process::output(&mut command).unwrap();
    checked(&output, 0);
    let replayed: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(original["compiler_advisory"], replayed["compiler_advisory"]);
    assert_eq!(original["matching"], replayed["matching"]);
    assert_eq!(original["accepted_pairs"], replayed["accepted_pairs"]);
}

#[test]
fn generic_and_dynamic_trait_calls_do_not_gain_an_exact_implementation_owner() {
    let callers = r#"
pub fn generic<T:renamed::Action>(value:&T) { value.act(); }
pub fn dynamic(value:&dyn renamed::Action) { value.act(); }
"#;
    let directory = fixture(LEAF, callers);
    let (output, report) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 2);
    let caller = unit(&report, "caller");
    assert_eq!(io(caller), 0);
    assert_eq!(
        caller["unresolved_callees"],
        serde_json::json!(["leaf::Action::act"])
    );
    assert!(caller.get("resolved_dependency_callees").is_none());
    assert_eq!(report["compiler_advisory"]["complete"], false);
    assert_eq!(
        report["compiler_advisory"]["planning"]["input_complete"],
        false
    );
}

#[test]
fn missing_generated_and_excluded_selected_implementations_remain_unresolved() {
    let callers = "use renamed::Action; pub fn method() { renamed::Reader.act(); }";
    let directory = fixture(LEAF, callers);
    let (output, missing) = clones(
        directory.path(),
        &[
            "-p",
            "caller",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
            "--compiler-plans",
        ],
    );
    checked(&output, 2);
    std::fs::write(
        directory.path().join("policy.toml"),
        "exclude=['leaf/**']\n",
    )
    .unwrap();
    let (output, excluded) = capture(
        directory.path(),
        &[
            "--compiler-plans",
            "--compiler-advisory-config",
            "policy.toml",
        ],
    );
    checked(&output, 2);
    let generated = LEAF.replace(
        "impl Action for Reader {\n    fn act(&self) { std::fs::write(\"/unused\", b\"reader\").ok(); }\n}",
        "macro_rules! implement {()=>{impl Action for Reader { fn act(&self) { std::fs::write(\"/unused\", b\"reader\").ok(); } }}} implement!();",
    );
    assert_ne!(generated, LEAF);
    std::fs::write(directory.path().join("leaf/src/lib.rs"), generated).unwrap();
    let (output, generated) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 2);
    for report in [&missing, &excluded, &generated] {
        let caller = unit(report, "caller");
        assert_eq!(io(caller), 0);
        let unresolved = caller["unresolved_callees"].as_array().unwrap();
        assert_eq!(unresolved.len(), 1);
        assert!(unresolved[0].as_str().unwrap().ends_with("::act"));
        assert_ne!(unresolved[0], "leaf::Action::act");
        assert!(caller.get("resolved_dependency_callees").is_none());
        assert_eq!(report["compiler_advisory"]["complete"], false);
        assert_eq!(
            report["compiler_advisory"]["planning"]["input_complete"],
            false
        );
    }
}

#[test]
fn selected_edges_preserve_configured_external_trait_call_classification() {
    let leaf = "pub struct Reader; pub trait Action { fn act(&self); } impl Action for Reader { fn act(&self) {} }";
    let directory = fixture(
        leaf,
        "use renamed::Action; pub fn method() { renamed::Reader.act(); }",
    );
    std::fs::write(
        directory.path().join("policy.toml"),
        "[side_effects.paths]\nio=['leaf::Action::act']\n",
    )
    .unwrap();
    let (output, report) = capture(
        directory.path(),
        &[
            "--compiler-plans",
            "--compiler-advisory-config",
            "policy.toml",
        ],
    );
    checked(&output, 0);
    assert_eq!(io(unit(&report, "caller")), 1);
    assert_eq!(io(unit(&report, "leaf")), 0);
    let resolved = unit(&report, "caller")["resolved_dependency_callees"]
        .as_object()
        .unwrap();
    assert_eq!(resolved.len(), 1);
    assert!(!resolved.contains_key("leaf::Action::act"));
}
