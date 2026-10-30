use super::*;

const METHODS: &str = r#"
pub struct Reader;
impl Reader {
    pub fn static_read() { std::fs::write("/unused", b"static").ok(); }
    pub fn receiver_read(&self) { std::fs::write("/unused", b"receiver").ok(); }
}
pub struct Other;
impl Other { pub fn static_read() {} }
pub struct Generic<T>(pub T);
impl<T> Generic<T> {
    pub fn generic_read(&self) { std::fs::write("/unused", b"generic").ok(); }
}
"#;

const METHOD_CALLER: &str = r#"
pub fn static_caller() { renamed::Reader::static_read(); }
pub fn receiver_caller() { renamed::Reader.receiver_read(); }
pub fn generic_caller(generic: &renamed::Generic<u8>) { generic.generic_read(); }
pub fn namesake_caller() { renamed::Other::static_read(); }
"#;

fn methods() -> TempDir {
    let directory = workspace(METHODS);
    std::fs::write(directory.path().join("caller/src/lib.rs"), METHOD_CALLER).unwrap();
    directory
}

#[test]
fn exact_inherent_static_receiver_and_generic_targets_propagate_observable_effects() {
    let directory = methods();
    let (output, report) = capture(directory.path(), &["--compiler-plans", "--max-pairs", "0"]);
    checked(&output, 0);
    let caller = unit(&report, "caller");
    let leaf = unit(&report, "leaf");
    assert_eq!(io(caller), 3, "{caller}");
    assert_eq!(io(leaf), 3);
    assert_eq!(caller["unresolved_callees"], serde_json::json!([]));
    let resolved = caller["resolved_dependency_callees"].as_object().unwrap();
    assert_eq!(resolved.len(), 4);
    assert!(resolved.values().all(|target| target == &leaf["unit"]));
    for method in ["static_read", "receiver_read", "generic_read"] {
        assert!(resolved.keys().any(|callee| callee.ends_with(method)));
    }
    assert_eq!(
        report["compiler_advisory"]["planning"]["input_complete"],
        true
    );
    assert!(report["accepted_pairs"].as_array().unwrap().is_empty());
}

#[test]
fn missing_and_excluded_inherent_owners_keep_their_callers_incomplete() {
    let directory = methods();
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
    for report in [&missing, &excluded] {
        let caller = unit(report, "caller");
        assert_eq!(io(caller), 0);
        assert_eq!(caller["unresolved_callees"].as_array().unwrap().len(), 4);
        assert!(caller.get("resolved_dependency_callees").is_none());
        assert_eq!(report["compiler_advisory"]["complete"], false);
        assert_eq!(
            report["compiler_advisory"]["planning"]["input_complete"],
            false
        );
    }
}

#[test]
fn inherent_receiver_plans_preserve_argument_positions_and_replay_without_sources() {
    let directory = workspace(
        "pub struct Reader; impl Reader { pub fn read(&self,a:u128,b:u128,c:u128,d:u128,e:u128) { std::fs::write(\"/unused\",b\"x\").ok(); } }",
    );
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "pub fn caller(a:u128,b:u128,c:u128,d:u128,e:u128) { renamed::Reader.read(a,b,c,d,e); }",
    )
    .unwrap();
    let (output, original) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 0);
    let groups = original["compiler_advisory"]["planning"]["groups"]
        .as_array()
        .unwrap();
    assert_eq!(groups.len(), 1);
    let steps = &groups[0]["evaluation"]["plan"]["steps"];
    assert_eq!(steps[0]["incoming_sites"], 1);
    assert_eq!(steps[1]["outgoing_sites"], 1);
    assert_eq!(
        steps[1]["pass_through"],
        serde_json::json!(["a", "b", "c", "d", "e"])
    );
    for path in ["caller", "leaf"] {
        std::fs::remove_dir_all(directory.path().join(path)).unwrap();
    }
    for path in ["Cargo.toml", "Cargo.lock"] {
        std::fs::remove_file(directory.path().join(path)).unwrap();
    }
    let (output, replayed) = capture(
        directory.path(),
        &[
            "--compiler-plans",
            "--replay-run",
            original["run_directory"].as_str().unwrap(),
            "--max-pairs",
            "0",
        ],
    );
    checked(&output, 0);
    assert_eq!(original["compiler_advisory"], replayed["compiler_advisory"]);
    assert_eq!(original["matching"], replayed["matching"]);
}

#[test]
fn a_trait_dispatch_alias_does_not_become_an_exact_dependency_owner() {
    let directory = workspace(
        "pub struct Reader; pub trait Action { fn act(&self); } impl Action for Reader { fn act(&self) { std::fs::write(\"/unused\",b\"x\").ok(); } }",
    );
    std::fs::write(
        directory.path().join("caller/src/lib.rs"),
        "pub fn caller<T:renamed::Action>(value:&T) { value.act(); }",
    )
    .unwrap();
    let (output, report) = capture(directory.path(), &["--compiler-plans"]);
    checked(&output, 2);
    let caller = unit(&report, "caller");
    assert_eq!(io(caller), 0);
    assert_eq!(io(unit(&report, "leaf")), 1);
    assert_eq!(
        caller["unresolved_callees"],
        serde_json::json!(["leaf::Action::act"])
    );
    assert!(caller.get("resolved_dependency_callees").is_none());
    assert_eq!(
        report["compiler_advisory"]["planning"]["input_complete"],
        false
    );
}
