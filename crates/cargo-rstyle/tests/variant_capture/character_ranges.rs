use super::*;

const SOURCE: &str = r#"
    pub fn alnum(c: char) -> bool {
        match c { '0'..='9' | 'A'..='Z' | 'a'..='z' => true, _ => false }
    }
    pub fn gap(c: char) -> bool {
        match c { '\u{D7FF}'..='\u{E000}' => true, _ => false }
    }
    pub fn exclusive(c: char) -> bool {
        match c { 'a'..'z' => true, _ => false }
    }
    pub fn suffix(c: char) -> bool {
        match c { '\u{10FFFF}'.. => true, _ => false }
    }
"#;

#[test]
fn character_ranges_retain_endpoint_spelling_order_and_unicode_domain() {
    let directory = clone_workspace();
    runtime_oracle(directory.path());
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 4);
    for function in functions {
        assert_eq!(function["outcome"], "admitted", "{}", function["reasons"]);
        if function["facts"]["name"] == "gap" {
            reject_range_mutations(function);
            reject_documents(function);
        }
    }
    super::word_classifiers::verify_admissions(&report);
    assert_eq!(report["pair_evaluation_complete"], true);
    assert_eq!(report["matching"]["pairs"].as_array().unwrap().len(), 6);
}

fn reject_range_mutations(function: &Value) {
    for mutation in ["endpoint", "inclusive", "type"] {
        shared_evidence::reject_edit(function, |bundle, index| {
            assert!(mutate_range(
                &mut bundle["functions"][index]["body"],
                mutation
            ));
        });
    }
}

fn mutate_range(node: &mut Value, mutation: &str) -> bool {
    if node["kind"] == "pat_range" {
        if mutation == "type" {
            node["ty"]["normalized"] = "u32".into();
        } else {
            let mut range: Value = serde_json::from_str(node["lit"].as_str().unwrap()).unwrap();
            if mutation == "endpoint" {
                range["start"] = "'x'".into();
            } else {
                range["inclusive"] = false.into();
            }
            node["lit"] = serde_json::to_string(&range).unwrap().into();
        }
        return true;
    }
    node["children"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .any(|child| mutate_range(child, mutation))
}

fn reject_documents(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let mut changed = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    let selector = changed
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("match.range"))
        .unwrap();
    selector.attributes.literal = Some(rstyle_pdg::StringFact::known(
        r#"{"start":"'a'","end":"'z'","inclusive":true}"#,
    ));
    assert!(rstyle_pdg::data_documents::verify(&changed, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&changed, &facts).is_err());
    let mut historical = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    historical.pdg.profile.version = "3.61.0".into();
    historical.cfg.profile.version = "3.61.0".into();
    assert!(rstyle_pdg::data_documents::verify(&historical, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&historical, &facts).is_err());
}

fn runtime_oracle(directory: &Path) {
    let oracle = r#"
        #[test] fn independent_numeric_intervals() {
            for n in 0..=0x10FFFF {
                let Some(c) = char::from_u32(n) else { continue; };
                let expected = (48..=57).contains(&n)
                    || (65..=90).contains(&n) || (97..=122).contains(&n);
                assert_eq!(alnum(c), expected, "alnum U+{n:04X}");
                assert_eq!(gap(c), n == 0xD7FF || n == 0xE000, "gap U+{n:04X}");
                assert_eq!(exclusive(c), (97..122).contains(&n), "exclusive U+{n:04X}");
                assert_eq!(suffix(c), n == 0x10FFFF, "suffix U+{n:04X}");
            }
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
