use super::propagation::checked;
use super::*;

const MODULES: usize = rstyle_pdg::nominal_boundary::census::MAX_FILES + 1;
const LARGE_OPTIONS: &[&str] = &["--compiler-advisory-only", "--compiler-plans"];
const BOUNDED: &str = r#"
use core::num::NonZeroU8;
pub struct Reader;
impl Reader { pub fn read(&self, values:&[u8])->usize { values.len() } }
macro_rules! generated {()=>{impl Reader {pub fn generated(&self)->u32 {7}}}}
generated!();
pub fn call(values:&[u8])->usize { Reader.read(values) }
pub fn nonzero(value:NonZeroU8)->u8 { value.get() }
"#;
const UNIT_KEYS: &[&str] = &[
    rstyle_pdg::nominal_boundary::census::KEY,
    rstyle_pdg::dependency_namespaces::capture::KEY,
    rstyle_pdg::generated_sources::capture::KEY,
    rstyle_pdg::data_library::KEY,
    rstyle_pdg::inherent_calls::UNIT_KEY,
];

fn large_fixture() -> TempDir {
    let directory = clone_workspace();
    let mut root = String::new();
    for index in 0..MODULES {
        root.push_str(&format!("pub mod m{index};\n"));
        let source = if index == 0 {
            "pub fn leaf(a:u128,b:u128,c:u128,d:u128,e:u128)->u128 {a+b+c+d+e}".into()
        } else {
            format!("pub fn original()->u32 {{{index}}}")
        };
        std::fs::write(directory.path().join(format!("src/m{index}.rs")), source).unwrap();
    }
    root.push_str("pub fn caller(a:u128,b:u128,c:u128,d:u128,e:u128)->u128 {m0::leaf(a,b,c,d,e)}");
    std::fs::write(directory.path().join("src/lib.rs"), root).unwrap();
    directory
}

fn large_advice(report: &Value) {
    let advisory = &report["compiler_advisory"];
    let unit = &advisory["units"][0];
    assert_eq!(advisory["complete"], true);
    assert_eq!(unit["expected_functions"], MODULES + 1);
    assert_eq!(unit["observations"].as_array().unwrap().len(), MODULES + 1);
    assert_eq!(unit["unavailable"], serde_json::json!([]));
    let diagnostics = unit["evaluation"]["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().all(|row| row["rule_id"] == "fn_params"));
    let groups = advisory["planning"]["groups"].as_array().unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0]["step_owners"].as_array().unwrap().len(), 2);
    let inventory = report["owned_source_inventory"][0]["source_functions"]
        .as_array()
        .unwrap();
    assert_eq!(inventory.len(), MODULES + 1);
    assert!(inventory.iter().all(|row| row["presence"] == "active"));
}

fn strict_census_rejects(directory: &Path) {
    let output = rstyle(
        directory,
        &[
            "clones",
            "--format",
            "json",
            "--extraction-profile",
            "hir-data-v3",
            "--compiler-advisory",
        ],
    );
    checked(&output, 2);
    let diagnostic = format!(
        "{}{}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(diagnostic.contains("Source census exceeds file/byte budget"));
}

fn export_and_dismiss(directory: &Path, original: &Value) {
    let mut options = LARGE_OPTIONS.to_vec();
    options.extend_from_slice(&[
        "--write-compiler-baseline",
        "baseline.json",
        "--review-file",
        "reviews.json",
        "--dismiss-compiler-diagnostic",
        "0",
        "0",
        "--review-reason",
        "Reviewed original large-project parameter boundary",
    ]);
    let (output, report) = super::reviews::replay(directory, original, &options);
    checked(&output, 0);
    assert_eq!(
        report["compiler_advisory"]["diagnostic_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    let baseline: Value =
        serde_json::from_slice(&std::fs::read(directory.join("baseline.json")).unwrap()).unwrap();
    assert_eq!(baseline["entries"].as_array().unwrap().len(), 2);
}

fn source_free_lookup(directory: &Path, original: &Value) {
    std::fs::remove_dir_all(directory.join("src")).unwrap();
    std::fs::remove_file(directory.join("Cargo.toml")).unwrap();
    let mut options = LARGE_OPTIONS.to_vec();
    options.extend_from_slice(&[
        "--compiler-baseline",
        "baseline.json",
        "--fail-on-new-compiler",
        "--review-file",
        "reviews.json",
    ]);
    let (output, report) = super::reviews::replay(directory, original, &options);
    checked(&output, 0);
    let advisory = &report["compiler_advisory"];
    assert_eq!(advisory["diagnostic_baseline"]["coverage"], "complete");
    assert_eq!(advisory["diagnostic_baseline"]["new_findings"], 0);
    assert_eq!(
        advisory["diagnostic_review"]["decisions"][0]["state"],
        "hidden_exact"
    );
    assert_eq!(
        advisory["units"][0]["evaluation"],
        original["compiler_advisory"]["units"][0]["evaluation"]
    );
    assert_eq!(
        advisory["planning"],
        original["compiler_advisory"]["planning"]
    );
}

#[test]
fn advisory_large_units_keep_original_measurements_reviews_and_source_free_plans() {
    let directory = large_fixture();
    let (output, report) = super::propagation::capture(directory.path(), LARGE_OPTIONS);
    checked(&output, 0);
    large_advice(&report);
    strict_census_rejects(directory.path());
    export_and_dismiss(directory.path(), &report);
    source_free_lookup(directory.path(), &report);
}

fn options(report: &Value) -> impl Iterator<Item = &Value> {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|function| &function["facts"]["build"]["frontend_options"])
}

fn strict_unit_metadata(report: &Value) {
    for key in UNIT_KEYS {
        assert!(
            options(report).any(|options| options.get(*key).is_some()),
            "{key}"
        );
    }
    for (key, roster) in [
        (rstyle_pdg::nominal_boundary::census::KEY, "files"),
        (rstyle_pdg::data_library::KEY, "libraries"),
        (rstyle_pdg::inherent_calls::UNIT_KEY, "definitions"),
        (rstyle_pdg::generated_sources::capture::KEY, "outcomes"),
    ] {
        assert!(
            options(report).any(|options| {
                options[key][roster]
                    .as_array()
                    .is_some_and(|rows| !rows.is_empty())
            }),
            "{key}: {roster}"
        );
    }
}

#[test]
fn advisory_omits_strict_unit_metadata_without_losing_generated_owner_distinctions() {
    let requested = ["--compiler-patterns", "--compiler-plans"];
    let (directory, output, combined) = capture(BOUNDED, &requested);
    checked(&output, 2);
    strict_unit_metadata(&combined);
    let mut requested = requested.to_vec();
    requested.push("--compiler-advisory-only");
    let (output, raw) = super::propagation::capture(directory.path(), &requested);
    checked(&output, 2);
    assert_eq!(raw["compiler_advisory"], combined["compiler_advisory"]);
    assert_eq!(
        raw["owned_source_inventory"],
        combined["owned_source_inventory"]
    );
    assert_eq!(
        raw["compiler_advisory"]["units"][0]["unavailable"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    for options in options(&raw) {
        assert!(UNIT_KEYS.iter().all(|key| options.get(*key).is_none()));
        assert!(options.get("source_file").is_some());
        assert!(
            options
                .get(rstyle_pdg::nominal_boundary::CONTEXT_KEY)
                .is_some()
        );
    }
}
