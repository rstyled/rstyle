use super::*;

#[test]
fn subslices_and_assertion_receivers_use_each_pinned_ci_authority() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let output = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(output.status.success());
        if !Path::new(String::from_utf8(output.stdout).unwrap().trim()).is_dir() {
            continue;
        }
        check_target(target);
    }
}

fn check_target(target: &str) {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub fn window(a: &[u8], lo: usize, hi: usize) -> &[u8] { &a[lo..hi] }
        pub fn same(a: &[u8], lo: usize, hi: usize) -> &[u8] { &a[lo..hi] }
        pub fn read(a: &[u8], lo: usize, hi: usize, i: usize) -> u8 {
            let view = &a[lo..hi]; view[i]
        }
        pub fn saved<'a>(a: &'a [u8], b: &'a [u8], lo: usize, hi: usize) -> &'a [u8] {
            let mut selected = a; let view = &selected[lo..hi]; selected = b; view
        }
        pub fn snapshot<'a>(a: &'a [u8], b: &'a [u8]) -> &'a [u8] {
            let mut selected = a; &selected[{ selected = b; 0 }..1]
        }
        pub fn check(bytes: &[u8]) -> usize { assert!(bytes.len() > 0); bytes.len() }
        pub fn inclusive(a: &[u8], hi: usize) -> &[u8] { &a[0..=hi] }
        pub fn write(a: &mut [u8], lo: usize, hi: usize) -> &mut [u8] { &mut a[lo..hi] }
    "#,
    )
    .unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--target", target, "--extraction-profile", "hir-data-v3"],
    );
    assert!(output.status.success(), "{target}: {report}");
    assert_eq!(report["capture_complete"], true, "{target}: {report}");
    assert_eq!(report["assessment_complete"], true, "{target}: {report}");
    assert_eq!(report["functions"].as_array().unwrap().len(), 8);
    for function in report["functions"].as_array().unwrap() {
        let name = function["facts"]["name"].as_str().unwrap();
        assert_eq!(
            function["outcome"],
            if matches!(name, "inclusive" | "write") {
                "excluded"
            } else {
                "admitted"
            },
            "{target}: {function}"
        );
        if name == "snapshot" {
            check_snapshot_origin(function, target);
        }
    }
    check_aggregate_documents(&report);
    check_subslice_corruption(&report);
    check_equivalent_matching(&report, ["window", "same"]);
}

fn check_snapshot_origin(function: &Value, target: &str) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let indexed = find_kind(&facts.body, "expr_index").unwrap();
    let roots = rstyle_pdg::data_origins::origins(&indexed.children[0], &facts).unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(
        roots[0].parameter, 0,
        "{target}: base must precede rebinding"
    );
}
