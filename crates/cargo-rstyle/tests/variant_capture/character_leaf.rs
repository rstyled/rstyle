use super::*;

const SOURCE: &str = r#"
    pub fn meta(c: char) -> bool {
        match c {
            '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{'
            | '}' | '^' | '$' | '#' | '&' | '-' | '~' => true,
            _ => false,
        }
    }
    pub fn same(c: char) -> bool { meta(c) }
    pub fn escaped(c: char) -> bool { c == '\u{1F600}' }
    pub fn ordered(a: char, b: char) -> bool { a < b }
    pub fn saved(mut a: char, b: char) -> char { let old = a; a = b; old }
    pub fn unused(_c: char) -> bool { true }
    pub fn array(c: char) -> [char; 1] { [c] }
    pub fn reference(c: &char) -> bool { *c == 'a' }
    pub fn cast(c: char) -> u32 { c as u32 }
    pub fn range(c: char) -> bool { match c { 'a'..='z' => true, _ => false } }
    pub fn api(c: char) -> bool { c.is_alphabetic() }
    pub fn option(c: char) -> Option<char> { Some(c) }
    pub fn aggregate(c: char) -> (char, char) { (c, c) }
"#;

#[test]
fn character_leaf_preserves_scalar_domain_and_closed_neighboring_features() {
    let directory = clone_workspace();
    runtime_oracle(directory.path());
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 13);
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        let expected = if [
            "meta", "same", "escaped", "ordered", "saved", "unused", "range",
        ]
        .contains(&name)
        {
            "admitted"
        } else {
            "excluded"
        };
        assert_eq!(
            function["outcome"], expected,
            "{name}: {}",
            function["reasons"]
        );
        if name == "unused" {
            reject_unused_historical_policy(function);
        }
        if name == "meta" || name == "ordered" {
            reject_structure_mutations(function, name);
        }
        if name == "escaped" {
            reject_literal_mutations(function);
        }
    }
    super::word_classifiers::verify_admissions(&report);
    assert_eq!(report["pair_evaluation_complete"], true);
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 21);
}

fn reject_literal_mutations(function: &Value) {
    for mutation in ["value", "type", "span"] {
        shared_evidence::reject_edit(function, |bundle, index| {
            let body = &mut bundle["functions"][index]["body"];
            mutate_literal(body, mutation);
        });
    }
}

fn mutate_literal(node: &mut Value, mutation: &str) {
    if node["kind"] == "expr_lit" {
        match mutation {
            "value" => node["lit"] = "'x'".into(),
            "type" => node["ty"]["normalized"] = "u32".into(),
            "span" => node["span"]["end"] = node["span"]["start"].clone(),
            _ => unreachable!(),
        }
    }
    for child in node["children"].as_array_mut().unwrap() {
        mutate_literal(child, mutation);
    }
}

fn reject_unused_historical_policy(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let mut changed = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    changed.pdg.profile.version = "3.60.0".into();
    changed.cfg.profile.name = changed.pdg.profile.id.clone();
    changed.cfg.profile.version = changed.pdg.profile.version.clone();
    let error = rstyle_pdg::data_documents::verify(&changed, &facts).unwrap_err();
    assert!(error.contains("Character scalars require"), "{error}");
    assert!(rstyle_ccgraph::admit_produced(&changed, &facts).is_err());
}

fn reject_structure_mutations(function: &Value, name: &str) {
    let mutations: &[&str] = if name == "meta" {
        &["or_child", "wildcard"]
    } else {
        &["operator", "dispatch_type"]
    };
    for mutation in mutations {
        shared_evidence::reject_edit(function, |bundle, index| {
            assert!(mutate_structure(
                &mut bundle["functions"][index]["body"],
                mutation
            ));
        });
    }
}

fn mutate_structure(node: &mut Value, mutation: &str) -> bool {
    match (node["kind"].as_str(), mutation) {
        (Some("pat_or"), "or_child") => {
            assert!(node["children"].as_array_mut().unwrap().pop().is_some());
            return true;
        }
        (Some("pat_wild"), "wildcard") => {
            node["kind"] = "pat_literal".into();
            node["lit"] = "'x'".into();
            return true;
        }
        (Some("expr_binary"), "operator") => {
            node["dispatch"]["op"] = "+".into();
            return true;
        }
        (Some("expr_binary"), "dispatch_type") => {
            node["dispatch"]["operand_tys"][0] = "u32".into();
            return true;
        }
        _ => {}
    }
    node["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .any(|child| mutate_structure(child, mutation))
}

fn runtime_oracle(directory: &Path) {
    let oracle = r#"
        #[test] fn independent_saved_values_and_unicode_order() {
            assert_eq!(saved('a', 'b'), 'a');
            assert_eq!(saved('\u{10FFFF}', '\0'), '\u{10FFFF}');
            assert!(ordered('\u{D7FF}', '\u{E000}'));
            assert!(!ordered('\u{E000}', '\u{D7FF}'));
            assert!(!ordered('😀', '😀'));
            assert!(escaped('😀'));
            assert!(!escaped('a'));
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
