//! Source expectations are fixed before evaluating admission: first true branch
//! owns selected.rs, two imports compose calls, and unchanged float kernels remain
//! ordinary original-source functions. Later true/false branches own no functions.
use super::*;

const CFG_IF: &str = r###"/// `libm` cannot have dependencies, so this is vendored directly from the `cfg-if` crate
/// (with some comments stripped for compactness).
macro_rules! cfg_if {
    // match if/else chains with a final `else`
    ($(
        if #[cfg($meta:meta)] { $($tokens:tt)* }
    ) else * else {
        $($tokens2:tt)*
    }) => {
        cfg_if! { @__items () ; $( ( ($meta) ($($tokens)*) ), )* ( () ($($tokens2)*) ), }
    };

    // match if/else chains lacking a final `else`
    (
        if #[cfg($i_met:meta)] { $($i_tokens:tt)* }
        $( else if #[cfg($e_met:meta)] { $($e_tokens:tt)* } )*
    ) => {
        cfg_if! {
            @__items
            () ;
            ( ($i_met) ($($i_tokens)*) ),
            $( ( ($e_met) ($($e_tokens)*) ), )*
            ( () () ),
        }
    };

    // Internal and recursive macro to emit all the items
    //
    // Collects all the negated cfgs in a list at the beginning and after the
    // semicolon is all the remaining items
    (@__items ($($not:meta,)*) ; ) => {};
    (@__items ($($not:meta,)*) ; ( ($($m:meta),*) ($($tokens:tt)*) ), $($rest:tt)*) => {
        #[cfg(all($($m,)* not(any($($not),*))))] cfg_if! { @__identity $($tokens)* }
        cfg_if! { @__items ($($not,)* $($m,)*) ; $($rest)* }
    };

    // Internal macro to make __apply work out right for different match types,
    // because of how macros matching/expand stuff.
    (@__identity $($tokens:tt)*) => { $($tokens)* };
}
"###;

const ROOT: &str = r#"
#[macro_use]
mod macros;
cfg_if! {
    if #[cfg(any(any(), rstyle_unset_branch))] {
        mod never;
    } else if #[cfg(all())] {
        mod selected;
        use selected::selected;
    } else if #[cfg(all())] {
        mod later;
        use later::selected;
    } else {
        mod fallback;
        use fallback::selected;
    }
}
pub fn caller(x: u8) -> u8 { selected(x) }
pub fn same(x: u8) -> u8 { selected(x) }
mod k_sinf;
mod k_cosf;
"#;

fn workspace(root: &str, definition: &str) -> TempDir {
    let directory = clone_workspace();
    let source = directory.path().join("src");
    std::fs::create_dir(source.join("selected")).unwrap();
    for (path, bytes) in [
        ("lib.rs", root),
        ("macros.rs", definition),
        (
            "selected.rs",
            "mod nested; pub fn selected(x: u8) -> u8 { x }",
        ),
        ("selected/nested.rs", "pub fn nested(x: u8) -> u8 { x }"),
        ("never.rs", "pub fn never() -> u8 { 1 }"),
        ("later.rs", "pub fn selected(_x: u8) -> u8 { 2 }"),
        ("fallback.rs", "pub fn selected(_x: u8) -> u8 { 3 }"),
        ("k_sinf.rs", include_str!("float_kernels/k_sinf.rs")),
        ("k_cosf.rs", include_str!("float_kernels/k_cosf.rs")),
    ] {
        std::fs::write(source.join(path), bytes).unwrap();
    }
    directory
}

#[test]
fn cfg_if_original_definition_preserves_selected_module_imports_and_float_kernels() {
    let directory = workspace(ROOT, CFG_IF);
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    check_family(&report);
}

fn check_family(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    let expected = [
        "selected::selected",
        "selected::nested::nested",
        "caller",
        "same",
        "k_sinf::k_sinf",
        "k_cosf::k_cosf",
    ];
    assert_eq!(functions.len(), expected.len());
    for name in expected {
        let function = function(report, name);
        assert_eq!(
            function["outcome"], "admitted",
            "{name}: {}",
            function["reasons"]
        );
    }
    super::word_classifiers::verify_admissions(report);
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 15);
}

fn function<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap()
}

#[test]
fn cfg_if_changed_or_shadowed_definition_cannot_authorize_module_outputs() {
    let changed = CFG_IF.replace("=> { $($tokens)* };", "=> { $($tokens)+ };");
    assert_ne!(changed, CFG_IF);
    let shadowed = ROOT.replace(
        "cfg_if! {",
        "macro_rules! cfg_if { ($($t:tt)*) => { mod selected; use selected::selected; }; }\ncfg_if! {",
    );
    let unsupported = ROOT.replace("mod selected;", "const HIDDEN: u8 = 1; mod selected;");
    for (root, definition) in [
        (ROOT, changed.as_str()),
        (shadowed.as_str(), CFG_IF),
        (unsupported.as_str(), CFG_IF),
    ] {
        let directory = workspace(root, definition);
        let (_, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        for name in ["k_sinf::k_sinf", "k_cosf::k_cosf"] {
            let function = function(&report, name);
            assert_eq!(
                function["outcome"], "excluded",
                "{name}: {}",
                function["reasons"]
            );
        }
        for name in [
            "selected::selected",
            "selected::nested::nested",
            "caller",
            "same",
        ] {
            let function = function(&report, name);
            assert_eq!(
                function["outcome"], "admitted",
                "{name}: {}",
                function["reasons"]
            );
        }
    }
}

#[test]
fn cfg_if_resealed_module_evidence_requires_definition_callsite_and_output_linkage() {
    let directory = workspace(ROOT, CFG_IF);
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    check_family(&report);
    let original = shared_evidence::wire(function(&report, "k_sinf::k_sinf"));

    for mutation in [
        "definition_identity",
        "definition_span",
        "definition_digest",
        "invocation_span",
        "invocation_digest",
        "declaration_span",
        "loaded_file",
        "module_identity",
        "parent_file",
        "missing_expansion",
        "duplicate_expansion",
        "missing_step",
        "missing_middle_step",
        "reordered_middle_steps",
        "repeated_middle_step",
        "duplicate_file",
        "orphan_file",
        "changed_definition",
    ] {
        let mut bundle = original.clone();
        mutate_census(&mut bundle, mutation);
        reject_resealed(bundle, mutation);
    }
}

fn census(bundle: &mut Value) -> &mut Value {
    find_census(bundle).expect("shared bundle retains source census")
}

fn find_census(value: &mut Value) -> Option<&mut Value> {
    if value["identity"] == rstyle_pdg::nominal_boundary::census::IDENTITY {
        return Some(value);
    }
    match value {
        Value::Array(values) => values.iter_mut().find_map(find_census),
        Value::Object(values) => values.values_mut().find_map(find_census),
        _ => None,
    }
}

fn mutate_census(bundle: &mut Value, mutation: &str) {
    if mutation == "changed_definition" {
        change_definition(bundle);
        return;
    }
    let census = census(bundle);
    let expansions = census["expansions"]
        .as_array_mut()
        .expect("captured module expansion linkage");
    assert!(!expansions.is_empty());
    match mutation {
        "missing_expansion" => {
            expansions.remove(0);
        }
        "duplicate_expansion" => {
            expansions.push(expansions[0].clone());
        }
        "missing_middle_step" | "reordered_middle_steps" | "repeated_middle_step" => {
            mutate_chain(&mut expansions[0]["chain"], mutation);
        }
        "duplicate_file" | "orphan_file" => extra_file(census, mutation),
        _ => mutate_expansion(&mut expansions[0], mutation),
    }
}

fn mutate_expansion(expansion: &mut Value, mutation: &str) {
    let (path, value) = match mutation {
        "definition_identity" => ("/chain/0/macro_definition", "other::cfg_if".into()),
        "definition_span" => ("/chain/0/definition/span/start", 0.into()),
        "definition_digest" => ("/chain/0/definition/digest", "sha256:wrong".into()),
        "invocation_span" => ("/chain/0/invocation/span/start", 0.into()),
        "invocation_digest" => ("/chain/0/invocation/digest", "sha256:wrong".into()),
        "declaration_span" => ("/declaration/span/start", 0.into()),
        "loaded_file" => ("/loaded_file", "/not-the-selected-file.rs".into()),
        "module_identity" => ("/module", "other::selected".into()),
        "parent_file" => ("/parent", "/not-the-invoking-file.rs".into()),
        "missing_step" => ("/chain", serde_json::json!([])),
        _ => unreachable!("{mutation}"),
    };

    let field = expansion.pointer_mut(path).expect("captured linkage field");
    assert_ne!(*field, value, "{mutation} must change captured evidence");
    *field = value;
}

fn mutate_chain(chain: &mut Value, mutation: &str) {
    let chain = chain.as_array_mut().unwrap();
    assert!(
        chain.len() >= 4,
        "firsttrue selection must retain its skipped-branch stage"
    );
    assert_ne!(
        chain[1], chain[2],
        "distinct original internal invocation sites"
    );

    match mutation {
        "missing_middle_step" => {
            chain.remove(1);
        }
        "reordered_middle_steps" => chain.swap(1, 2),
        "repeated_middle_step" => chain[1] = chain[2].clone(),
        _ => unreachable!("{mutation}"),
    }
}

fn extra_file(census: &mut Value, mutation: &str) {
    let files = census["files"].as_array_mut().unwrap();
    let mut extra = files[0].clone();
    if mutation == "orphan_file" {
        extra["path"] = "/unowned-orphan.rs".into();
    }
    files.push(extra);
}

fn change_definition(bundle: &mut Value) {
    let files = census(bundle)["files"].as_array_mut().unwrap();
    let file = files
        .iter_mut()
        .find(|file| file["path"].as_str().unwrap().ends_with("/macros.rs"))
        .unwrap();
    let snapshot = file["snapshot"].as_u64().unwrap() as usize;
    let changed = CFG_IF.replace("=> { $($tokens)* };", "=> { $($tokens)+ };");
    let digest = rstyle_pdg::concrete_types::digest(changed.as_bytes());
    file["digest"] = digest.clone().into();
    bundle["sources"][snapshot] = changed.into();

    for expansion in census(bundle)["expansions"].as_array_mut().unwrap() {
        for step in expansion["chain"].as_array_mut().unwrap() {
            reseal_macro_step(step, &digest);
        }
    }
}

fn reseal_macro_step(step: &mut Value, digest: &str) {
    for kind in ["definition", "invocation"] {
        if step[kind]["path"].as_str().unwrap().ends_with("/macros.rs") {
            step[kind]["digest"] = digest.into();
        }
    }
}

fn reject_resealed(bundle: Value, mutation: &str) {
    let bytes = serde_json::to_vec(&bundle).unwrap();
    let seal = rstyle_pdg::unit_store::BundleManifest {
        schema: "rstyle.shared-unit.manifest/2".into(),
        checksum: rstyle_pdg::unit_store::transport_checksum(&bytes),
        bytes: bytes.len() as u64,
    };
    let verified = seal.verify_owned(bytes).unwrap();
    assert!(
        rstyle_pdg::unit_store::UnitStore::from_bundle(verified.bytes()).is_err(),
        "{mutation}"
    );
}

#[test]
fn cfg_if_first_true_precedence_has_independent_runtime_expectations() {
    let root = format!(
        "{ROOT}\n#[test] fn source_oracle() {{ assert_eq!(caller(41), 41); assert_eq!(same(73), 73); }}"
    );
    let directory = workspace(&root, CFG_IF);
    let binary = directory.path().join("source-oracle");
    let compiled = Command::new("rustc")
        .args(["--edition=2021", "--test"])
        .arg(directory.path().join("src/lib.rs"))
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let executed = Command::new(binary).output().unwrap();
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stdout)
    );
}

#[test]
fn cfg_if_original_bom_buffers_retain_compiler_expansion_coordinates() {
    let directory = workspace(&format!("\u{feff}{ROOT}"), &format!("\u{feff}{CFG_IF}"));
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    check_family(&report);
}
