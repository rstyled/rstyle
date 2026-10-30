use super::float_kernels::{function, reject_resealed};
use super::*;

const ORIGINAL: &str =
    include_str!("../../../../tests/fixtures/function-families/base64ct-length.rs");
const SOURCE: &str = r#"
pub fn caller(n: usize, padded: bool) -> Option<usize> { encoded_len_inner(n, padded) }
pub fn same(n: usize, padded: bool) -> Option<usize> { encoded_len_inner(n, padded) }
#[derive(Clone, Debug)]
pub struct Line<'a> { remaining: &'a [u8] }
impl<'a> Line<'a> {
    fn len(&self) -> usize { self.remaining.len() }
    fn is_empty(&self) -> bool { self.len() == 0 }
}
pub fn allocated() -> usize { vec![1u8, 2].len() }
pub fn formatted(n: usize) -> usize { format!("{}", n).len() }
pub fn discarded(n: usize) -> usize { let _ = vec![n]; n }
"#;
const IMPORT: &str = "#[macro_use] extern crate alloc;";

fn capture(target: Option<&str>, prefix: &str, extra: &str) -> (TempDir, Value) {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("README.md"),
        "# Immutable original input\n",
    )
    .unwrap();
    std::fs::write(directory.path().join("src/lib.rs"), format!("{prefix}\n#![doc = include_str!(\"../README.md\")]\n{IMPORT}\n{ORIGINAL}\n{SOURCE}\n{extra}")).unwrap();
    let mut args = vec!["--extraction-profile", "hir-data-v3"];
    if let Some(target) = target {
        args.extend(["--target", target]);
    }
    let (output, report) = clones(directory.path(), &args);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn pinned_alloc_imports_compose_original_lengths_getters_and_helpers_without_macro_call_permission()
{
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let library = Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(library.status.success());
        if !Path::new(String::from_utf8_lossy(&library.stdout).trim()).is_dir() {
            continue;
        }
        for prefix in ["", "#![no_std]"] {
            let (_directory, report) = capture(Some(target), prefix, "");
            check_family(&report);
        }
    }
}

fn check_family(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 10);
    for entry in functions {
        let name = entry["facts"]["name"].as_str().unwrap();
        assert_eq!(
            entry["outcome"],
            if ["encoded_len_inner", "caller", "same"].contains(&name)
                || name.ends_with("::len")
                || name.ends_with("::is_empty")
            {
                "admitted"
            } else {
                "excluded"
            },
            "{name}: {}",
            entry["reasons"]
        );
    }
    super::word_classifiers::verify_admissions(report);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true);
    }
    let pairs = report["matching"]["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 10);
    let left = &function(report, "caller")["key"];
    let right = &function(report, "same")["key"];
    assert!(pairs.iter().any(|pair| {
        ((&pair["first"] == left && &pair["second"] == right)
            || (&pair["first"] == right && &pair["second"] == left))
            && pair["decision"] == "Accepted"
    }));
}

#[test]
fn external_import_resealed_source_library_and_complete_namespace_mutations_reject() {
    let (_directory, report) = capture(None, "", "");
    let entry = function(&report, "encoded_len_inner");
    assert_eq!(entry["outcome"], "admitted", "{}", entry["reasons"]);
    for mutation in [
        "owner",
        "span",
        "digest",
        "library",
        "root",
        "macro",
        "macro_path",
        "macro_impl",
        "kind",
        "visibility",
        "missing",
        "extra",
        "duplicate",
        "missing_import",
        "duplicate_import",
    ] {
        let mut bundle = shared_evidence::wire(entry);
        let imports = &mut bundle["functions"][0]["build"]["frontend_options"]
            [rstyle_pdg::nominal_boundary::census::KEY]["value"]["external_imports"];
        mutate(imports, mutation);
        reject_resealed(bundle, mutation);
    }
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let mut pair = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    pair.pdg.profile.version = "3.73.0".into();
    pair.cfg.profile.version = "3.73.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

fn mutate(imports: &mut Value, mutation: &str) {
    match mutation {
        "owner" => imports[0]["declaration"]["path"] = "/outside/lib.rs".into(),
        "span" => imports[0]["declaration"]["span"]["start"] = 0.into(),
        "digest" => imports[0]["declaration"]["digest"] = "changed".into(),
        "library" => imports[0]["exports"]["library"]["crate_hash"] = "changed".into(),
        "root" => imports[0]["exports"]["library"]["definition_hash"] = "changed".into(),
        "macro" => {
            imports[0]["exports"]["macros"][0]["definition"]["definition_hash"] = "changed".into()
        }
        "macro_path" => {
            imports[0]["exports"]["macros"][0]["definition"]["path"] = "std::panic".into()
        }
        "macro_impl" => {
            imports[0]["exports"]["macros"][0]["definition"]["implementation"] =
                imports[0]["exports"]["library"].clone()
        }
        "kind" => imports[0]["exports"]["macros"][0]["kind"] = "derive".into(),
        "visibility" => imports[0]["exports"]["macros"][0]["public"] = false.into(),
        "missing" => {
            imports[0]["exports"]["macros"]
                .as_array_mut()
                .unwrap()
                .pop();
        }
        "extra" => {
            let mut item = imports[0]["exports"]["macros"][0].clone();
            item["name"] = "cfg_if".into();
            imports[0]["exports"]["macros"]
                .as_array_mut()
                .unwrap()
                .push(item);
        }
        "duplicate" => {
            let item = imports[0]["exports"]["macros"][0].clone();
            imports[0]["exports"]["macros"]
                .as_array_mut()
                .unwrap()
                .push(item);
        }
        "missing_import" => *imports = serde_json::json!([]),
        "duplicate_import" => {
            let item = imports[0].clone();
            imports.as_array_mut().unwrap().push(item);
        }
        _ => unreachable!(),
    }
}

#[test]
fn pinned_import_does_not_hide_neighboring_macro_items_or_protected_definitions() {
    for extra in [
        "macro_rules! hidden { () => { struct Neighbor; }; } hidden!();",
        "macro_rules! Copy { ($($tt:tt)*) => {}; }",
    ] {
        let (_directory, report) = capture(None, "", extra);
        assert_eq!(
            function(&report, "encoded_len_inner")["outcome"],
            "excluded"
        );
    }
}

#[test]
fn selective_and_renamed_external_imports_remain_outside_contract() {
    for import in [
        "#[macro_use(vec)] extern crate alloc;",
        "#[macro_use] extern crate alloc as other;",
    ] {
        let directory = clone_workspace();
        std::fs::write(
            directory.path().join("src/lib.rs"),
            format!("{import}\n{ORIGINAL}"),
        )
        .unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        assert!(output.status.success(), "{}", report["capture_failures"]);
        assert_eq!(
            function(&report, "encoded_len_inner")["outcome"],
            "excluded"
        );
    }
}

#[test]
fn external_crate_named_alloc_cannot_substitute_for_pinned_system_library() {
    let directory = clone_workspace();
    std::fs::create_dir(directory.path().join("fake")).unwrap();
    std::fs::write(
        directory.path().join("fake/Cargo.toml"),
        "[package]\nname='alloc'\nversion='0.0.0'\nedition='2021'\n[lib]\npath='lib.rs'\n",
    )
    .unwrap();
    std::fs::write(
        directory.path().join("fake/lib.rs"),
        "#[macro_export] macro_rules! cfg_if { ($($tt:tt)*) => {}; }\n",
    )
    .unwrap();
    let manifest = directory.path().join("Cargo.toml");
    let mut content = std::fs::read_to_string(&manifest).unwrap();
    content.push_str("\n[dependencies]\nalloc={path='fake'}\n");
    std::fs::write(manifest, content).unwrap();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{IMPORT}\n{ORIGINAL}"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    let entry = function(&report, "encoded_len_inner");
    assert_eq!(entry["outcome"], "excluded");
    assert!(entry["reasons"].as_array().unwrap().iter().any(|reason| {
        reason
            .as_str()
            .unwrap()
            .contains("overridden by an extern dependency")
    }));
}

#[test]
fn pinned_import_invocation_overrides_cannot_reuse_an_exact_inventory() {
    let (_directory, report) = capture(None, "", "");
    let entry = function(&report, "encoded_len_inner");
    assert_eq!(entry["outcome"], "admitted");
    for arguments in [
        vec!["--extern=alloc=/forged/alloc.rlib"],
        vec!["--extern", "priv:core=/forged/core.rlib"],
        vec!["--extern=std=/forged/std.rlib"],
        vec!["@unexpanded.rsp"],
        vec!["--extern"],
    ] {
        let mut bundle = shared_evidence::wire(entry);
        let invocation = bundle["functions"][0]["build"]["frontend_options"]["invocation"]
            .as_array_mut()
            .unwrap();
        invocation.extend(arguments.iter().map(|argument| Value::from(*argument)));
        reject_resealed(bundle, &arguments.join(" "));
    }
}

#[test]
fn import_input_closure_preserves_existing_nested_scalar_helpers_during_strict_replay() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{IMPORT}\npub fn owner(a: String) -> String {{ fn nested(n: usize) -> usize {{ n }} a }}"),
    ).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 2);
    let nested = functions
        .iter()
        .find(|entry| {
            entry["facts"]["name"]
                .as_str()
                .unwrap()
                .ends_with("::nested")
        })
        .unwrap();
    assert_eq!(nested["outcome"], "admitted", "{}", nested["reasons"]);
    assert_eq!(function(&report, "owner")["outcome"], "excluded");
    super::word_classifiers::verify_admissions(&report);
    for mutation in ["digest", "span", "extra", "duplicate_import"] {
        let mut bundle = shared_evidence::wire(nested);
        let imports = &mut bundle["functions"][shared_evidence::index(nested)]["build"]["frontend_options"]
            [rstyle_pdg::nominal_boundary::census::KEY]["value"]["external_imports"];
        mutate(imports, mutation);
        reject_resealed(bundle, mutation);
    }
    let mut changed: rstyle_pdg::HirFnFacts =
        serde_json::from_value(nested["facts"].clone()).unwrap();
    changed
        .build
        .frontend_options
        .insert("source_file".into(), "/unowned/source.rs".into());
    shared_evidence::reject_facts(nested, &changed);
}
