use super::*;

fn external(source: &str) -> TempDir {
    let directory = workspace("pub mod external;\n");
    std::fs::write(directory.path().join("src/external.rs"), source).unwrap();
    directory
}

fn dismiss(directory: &Path, original: &Value, name: &str) -> Value {
    let output = Replay {
        directory,
        original,
    }
    .run(&[
        "--dismiss-parameter",
        key(original, name),
        "--review-reason",
        REASON,
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    parsed(&output)
}

fn stored(directory: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(directory.join("reviews.json")).unwrap()).unwrap()
}

#[test]
fn external_associated_and_receiver_methods_have_original_method_locators() {
    for (signature, name, has_self) in [
        ("pub fn read(a:u32,b:u32)->u32 { a+b }", "read", false),
        ("pub fn read(&self,a:u32,b:u32)->u32 { a+b }", "read", true),
    ] {
        let directory = external(&format!("pub struct Unit; impl Unit {{ {signature} }}\n"));
        let (output, original) = capture(directory.path(), &[]);
        assert!(output.status.success());
        counts(&original, 0, 1);
        let reviewed = dismiss(
            directory.path(),
            &original,
            &format!("external::Unit::{name}"),
        );
        counts(&reviewed, 1, 0);
        assert_eq!(reviewed["analysis"]["findings"], 1);
        let store = stored(directory.path());
        let finding = &store["parameter_entries"][0]["finding"];
        assert_eq!(finding["locator"]["source"], "src/external.rs");
        assert_eq!(
            finding["locator"]["qualified_name"],
            "crate::external::Unit::read"
        );
        assert_eq!(finding["locator"]["declaration_kind"], "method");
        assert_eq!(finding["signature"]["has_self"], has_self);
        assert_eq!(finding["signature"]["parameters"], 2);
        assert_eq!(finding["signature"]["actual"], 2);
    }
}

#[test]
fn external_method_review_replays_without_source_or_tools_in_full_and_compact() {
    let directory =
        external("pub struct Unit; impl Unit { pub fn read(&self,a:u32,b:u32)->u32 { a+b } }\n");
    let (_, original) = capture(directory.path(), &[]);
    dismiss(directory.path(), &original, "external::Unit::read");
    let bytes = std::fs::read(directory.path().join("reviews.json")).unwrap();
    for file in ["src/lib.rs", "src/external.rs", "Cargo.toml"] {
        std::fs::remove_file(directory.path().join(file)).unwrap();
    }
    let replay = Replay {
        directory: directory.path(),
        original: &original,
    };
    let mut reviews = Vec::new();
    for detail in ["full", "compact"] {
        let output = replay.run(&["--report-detail", detail, "--fail-on-findings"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report = parsed(&output);
        assert_eq!(report["verification_policy"], "strict-replay");
        counts(&report, 1, 0);
        assert_eq!(report["analysis"]["findings"], 1);
        assert_eq!(report["parameter_review"]["coverage"], "complete");
        reviews.push(report["parameter_review"].clone());
    }
    assert_eq!(reviews[0], reviews[1]);
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        bytes
    );
}

const NAMESAKES: &str = "pub struct Unit; pub struct Other;
impl Unit { pub fn read(&self,a:u32,b:u32)->u32 { a+b } }
impl Other { pub fn read(&self,a:u32,b:u32)->u32 { a+b } }
pub mod nested { pub struct Unit; impl Unit { pub fn read(&self,a:u32,b:u32)->u32 { a+b } } }
";

#[test]
fn external_method_namesakes_keep_owner_and_module_identity_across_movement() {
    let directory = external(NAMESAKES);
    let (_, original) = capture(directory.path(), &[]);
    counts(&original, 0, 3);
    let reviewed = dismiss(directory.path(), &original, "external::Unit::read");
    counts(&reviewed, 1, 2);
    let bytes = std::fs::read(directory.path().join("reviews.json")).unwrap();
    let moved = NAMESAKES.replace(
        "impl Unit { pub fn read(&self,a:u32,b:u32)->u32 { a+b } }",
        "// moved and changed body\n\nimpl Unit { pub fn read(&self,left:u32,right:u32)->u32 { let saved=left*right; saved+3 } }",
    );
    std::fs::write(directory.path().join("src/external.rs"), moved).unwrap();
    let (_, changed) = capture(directory.path(), &["--review-status"]);
    counts(&changed, 1, 2);
    assert_eq!(
        changed["review_status"]["records"][0]["state"],
        "hidden_exact"
    );
    let selected = changed["parameter_review"]["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|decision| decision["key"] == key(&changed, "external::Unit::read"))
        .unwrap();
    assert_eq!(selected["state"], "hidden_exact");
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        bytes
    );
}

#[test]
fn changed_external_method_arity_stales_the_saved_parameter_boundary() {
    let directory =
        external("pub struct Unit; impl Unit { pub fn read(&self,a:u32,b:u32)->u32 { a+b } }\n");
    let (_, original) = capture(directory.path(), &[]);
    dismiss(directory.path(), &original, "external::Unit::read");
    let bytes = std::fs::read(directory.path().join("reviews.json")).unwrap();
    std::fs::write(
        directory.path().join("src/external.rs"),
        "pub struct Unit; impl Unit { pub fn read(&self,a:u32,b:u32,c:u32)->u32 { a+b+c } }\n",
    )
    .unwrap();
    let (output, changed) = capture(directory.path(), &["--review-status", "--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1));
    counts(&changed, 0, 1);
    assert_eq!(
        changed["parameter_review"]["decisions"][0]["state"],
        "stale"
    );
    assert_eq!(changed["review_status"]["records"][0]["state"], "stale");
    assert_eq!(
        std::fs::read(directory.path().join("reviews.json")).unwrap(),
        bytes
    );
}

#[test]
fn external_local_trait_impl_method_uses_the_qualified_trait_owner() {
    let directory = external(
        "pub struct Unit; pub trait Local { fn read(&self,a:u32,b:u32)->u32; }
impl Local for Unit { fn read(&self,a:u32,b:u32)->u32 { a+b } }\n",
    );
    let (output, original) = capture(directory.path(), &[]);
    assert!(output.status.success());
    counts(&original, 0, 1);
    let function = parameter_finding(&original);
    let reviewed = dismiss(
        directory.path(),
        &original,
        function["name"].as_str().unwrap(),
    );
    counts(&reviewed, 1, 0);
    assert_eq!(reviewed["analysis"]["findings"], 1);
    let store = stored(directory.path());
    let locator = &store["parameter_entries"][0]["finding"]["locator"];
    assert_eq!(
        locator["qualified_name"],
        "crate::external::<Unit as Local>::read"
    );
    assert_eq!(locator["declaration_kind"], "method");
}

fn parameter_finding(report: &Value) -> &Value {
    report["analysis"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            function["assessments"]
                .as_array()
                .unwrap()
                .iter()
                .any(|assessment| {
                    assessment["rule_id"] == "fn_params" && assessment["outcome"] == "finding"
                })
        })
        .unwrap()
}

#[test]
fn unsupported_external_method_owner_forms_keep_raw_findings_and_refuse_dismissal() {
    for source in [
        "pub struct Unit<T>(pub T); impl<T> Unit<T> { pub fn read(a:u32,b:u32)->u32 { a+b } }\n",
        "pub struct Unit; type Alias=Unit; impl Alias { pub fn read(a:u32,b:u32)->u32 { a+b } }\n",
        "pub struct Unit; impl crate::external::Unit { pub fn read(a:u32,b:u32)->u32 { a+b } }\n",
        "pub struct Unit; impl std::ops::Add for Unit { type Output=Self; fn add(self,_rhs:Self)->Self { self } }\n",
        "pub struct Unit; mod names { pub trait Local { fn read(a:u32,b:u32)->u32; } } use names::Local; impl Local for Unit { fn read(a:u32,b:u32)->u32 { a+b } }\n",
        "pub trait Local { fn read(a:u32,b:u32)->u32 { a+b } }\n",
    ] {
        let directory = external(source);
        std::fs::write(
            directory.path().join("analysis.toml"),
            "[fn_length]\nmax_lines=100\n[fn_params]\nmax_params=0\ncount_self=false\n",
        )
        .unwrap();
        let (output, original) = capture(directory.path(), &[]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{source}: {}",
            original["parameter_review"]
        );
        assert_eq!(original["analysis"]["findings"], 1, "{source}");
        counts(&original, 0, 1);
        assert_eq!(
            original["parameter_review"]["decisions"][0]["state"], "unavailable",
            "{source}"
        );
        let function = parameter_finding(&original);
        let output = Replay {
            directory: directory.path(),
            original: &original,
        }
        .run(&[
            "--dismiss-parameter",
            function["key"].as_str().unwrap(),
            "--review-reason",
            REASON,
        ]);
        assert_eq!(output.status.code(), Some(2), "{source}");
        assert!(!directory.path().join("reviews.json").exists(), "{source}");
    }
}
