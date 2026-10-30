//! Borrowed access permission must not certify Copy or drop-free ownership.
use super::*;

#[test]
fn borrowed_scalar_owners_compose_without_owned_permission() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../tests/fixtures/function-families/borrowed-scalar-owners.rs"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    check_outcomes(&report);
}

fn check_outcomes(report: &Value) {
    assert_eq!(report["functions"].as_array().unwrap().len(), 10);
    for function in report["functions"].as_array().unwrap() {
        let name = function["facts"]["name"].as_str().unwrap();
        let positive = [
            "ScalarRange::is_valid",
            "ScalarRange::is_ascii",
            "ScalarRange::owned",
            "borrowed",
            "same",
        ]
        .contains(&name);
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{function}"
        );
        if name == "ScalarRange::owned" {
            // Fresh return permission does not grant owned parameters or locals.
            crate::fresh_scalar_owners::verify_boundary(function);
        }
    }
    check_aggregate_documents(report);
    check_repeated_reads(report);
    assert_eq!(report["pair_evaluation_complete"], true);
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 10);
}

fn check_repeated_reads(report: &Value) {
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "ScalarRange::is_ascii")
        .unwrap();
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = shared_evidence::pair(&unit, function);
    let reads = pair
        .pdg
        .nodes
        .iter()
        .flat_map(|node| &node.attributes.reads)
        .filter(|read| read.location_id.is_some())
        .count();
    assert_eq!(reads, 3);
    let mut changed = rstyle_pdg::producer::produce(&facts).unwrap();
    let call = changed
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.reads.len() == 2)
        .unwrap();
    call.attributes.reads.pop();
    assert!(rstyle_pdg::data_documents::verify(&changed, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&changed, &facts).is_err());
}

#[test]
fn borrowed_scalar_owners_preserve_installed_ci_contexts() {
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
        let directory = clone_workspace();
        std::fs::write(
            directory.path().join("src/lib.rs"),
            include_str!("../../../../tests/fixtures/function-families/borrowed-scalar-owners.rs"),
        )
        .unwrap();
        let (output, report) = clones(
            directory.path(),
            &["--target", target, "--extraction-profile", "hir-data-v3"],
        );
        assert!(output.status.success(), "{target}: {report}");
        check_outcomes(&report);
    }
}

#[test]
fn borrowed_scalar_owners_keep_reference_fields_and_generics_closed() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        pub enum Neighbor { Empty }
        pub struct RefField<'a> { pub value: &'a u32 }
        pub fn reference(value: &RefField<'_>) -> u32 { *value.value }
        pub struct Generic<T> { pub value: T }
        pub fn generic(value: &Generic<u32>) -> u32 { value.value }
        pub struct Owner { pub n: u32 }
        pub fn construct(value: &Owner) -> u32 {
            let owned = Owner { n: value.n }; owned.n
        }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 3);
    for function in functions {
        assert_eq!(function["outcome"], "excluded", "{function}");
    }
}

#[test]
fn borrowed_scalar_owners_do_not_activate_historical_profiles() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../tests/fixtures/function-families/borrowed-scalar-owners.rs"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v2"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 10);
    for function in functions {
        assert_eq!(function["outcome"], "excluded", "{function}");
    }
}
