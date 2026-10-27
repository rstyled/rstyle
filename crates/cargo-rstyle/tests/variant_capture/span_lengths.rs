//! Saturating length arithmetic composes with complete Copy helper results.
use super::float_kernels::{function, reject_resealed};
use super::*;

const SOURCE: &str = r#"
#[derive(Clone, Copy)] pub struct SmallIndex(u32);
#[derive(Clone, Copy)] pub struct PatternID(SmallIndex);
#[derive(Clone, Copy)] pub struct Span { pub start: usize, pub end: usize }
#[derive(Clone, Copy)] pub struct Match { pattern: PatternID, span: Span }
impl Span {
    pub fn len(&self) -> usize { self.end.saturating_sub(self.start) }
    pub fn is_empty(&self) -> bool { self.start >= self.end }
}
impl Match {
    pub fn span(&self) -> Span { self.span }
    pub fn len(&self) -> usize { self.span().len() }
    pub fn is_empty(&self) -> bool { self.span().is_empty() }
    pub fn start(&self) -> usize { self.span().start }
    pub fn end(&self) -> usize { self.span().end }
}
pub fn caller(a: &Match) -> usize { a.len() }
pub fn same(a: &Match) -> usize { a.len() }
pub fn subtract(a: usize, b: usize) -> usize { a.saturating_sub(b) }
pub fn saved(mut a: usize, b: usize) -> usize { a.saturating_sub({ a = 0; b }) }
pub fn narrow(a: u32, b: u32) -> u32 { a.saturating_sub(b) }
pub fn signed(a: isize, b: isize) -> isize { a.saturating_sub(b) }
pub fn addition(a: usize, b: usize) -> usize { a.saturating_add(b) }
"#;

fn capture(target: Option<&str>, no_std: bool) -> (TempDir, Value) {
    let directory = clone_workspace();
    let prefix = if no_std { "#![no_std]\n" } else { "" };
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{prefix}{SOURCE}"),
    )
    .unwrap();
    let mut flags = vec!["--extraction-profile", "hir-data-v3"];
    if let Some(target) = target {
        flags.extend(["--target", target]);
    }

    let (output, report) = clones(directory.path(), &flags);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn span_lengths_compose_saturation_saved_operands_and_owned_helper_receivers() {
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
            verify_context(target, no_std);
        }
    }
}

fn verify_context(target: &str, no_std: bool) {
    let (_directory, report) = capture(Some(target), no_std);
    for name in [
        "Span::len",
        "Span::is_empty",
        "Match::span",
        "Match::len",
        "Match::is_empty",
        "Match::start",
        "Match::end",
        "caller",
        "same",
        "subtract",
        "saved",
    ] {
        let entry = function(&report, name);
        assert_eq!(
            entry["outcome"], "admitted",
            "{target} {name}: {}",
            entry["reasons"]
        );
    }
    for name in ["narrow", "signed", "addition"] {
        assert_eq!(function(&report, name)["outcome"], "excluded", "{name}");
    }
    super::word_classifiers::verify_admissions(&report);
    assert_eq!(report["pair_evaluation_complete"], true);
    for (first, second, expected) in [
        ("caller", "same", "Accepted"),
        ("Match::len", "Match::start", "Rejected"),
        ("Match::len", "Match::end", "Rejected"),
    ] {
        let left = &function(&report, first)["key"];
        let right = &function(&report, second)["key"];
        let pair = report["matching"]["pairs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pair| {
                (&pair["first"] == left && &pair["second"] == right)
                    || (&pair["first"] == right && &pair["second"] == left)
            })
            .unwrap();
        assert_eq!(pair["decision"], expected, "{first} / {second}");
    }
}

#[test]
fn saturating_lengths_reject_resealed_arithmetic_order_authority_and_policy_changes() {
    let (_directory, report) = capture(None, false);
    let entry = function(&report, "subtract");
    for mutation in [
        "success",
        "overflow",
        "arithmetic",
        "evaluation_order",
        "evaluation_count",
        "may_panic",
    ] {
        let mut bundle = shared_evidence::wire(entry);
        let root = bundle["roots"][shared_evidence::index(entry)]
            .as_u64()
            .unwrap() as usize;
        let node = bundle["graphs"][root]["pdg"]["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|node| node["attributes"]["construct"] == "data.length.saturating-sub-usize")
            .unwrap();
        node["annotations"]["rstyle.data:checked-length"][mutation] = match mutation {
            "success" => serde_json::json!("receiver-argument"),
            "overflow" => serde_json::json!("wrap"),
            "arithmetic" => serde_json::json!("addition"),
            "evaluation_order" => serde_json::json!([1, 0]),
            "evaluation_count" => serde_json::json!("twice-each"),
            "may_panic" => serde_json::json!(true),
            _ => unreachable!(),
        };
        reject_resealed(bundle, mutation);
    }
    for mutation in ["definition_hash", "crate_hash", "path", "operands"] {
        shared_evidence::reject_edit(entry, |bundle, index| {
            corrupt_authority(&mut bundle["functions"][index]["body"], mutation);
        });
    }
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let mut pair = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    pair.pdg.profile.version = "3.71.0".into();
    pair.cfg.profile.version = "3.71.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

fn corrupt_authority(node: &mut Value, mutation: &str) {
    if node["dispatch"]["op"] == "core::num::<impl usize>::saturating_sub" {
        let mut resolution: Value =
            serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
        if mutation == "operands" {
            node["children"].as_array_mut().unwrap().swap(0, 1);
        } else {
            resolution[mutation] = "wrong-authority".into();
        }
        node["res"]["description"] = serde_json::to_string(&resolution).unwrap().into();
    }
    for child in node["children"].as_array_mut().unwrap() {
        corrupt_authority(child, mutation);
    }
}

#[test]
fn saturating_length_saved_operands_follow_independent_mathematical_difference() {
    let directory = clone_workspace();
    let oracle = r#"
#[test] fn values_and_order() {
    let mut values: Vec<usize> = (0..=32).collect();
    values.extend([255, 256, 65535, 65536, usize::MAX / 2, usize::MAX - 1, usize::MAX]);
    for &a in &values {
        for &b in &values {
            let expected = ((a as i128) - (b as i128)).max(0) as usize;
            assert_eq!(subtract(a, b), expected);
            assert_eq!(saved(a, b), expected);
            let matched = Match { pattern: PatternID(SmallIndex(7)), span: Span { start: b, end: a } };
            assert_eq!(matched.len(), expected);
        }
    }
}
"#;
    let source = directory.path().join("oracle.rs");
    let binary = directory.path().join("oracle");
    std::fs::write(&source, format!("{SOURCE}\n{oracle}")).unwrap();
    let compiled = Command::new("rustc")
        .args(["--test", "--edition=2021", "-C", "overflow-checks=on"])
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
    let output = Command::new(binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}
