use super::*;
mod prune;
mod status;

fn case(id: &str) -> Value {
    let labels: Value = serde_json::from_str(include_str!(
        "../../../../eval/clones-capability/abstraction-review-cases.json"
    ))
    .unwrap();
    labels["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap()
        .clone()
}

fn workspace(id: &str) -> TempDir {
    let directory = clone_workspace();
    write_source(directory.path(), case(id)["source"].as_str().unwrap());
    directory
}

fn write_source(directory: &Path, source: &str) {
    std::fs::write(directory.join("src/lib.rs"), source).unwrap();
}

fn capture(directory: &Path, id: &str, extra: &[&str]) -> Value {
    let labels = case(id);
    let mut arguments = vec![
        "--extraction-profile",
        "hir-data-v3",
        labels["flag"].as_str().unwrap(),
    ];
    arguments.extend_from_slice(extra);
    let (output, report) = clones(directory, &arguments);
    assert!(output.status.success(), "{}", stdout(&output));
    report
}

fn key<'a>(report: &'a Value, name: &str) -> &'a str {
    let qualified = format!("::{name}");
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| {
            let captured = function["facts"]["name"].as_str().unwrap();
            captured == name || captured.ends_with(&qualified)
        })
        .unwrap()["key"]
        .as_str()
        .unwrap()
}

struct Replay<'a> {
    directory: &'a Path,
    original: &'a Value,
    file: &'a Path,
    id: &'a str,
}

impl Replay<'_> {
    fn run(&self, extra: &[&str]) -> Output {
        let executable = cargo_bin("cargo-rstyle").canonicalize().unwrap();
        let labels = case(self.id);
        diagnostic_process::output(
            Command::new(executable)
                .current_dir(self.directory)
                .args([
                    "clones",
                    "--format",
                    "json",
                    "--extraction-profile",
                    "hir-data-v3",
                    labels["flag"].as_str().unwrap(),
                    "--replay-run",
                    self.original["run_directory"].as_str().unwrap(),
                    "--review-file",
                    self.file.to_str().unwrap(),
                ])
                .args(extra)
                .env("PATH", self.directory.join("no-tools"))
                .env("CARGO", self.directory.join("no-cargo"))
                .env("RSTYLE_DRIVER", self.directory.join("no-driver")),
        )
        .unwrap()
    }

    fn dismiss(&self) {
        let labels = case(self.id);
        let mut arguments = vec![labels["dismiss"].as_str().unwrap()];
        for endpoint in labels["endpoints"].as_array().unwrap() {
            arguments.push(key(self.original, endpoint.as_str().unwrap()));
        }
        arguments.extend([
            "--review-reason",
            "Keep this witnessed abstraction separate.",
        ]);
        let output = self.run(&arguments);
        assert!(output.status.success(), "{}", stdout(&output));
        counts(&parsed(&output), 1, 0);
    }
}

fn parsed(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&output.stderr)))
}

fn counts(report: &Value, hidden: usize, visible: usize) {
    let review = &report["abstraction_review"];
    assert_eq!(review["schema"], "rstyle.abstraction-review/1");
    assert_eq!(review["execution"], "completed");
    assert_eq!(review["findings"], hidden + visible);
    assert_eq!(review["hidden_findings"], hidden);
    assert_eq!(review["visible_findings"], visible);
    assert_eq!(
        review["dispatch_visible_findings"].as_u64().unwrap()
            + review["capability_visible_findings"].as_u64().unwrap(),
        visible as u64
    );
    assert_eq!(
        report["enum_dispatch"]["findings"].as_u64().unwrap()
            + report["capability_proposals"]["findings"].as_u64().unwrap(),
        (hidden + visible) as u64
    );
}

#[test]
fn explicit_dismissals_replay_without_original_tree_or_tools_and_lookup_never_writes() {
    for id in ["dispatch", "capability"] {
        let directory = workspace(id);
        let original = capture(directory.path(), id, &[]);
        let file = directory.path().join("reviews.json");
        let replay = Replay {
            directory: directory.path(),
            original: &original,
            file: &file,
            id,
        };
        let output = replay.run(&["--fail-on-findings"]);
        assert_eq!(output.status.code(), Some(1));
        counts(&parsed(&output), 0, 1);
        assert!(!file.exists());
        replay.dismiss();
        let bytes = std::fs::read(&file).unwrap();
        std::fs::remove_file(directory.path().join("src/lib.rs")).unwrap();
        std::fs::remove_file(directory.path().join("Cargo.toml")).unwrap();
        assert_source_free_review(&replay);
        assert_eq!(std::fs::read(file).unwrap(), bytes);
    }
}

fn assert_source_free_review(replay: &Replay<'_>) {
    let mut reviews = Vec::new();
    for detail in ["full", "compact"] {
        let output = replay.run(&["--fail-on-findings", "--report-detail", detail]);
        assert!(output.status.success(), "{}", stdout(&output));
        let report = parsed(&output);
        counts(&report, 1, 0);
        assert_eq!(report["verification_policy"], "strict-replay");
        let review = &report["abstraction_review"];
        assert_eq!(review["coverage"], "complete");
        assert_eq!(review["decisions"][0]["state"], "hidden_exact");
        assert_eq!(review["decisions"][0]["family"], case(replay.id)["family"]);
        reviews.push(review.clone());
    }
    assert_eq!(reviews[0], reviews[1]);
}

#[test]
fn line_binder_and_opaque_callee_body_edits_preserve_declaration_based_reviews() {
    for id in ["dispatch", "capability"] {
        let directory = workspace(id);
        let original = capture(directory.path(), id, &[]);
        let file = directory.path().join("reviews.json");
        Replay {
            directory: directory.path(),
            original: &original,
            file: &file,
            id,
        }
        .dismiss();
        let bytes = std::fs::read(&file).unwrap();
        let source = stable_edit(id);
        write_source(directory.path(), &source);
        let changed = capture(directory.path(), id, &[]);
        let output = Replay {
            directory: directory.path(),
            original: &changed,
            file: &file,
            id,
        }
        .run(&["--fail-on-findings"]);
        assert!(output.status.success(), "{}", stdout(&output));
        counts(&parsed(&output), 1, 0);
        assert_eq!(std::fs::read(file).unwrap(), bytes);
    }
}

fn stable_edit(id: &str) -> String {
    let source = case(id)["source"].as_str().unwrap().to_owned();
    let changed = match id {
        "dispatch" => source
            .replace("item:&Item", "renamed:&Item")
            .replace("match item", "match renamed")
            .replace(
                "let read=||self.value; read()+2",
                "let read=||self.value; read()+9",
            ),
        _ => source
            .replace("x:&Alpha", "renamed:&Alpha")
            .replace("x.used()", "renamed.used()")
            .replace("x.limit()", "renamed.limit()")
            .replace("let read=||3", "let read=||9"),
    };
    format!("\n\n// presentation movement\n{changed}")
}

#[test]
fn changed_ordered_roles_require_a_fresh_user_review() {
    for id in ["dispatch", "capability"] {
        let directory = workspace(id);
        let original = capture(directory.path(), id, &[]);
        let file = directory.path().join("reviews.json");
        Replay {
            directory: directory.path(),
            original: &original,
            file: &file,
            id,
        }
        .dismiss();
        let bytes = std::fs::read(&file).unwrap();
        let source = role_edit(id);
        write_source(directory.path(), &source);
        let changed = capture(directory.path(), id, &[]);
        let output = Replay {
            directory: directory.path(),
            original: &changed,
            file: &file,
            id,
        }
        .run(&["--fail-on-findings"]);
        assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
        let report = parsed(&output);
        counts(&report, 0, 1);
        assert_eq!(
            report["abstraction_review"]["decisions"][0]["state"],
            "stale"
        );
        assert_eq!(std::fs::read(file).unwrap(), bytes);
    }
}

fn role_edit(id: &str) -> String {
    let source = case(id)["source"].as_str().unwrap().to_owned();
    match id {
        "dispatch" => source
            .replace("pub fn amount(&self)", "pub fn alternate(&self)")
            .replace("b.amount()", "b.alternate()"),
        _ => source.replace("(y.size(),y.capacity())", "(y.capacity(),y.size())"),
    }
}

#[test]
fn changed_feature_context_does_not_inherit_an_abstraction_dismissal() {
    let id = "capability";
    let directory = workspace(id);
    let manifest = directory.path().join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(&manifest, text + "\n[features]\nextra=[]\n").unwrap();
    let original = capture(directory.path(), id, &[]);
    let file = directory.path().join("reviews.json");
    Replay {
        directory: directory.path(),
        original: &original,
        file: &file,
        id,
    }
    .dismiss();
    let changed = capture(directory.path(), id, &["--features", "extra"]);
    let output = Replay {
        directory: directory.path(),
        original: &changed,
        file: &file,
        id,
    }
    .run(&["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    let report = parsed(&output);
    counts(&report, 0, 1);
    assert_eq!(
        report["abstraction_review"]["decisions"][0]["state"],
        "stale"
    );
}

#[test]
fn hidden_known_capabilities_do_not_clear_an_unsupported_neighbor() {
    let id = "capability";
    let directory = workspace(id);
    let original = capture(directory.path(), id, &[]);
    let file = directory.path().join("reviews.json");
    Replay {
        directory: directory.path(),
        original: &original,
        file: &file,
        id,
    }
    .dismiss();
    let bytes = std::fs::read(&file).unwrap();
    let source = format!(
        "{} pub fn opaque(y:&Beta)->(u32,u32) {{({{let saved=y.size();saved}},y.capacity())}}",
        case(id)["source"].as_str().unwrap()
    );
    write_source(directory.path(), &source);
    let (output, mixed) = clones(
        directory.path(),
        &[
            "--extraction-profile",
            "hir-data-v3",
            "--capability-proposals",
        ],
    );
    assert_eq!(output.status.code(), Some(2));
    let output = Replay {
        directory: directory.path(),
        original: &mixed,
        file: &file,
        id,
    }
    .run(&["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(2), "{}", stdout(&output));
    let report = parsed(&output);
    counts(&report, 1, 0);
    assert_eq!(report["abstraction_review"]["coverage"], "incomplete");
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn invalid_selectors_reasons_and_duplicate_dismissals_never_write() {
    for id in ["dispatch", "capability"] {
        let directory = workspace(id);
        let original = capture(directory.path(), id, &[]);
        let file = directory.path().join("reviews.json");
        let replay = Replay {
            directory: directory.path(),
            original: &original,
            file: &file,
            id,
        };
        assert_invalid_dismissals(&replay);
        replay.dismiss();
        let bytes = std::fs::read(&file).unwrap();
        let labels = case(id);
        let mut arguments = vec![labels["dismiss"].as_str().unwrap()];
        for endpoint in labels["endpoints"].as_array().unwrap() {
            arguments.push(key(&original, endpoint.as_str().unwrap()));
        }
        arguments.extend([
            "--review-reason",
            "Repeated exact dismissal must not write.",
        ]);
        assert_eq!(replay.run(&arguments).status.code(), Some(2));
        assert_eq!(std::fs::read(file).unwrap(), bytes);
    }
}

fn assert_invalid_dismissals(replay: &Replay<'_>) {
    let labels = case(replay.id);
    for reason in ["reviewed", ""] {
        let first = labels["endpoints"][0].as_str().unwrap();
        let selector = if reason.is_empty() {
            key(replay.original, first)
        } else {
            "function:u999:f999"
        };
        let mut arguments = vec![labels["dismiss"].as_str().unwrap(), selector];
        if replay.id == "capability" {
            arguments.push(key(replay.original, "beta"));
        }
        arguments.extend(["--review-reason", reason]);
        assert_eq!(replay.run(&arguments).status.code(), Some(2));
        assert!(!replay.file.exists());
    }
}

#[test]
fn helper_v1_choices_survive_explicit_source_family_addition_without_cross_dismissal() {
    let directory = workspace("capability");
    write_source(
        directory.path(),
        "pub fn first(x:u32)->u32 {(x+1)*2} pub fn second(y:u32)->u32 {(y+3)*2}",
    );
    let (output, helper) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--helper-proposals"],
    );
    assert!(output.status.success());
    let file = directory.path().join("reviews.json");
    let output = helper_replay(
        directory.path(),
        &helper,
        &file,
        &[
            "--dismiss-helper-pair",
            key(&helper, "first"),
            key(&helper, "second"),
            "--review-reason",
            "Keep helper v1 behavior.",
        ],
    );
    assert!(output.status.success(), "{}", stdout(&output));
    let v1 = std::fs::read(&file).unwrap();
    let original_store: Value = serde_json::from_slice(&v1).unwrap();
    assert_eq!(original_store["schema_version"], 1);
    write_source(
        directory.path(),
        case("capability")["source"].as_str().unwrap(),
    );
    let original = capture(directory.path(), "capability", &[]);
    let replay = Replay {
        directory: directory.path(),
        original: &original,
        file: &file,
        id: "capability",
    };
    let output = replay.run(&["--fail-on-findings"]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    counts(&parsed(&output), 0, 1);
    assert_eq!(std::fs::read(&file).unwrap(), v1);
    replay.dismiss();
    assert_helper_preserved(directory.path(), &helper, &file, &original_store);
}

fn helper_replay(directory: &Path, original: &Value, file: &Path, extra: &[&str]) -> Output {
    let executable = cargo_bin("cargo-rstyle").canonicalize().unwrap();
    diagnostic_process::output(
        Command::new(executable)
            .current_dir(directory)
            .args([
                "clones",
                "--format",
                "json",
                "--extraction-profile",
                "hir-data-v3",
                "--helper-proposals",
                "--replay-run",
                original["run_directory"].as_str().unwrap(),
                "--review-file",
                file.to_str().unwrap(),
            ])
            .args(extra)
            .env("PATH", directory.join("no-tools"))
            .env("CARGO", directory.join("no-cargo"))
            .env("RSTYLE_DRIVER", directory.join("no-driver")),
    )
    .unwrap()
}

fn assert_helper_preserved(directory: &Path, original: &Value, file: &Path, previous: &Value) {
    let bytes = std::fs::read(file).unwrap();
    let store: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(store["schema_version"], 2);
    assert_eq!(store["entries"], previous["entries"]);
    assert_eq!(store["source_entries"].as_array().unwrap().len(), 1);
    let output = helper_replay(directory, original, file, &["--fail-on-findings"]);
    assert!(output.status.success(), "{}", stdout(&output));
    let report = parsed(&output);
    assert_eq!(report["review"]["hidden_findings"], 1);
    assert_eq!(report["review"]["pairs"][0]["state"], "hidden_exact");
    assert_eq!(std::fs::read(file).unwrap(), bytes);
}

#[test]
fn malformed_source_review_fields_are_rejected_without_rewriting_the_store() {
    let directory = workspace("capability");
    let original = capture(directory.path(), "capability", &[]);
    let file = directory.path().join("reviews.json");
    let replay = Replay {
        directory: directory.path(),
        original: &original,
        file: &file,
        id: "capability",
    };
    replay.dismiss();
    let bytes = std::fs::read(&file).unwrap();
    let mut invalid: Value = serde_json::from_slice(&bytes).unwrap();
    invalid["unrecognized_review_authority"] = true.into();
    let invalid = serde_json::to_vec(&invalid).unwrap();
    std::fs::write(&file, &invalid).unwrap();
    assert_eq!(replay.run(&[]).status.code(), Some(2));
    assert_eq!(std::fs::read(file).unwrap(), invalid);
}

#[test]
fn strict_matching_thresholds_do_not_change_local_abstraction_review_identity() {
    for id in ["dispatch", "capability"] {
        let directory = workspace(id);
        let original = capture(directory.path(), id, &[]);
        let file = directory.path().join("reviews.json");
        let replay = Replay {
            directory: directory.path(),
            original: &original,
            file: &file,
            id,
        };
        replay.dismiss();
        let bytes = std::fs::read(&file).unwrap();
        let config = directory.path().join("matching.json");
        std::fs::write(
            &config,
            r#"{"numerical_threshold":"0.5","scoring":"cross_product_cosine"}"#,
        )
        .unwrap();
        let output = replay.run(&[
            "--matching-config",
            config.to_str().unwrap(),
            "--fail-on-findings",
        ]);
        assert!(output.status.success(), "{}", stdout(&output));
        let report = parsed(&output);
        counts(&report, 1, 0);
        assert_eq!(
            report["abstraction_review"]["decisions"][0]["state"],
            "hidden_exact"
        );
        assert_eq!(
            report["matching"]["effective_config"]["numerical_threshold"],
            "1/2"
        );
        assert_eq!(report["matching"]["scoring"], "cross_product_cosine");
        assert_eq!(std::fs::read(file).unwrap(), bytes);
    }
}

#[test]
fn changed_inline_module_names_do_not_inherit_an_unrelated_review() {
    for id in ["dispatch", "capability"] {
        let directory = workspace(id);
        let source = case(id)["source"].as_str().unwrap().to_owned();
        write_source(directory.path(), &format!("pub mod original {{{source}}}"));
        let original = capture(directory.path(), id, &[]);
        let file = directory.path().join("reviews.json");
        Replay {
            directory: directory.path(),
            original: &original,
            file: &file,
            id,
        }
        .dismiss();
        let bytes = std::fs::read(&file).unwrap();
        write_source(directory.path(), &format!("pub mod relocated {{{source}}}"));
        let changed = capture(directory.path(), id, &[]);
        let output = Replay {
            directory: directory.path(),
            original: &changed,
            file: &file,
            id,
        }
        .run(&["--fail-on-findings"]);
        assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
        let report = parsed(&output);
        counts(&report, 0, 1);
        assert_eq!(
            report["abstraction_review"]["decisions"][0]["state"],
            "visible_new"
        );
        assert_eq!(std::fs::read(file).unwrap(), bytes);
    }
}
