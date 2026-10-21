use super::*;

const SOURCE: &str = r#"
pub fn is_meta_character(c: char) -> bool {
    match c {
        '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{'
        | '}' | '^' | '$' | '#' | '&' | '-' | '~' => true,
        _ => false,
    }
}
pub fn is_escapeable_character(c: char) -> bool {
    // Certainly escapable if it's a meta character.
    if is_meta_character(c) {
        return true;
    }
    // Any character that isn't ASCII is definitely not escapable. There's
    // no real need to allow things like \☃ right?
    if !c.is_ascii() {
        return false;
    }
    // Otherwise, we basically say that everything is escapable unless it's a
    // letter or digit. Things like \3 are either octal (when enabled) or an
    // error, and we should keep it that way. Otherwise, letters are reserved
    // for adding new syntax in a backwards compatible way.
    match c {
        '0'..='9' | 'A'..='Z' | 'a'..='z' => false,
        // While not currently supported, we keep these as not escapable to
        // give us some flexibility with respect to supporting the \< and
        // \> word boundary assertions in the future. By rejecting them as
        // escapable, \< and \> will result in a parse error. Thus, we can
        // turn them into something else in the future without it being a
        // backwards incompatible change.
        //
        // OK, now we support \< and \>, and we need to retain them as *not*
        // escapable here since the escape sequence is significant.
        '<' | '>' => false,
        _ => true,
    }
}
pub fn wrapper(c: char) -> bool { is_escapeable_character(c) }
pub fn ascii(c: char) -> bool { c.is_ascii() }
pub fn saved(mut a: char, b: char) -> bool {
    let old = a; a = b; old.is_ascii() == a.is_ascii()
}
pub fn reference(c: &char) -> bool { c.is_ascii() }
pub fn cast(c: char) -> u32 { c as u32 }
pub fn unrelated(c: char) -> bool { c.is_alphabetic() }
pub struct Pretend { c: char }
impl Pretend { pub fn is_ascii(&self) -> bool { true } }
pub fn custom(value: &Pretend) -> bool { value.is_ascii() }
"#;

#[test]
fn character_escape_family_composes_across_registered_build_contexts() {
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
        for no_std in [false, true] {
            let directory = clone_workspace();
            let source = format!("{}{}", if no_std { "#![no_std]\n" } else { "" }, SOURCE);
            std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
            let (output, report) = clones(
                directory.path(),
                &["--extraction-profile", "hir-data-v3", "--target", target],
            );
            assert!(output.status.success(), "{}", report["capture_failures"]);
            check_family(&report);
        }
    }
}

fn check_family(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 10);
    for function in functions {
        let positive = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "is_meta_character" | "is_escapeable_character" | "wrapper" | "ascii" | "saved"
        );
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{}: {}",
            function["facts"]["name"],
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
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 10);
}

#[test]
fn character_ascii_rejects_receiver_authority_and_saved_operand_corruption() {
    let directory = clone_workspace();
    runtime_oracle(directory.path());
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    check_family(&report);
    let ascii = function(&report, "ascii");
    for mutation in [
        "signature",
        "kind",
        "target",
        "count",
        "callee",
        "dispatch",
        "span",
    ] {
        let mut bundle = shared_evidence::wire(ascii);
        mutate_frontend(
            &mut bundle["functions"][shared_evidence::index(ascii)],
            mutation,
        );
        reject_resealed(bundle, mutation);
    }
    let saved = function(&report, "saved");
    let mut bundle = shared_evidence::wire(saved);
    let root = bundle["roots"][shared_evidence::index(saved)]
        .as_u64()
        .unwrap() as usize;
    swap_call_operands(&mut bundle["graphs"][root]["pdg"]);
    reject_resealed(bundle, "saved_operand");
    reject_old_policy(ascii);
    reject_non_scalar_receiver(&report);
    reject_forged_scalar_receiver(ascii);
}

fn method(node: &rstyle_pdg::HirNode) -> &rstyle_pdg::HirNode {
    if node.kind == "expr_method_call" {
        return node;
    }
    node.children
        .iter()
        .find_map(|child| {
            if child.kind == "expr_method_call" {
                Some(child)
            } else {
                None
            }
        })
        .unwrap()
}

fn reject_non_scalar_receiver(report: &Value) {
    let (unit, index) = data_unit(function(report, "reference"));
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    rstyle_pdg::data_receivers::verify_capture(&facts).unwrap();
    assert!(
        rstyle_pdg::call_summaries::resolve_for(method(&facts.body), &facts)
            .map_or(true, |summary| summary.is_none())
    );
}

fn reject_forged_scalar_receiver(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut changed = serde_json::to_value(&*facts).unwrap();
    replace_character_tokens(&mut changed);
    let changed: rstyle_pdg::HirFnFacts = serde_json::from_value(changed).unwrap();
    // The ordinary receiver inventory is internally consistent for u8. Its
    // retained char method identity cannot authorize a character summary.
    rstyle_pdg::data_receivers::verify_capture(&changed).unwrap();
    assert!(
        rstyle_pdg::call_summaries::resolve_for(method(&changed.body), &changed)
            .map_or(true, |summary| summary.is_none())
    );
}

fn replace_character_tokens(value: &mut Value) {
    match value {
        Value::String(text) if text == "char" => *text = "u8".into(),
        Value::Array(values) => values.iter_mut().for_each(replace_character_tokens),
        Value::Object(values) => values.values_mut().for_each(replace_character_tokens),
        _ => {}
    }
}

fn function<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == name)
        .unwrap()
}

fn mutate_frontend(facts: &mut Value, mutation: &str) {
    if matches!(mutation, "callee" | "dispatch") {
        assert!(mutate_call(&mut facts["body"], mutation));
        return;
    }
    let method = &mut facts["build"]["frontend_options"]["data_method_receivers"]["methods"][0];
    match mutation {
        "signature" => method["signature_inputs"][0] = method["receiver_type"].clone(),
        "kind" => method["adjustments"][0]["kind"] = "mutable_borrow".into(),
        "target" => method["adjustments"][0]["target"] = method["receiver_type"].clone(),
        "count" => method["adjustments"] = serde_json::json!([]),
        "span" => method["source_span"]["end"] = method["source_span"]["start"].clone(),
        _ => unreachable!(),
    }
}

fn mutate_call(node: &mut Value, mutation: &str) -> bool {
    if node["kind"] == "expr_method_call" {
        if mutation == "callee" {
            let mut callee: Value =
                serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
            callee["definition_hash"] = "invented-definition".into();
            node["res"]["description"] = serde_json::to_string(&callee).unwrap().into();
        } else {
            node["dispatch"]["operand_tys"][0] = "u32".into();
        }
        return true;
    }
    node["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .any(|child| mutate_call(child, mutation))
}

fn swap_call_operands(pdg: &mut Value) {
    let calls: Vec<_> = pdg["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["category"] == "call")
        .map(|node| node["id"].clone())
        .collect();
    assert_eq!(calls.len(), 2);
    let edges = pdg["edges"].as_array_mut().unwrap();
    let selected: Vec<_> = edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| edge["subkind"] == "value_operand" && calls.contains(&edge["target"]))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(selected.len(), 2);
    let before = edges[selected[0]]["source"].clone();
    assert_ne!(before, edges[selected[1]]["source"]);
    edges[selected[0]]["source"] = edges[selected[1]]["source"].clone();
    edges[selected[1]]["source"] = before;
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

fn reject_old_policy(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let mut pair = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    pair.pdg.profile.version = "3.61.0".into();
    pair.cfg.profile.version = "3.61.0".into();
    let error = rstyle_pdg::data_documents::verify(&pair, &facts).unwrap_err();
    assert!(error.contains("registered document policy"), "{error}");
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

fn runtime_oracle(directory: &Path) {
    let oracle = r#"
        #[test] fn independently_enumerated_ascii_and_unicode_boundaries() {
            for code in 0..=0x10ffff {
                let Some(c) = char::from_u32(code) else { continue; };
                let meta = code < 128 && b"\\.+*?()|[]{}^$#&-~".contains(&(code as u8));
                assert_eq!(is_meta_character(c), meta, "{code:x}");
                let expected = code < 128 && !matches!(code, 48..=57 | 65..=90 | 97..=122 | 60 | 62);
                assert_eq!(is_escapeable_character(c), expected, "{code:x}");
                assert_eq!(wrapper(c), expected, "{code:x}");
                assert_eq!(ascii(c), code < 128);
            }
            assert!(!saved('a', '😀'));
            assert!(!saved('😀', 'a'));
            assert!(saved('\u{7f}', '\0'));
            assert!(saved('\u{80}', '\u{10ffff}'));
        }
    "#;
    let source = directory.join("oracle.rs");
    let binary = directory.join("oracle");
    std::fs::write(&source, format!("{SOURCE}{oracle}")).unwrap();
    let compiled = Command::new("rustc")
        .args(["--test", "--edition=2021"])
        .arg(source)
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
