use super::*;

fn selection(paths: &[&str]) -> Selection {
    Selection::from_bytes(
        &serde_json::to_vec(&serde_json::json!({"schema":SCHEMA,"paths":paths})).unwrap(),
    )
    .unwrap()
}

fn owner(path: Option<&str>, complete: bool, visible_findings: usize) -> Owner<'_> {
    Owner {
        path,
        complete,
        outside_scope: false,
        visible_findings,
    }
}

#[test]
fn outside_scope_is_explicit_and_does_not_cover_a_missing_applicable_owner() {
    let selected = selection(&["src/lib.rs"]);
    let outside = || Owner {
        path: Some("src/lib.rs"),
        complete: false,
        outside_scope: true,
        visible_findings: 0,
    };
    let report = evaluate(&selected, [outside()]);
    assert!(report.permits_success());
    assert_eq!(report.paths[0].outside_scope_functions, 1);
    assert_eq!(report.paths[0].unavailable_functions, 0);
    let report = evaluate(&selected, [outside(), owner(Some("src/lib.rs"), false, 0)]);
    assert!(!report.permits_success());
    assert_eq!(report.paths[0].outside_scope_functions, 1);
    assert_eq!(report.paths[0].unavailable_functions, 1);
}

#[test]
fn manifest_requires_exact_schema_fields_and_valid_json() {
    for bytes in [
        b"{}".as_slice(),
        br#"{"schema":"rstyle.changed-files/2","paths":[]}"#,
        br#"{"schema":"rstyle.changed-files/1","paths":[],"extra":true}"#,
        br#"{"schema":"rstyle.changed-files/1","paths":"src/lib.rs"}"#,
        br#"{"schema":"rstyle.changed-files/1","paths":[],"paths":[]}"#,
        b"\xff",
    ] {
        assert!(Selection::from_bytes(bytes).is_err(), "{bytes:?}");
    }
}

#[test]
fn paths_are_strictly_normalized_portable_relative_rust_files() {
    for path in [
        "",
        "/src/lib.rs",
        "//host/lib.rs",
        "./src/lib.rs",
        "src/../lib.rs",
        "src/./lib.rs",
        "src//lib.rs",
        "src/lib.rs/",
        "../lib.rs",
        "C:/src/lib.rs",
        "src\\lib.rs",
        "src:lib.rs",
        "src/lib.txt",
        "src/lib.RS",
        "src/\0lib.rs",
        "src/\nlib.rs",
        "src/\u{7f}lib.rs",
    ] {
        assert!(validate_path(path).is_err(), "{path:?}");
    }
    for path in [
        "lib.rs",
        "src/lib.rs",
        "crates/παράδειγμα/値.rs",
        "src/my file.rs",
    ] {
        assert!(validate_path(path).is_ok(), "{path:?}");
    }
}

#[test]
fn path_and_raw_path_count_limits_apply_before_deduplication() {
    let boundary = format!("{}.rs", "a".repeat(MAX_PATH_BYTES - 3));
    assert!(validate_path(&boundary).is_ok());
    assert!(validate_path(&format!("a{boundary}")).is_err());
    let multibyte = format!("{}.rs", "é".repeat(MAX_PATH_BYTES / 2));
    assert!(validate_path(&multibyte).is_err());
    let paths = vec!["src/lib.rs"; MAX_PATHS];
    assert_eq!(selection(&paths).paths, ["src/lib.rs"]);
    let oversized = serde_json::json!({"schema":SCHEMA,"paths":vec!["src/lib.rs"; MAX_PATHS + 1]});
    assert!(Selection::from_bytes(&serde_json::to_vec(&oversized).unwrap()).is_err());
}

#[test]
fn bounded_file_loading_accepts_exact_limit_and_rejects_one_more_byte() {
    let mut bytes = serde_json::to_vec(&serde_json::json!({"schema":SCHEMA,"paths":[]})).unwrap();
    bytes.resize(MAX_BYTES, b' ');
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("changed.json");
    std::fs::write(&path, &bytes).unwrap();
    assert!(Selection::load(&path).unwrap().paths.is_empty());
    bytes.push(b' ');
    std::fs::write(&path, bytes).unwrap();
    assert!(Selection::load(&path).unwrap_err().contains("1 MiB"));
    assert!(Selection::load(&directory.path().join("missing.json")).is_err());
}

#[test]
fn selection_is_sorted_deduplicated_and_exact_without_filesystem_access() {
    let selected = selection(&["src/z.rs", "src/a.rs", "src/z.rs"]);
    assert_eq!(selected.paths, ["src/a.rs", "src/z.rs"]);
    assert!(selected.contains("src/a.rs"));
    assert!(!selected.contains("other/a.rs"));
    assert!(!selected.contains("src/A.rs"));
    assert!(!selected.contains("src/./a.rs"));
    assert!(!selected.contains("src/a.rs.bak"));
}

#[test]
fn selected_counts_keep_distinct_owner_contexts_and_ignore_unselected_findings() {
    let selected = selection(&["src/a.rs", "src/z.rs"]);
    let report = evaluate(
        &selected,
        [
            owner(Some("src/a.rs"), true, 2),
            owner(Some("src/a.rs"), true, 3),
            owner(Some("src/z.rs"), true, 0),
            owner(Some("other/a.rs"), false, 99),
        ],
    );
    assert!(report.permits_success());
    assert_eq!(report.visible_findings, 5);
    assert_eq!(report.paths[0].matched_functions, 2);
    assert_eq!(report.paths[1].matched_functions, 1);
    assert_eq!(report.paths[1].visible_findings, 0);
    assert_eq!(report.unclassified_functions, 0);
}

#[test]
fn unmatched_requested_paths_remain_explicitly_unobserved_and_incomplete() {
    let selected = selection(&["src/missing.rs"]);
    let report = evaluate(&selected, [owner(Some("src/other.rs"), true, 0)]);
    assert!(!report.permits_success());
    assert_eq!(report.visible_findings, 0);
    assert_eq!(report.paths[0].status, Status::Unobserved);
    assert_eq!(report.paths[0].matched_functions, 0);
    assert_eq!(report.paths[0].coverage, Coverage::Incomplete);
    assert!(report.render().contains("Unobserved"));
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["paths"][0]["status"], "unobserved");
    assert_eq!(json["coverage"], "incomplete");
}

#[test]
fn incomplete_selected_owners_preserve_positive_witnesses_and_coverage_gap() {
    let selected = selection(&["src/lib.rs"]);
    let report = evaluate(
        &selected,
        [
            owner(Some("src/lib.rs"), true, 1),
            owner(Some("src/lib.rs"), false, 2),
        ],
    );
    assert!(!report.permits_success());
    assert_eq!(report.visible_findings, 3);
    assert_eq!(report.paths[0].matched_functions, 2);
    assert_eq!(report.paths[0].unavailable_functions, 1);
    assert_eq!(report.paths[0].status, Status::Observed);
    assert_eq!(report.paths[0].coverage, Coverage::Incomplete);
}

#[test]
fn unclassified_owner_cannot_be_assumed_outside_a_nonempty_selection() {
    let selected = selection(&["src/lib.rs"]);
    let report = evaluate(
        &selected,
        [
            owner(Some("src/lib.rs"), true, 0),
            owner(None, true, 10),
            owner(None, false, 20),
        ],
    );
    assert!(!report.permits_success());
    assert_eq!(report.unclassified_functions, 2);
    assert_eq!(report.visible_findings, 0);
    assert_eq!(report.paths[0].coverage, Coverage::Complete);
}

#[test]
fn explicit_empty_selection_is_complete_even_with_unclassified_owners() {
    let selected = selection(&[]);
    let report = evaluate(&selected, [owner(None, false, 99)]);
    assert!(report.permits_success());
    assert!(report.paths.is_empty());
    assert_eq!(report.visible_findings, 0);
    assert_eq!(report.unclassified_functions, 0);
}

#[test]
fn report_paths_borrow_the_selection_and_do_not_retain_owner_paths() {
    let selected = selection(&["src/lib.rs"]);
    let report = {
        let temporary_path = String::from("src/lib.rs");
        evaluate(&selected, [owner(Some(&temporary_path), true, 1)])
    };
    assert!(std::ptr::eq(
        report.paths[0].path.as_ptr(),
        selected.paths[0].as_ptr()
    ));
    assert_eq!(report.visible_findings, 1);
}

#[test]
fn empty_selection_consumes_owners_so_adapters_can_annotate_every_function() {
    let selected = selection(&[]);
    let visits = std::cell::Cell::new(0);
    let owners = [owner(None, false, 99), owner(Some("src/lib.rs"), true, 1)]
        .into_iter()
        .inspect(|_| visits.set(visits.get() + 1));
    let report = evaluate(&selected, owners);
    assert_eq!(visits.get(), 2);
    assert!(report.permits_success());
    assert_eq!(report.unclassified_functions, 0);
    assert_eq!(report.visible_findings, 0);
}

#[test]
fn independent_diagnostic_counts_do_not_promote_unobserved_files() {
    let selection = selection(&["src/known.rs", "src/missing.rs"]);
    let mut report = evaluate(&selection, [owner(Some("src/known.rs"), true, 0)]);
    report.include_findings(
        [
            "src/known.rs",
            "src/known.rs",
            "src/missing.rs",
            "src/outside.rs",
        ]
        .into_iter(),
    );
    assert_eq!(report.visible_findings, 3);
    assert_eq!(report.paths[0].visible_findings, 2);
    assert_eq!(report.paths[1].visible_findings, 1);
    assert_eq!(report.paths[1].status, Status::Unobserved);
    assert!(!report.permits_success());
}

#[test]
fn owned_selection_report_retains_counts_after_manifest_owner_is_dropped() {
    let report = {
        let selection = selection(&["src/known.rs"]);
        evaluate(&selection, [owner(Some("src/known.rs"), true, 2)]).into_owned()
    };
    assert_eq!(report.paths[0].path, "src/known.rs");
    assert_eq!(report.visible_findings, 2);
    assert!(report.permits_success());
}
