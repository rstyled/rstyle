//! Fresh non-Copy Range outputs compose without authorizing Range storage or inputs.
use super::float_kernels::function;
use super::*;

const COMMON_SOURCE: &str = r#"
#[derive(Clone, Copy)] pub struct Span { pub start: usize, pub end: usize }
#[derive(Clone, Copy)] pub struct Match { span: Span }
#[derive(Clone, Copy)] pub struct Input { span: Span }
impl From<Span> for core::ops::Range<usize> {
    fn from(span: Span) -> Self { Self { start: span.start, end: span.end } }
}
impl Span {
    pub fn range(&self) -> core::ops::Range<usize> { core::ops::Range::from(*self) }
}
impl Match {
    pub fn span(&self) -> Span { self.span }
    pub fn range(&self) -> core::ops::Range<usize> { self.span().range() }
}
impl Input {
    pub fn span(&self) -> Span { self.span }
    pub fn get_range(&self) -> core::ops::Range<usize> { self.span().range() }
}
pub fn fresh(start: usize, end: usize) -> core::ops::Range<usize> {
    core::ops::Range { start, end }
}
pub fn same(start: usize, end: usize) -> core::ops::Range<usize> {
    core::ops::Range { start, end }
}
"#;

const FULL_SOURCE: &str = r#"
pub fn reordered(start: usize, end: usize) -> core::ops::Range<usize> {
    core::ops::Range { end, start }
}
pub fn forward(span: Span) -> core::ops::Range<usize> { span.range() }
pub fn returned(span: Span) -> core::ops::Range<usize> { return span.range(); }
pub fn blocked(span: Span) -> core::ops::Range<usize> { { span.range() } }
pub fn explicit(span: Span) -> core::ops::Range<usize> {
    <core::ops::Range<usize> as core::convert::From<Span>>::from(span)
}
pub fn input(range: core::ops::Range<usize>) -> core::ops::Range<usize> { range }
pub fn unused(_: core::ops::Range<usize>) -> usize { 0 }
pub fn discarded(start: usize, end: usize) -> usize { fresh(start, end); 0 }
pub fn nested_item(start: usize, end: usize) -> core::ops::Range<usize> {
    fn hidden() -> usize { 0 }
    core::ops::Range { start, end }
}
pub fn nested_shadow(start: usize, end: usize) -> core::ops::Range<usize> {
    struct Range { start: usize, end: usize }
    core::ops::Range { start, end }
}
pub fn nested_macro(start: usize, end: usize) -> core::ops::Range<usize> {
    macro_rules! make_range { () => { core::ops::Range { start, end } }; }
    make_range!()
}
pub fn stored(start: usize, end: usize) -> core::ops::Range<usize> {
    let range = core::ops::Range { start, end }; range
}
pub fn consumed(start: usize, end: usize) -> usize {
    fresh(start, end).start
}
pub fn iterator(end: usize) -> usize { (0..end).count() }
pub fn narrow(start: u32, end: u32) -> core::ops::Range<u32> {
    core::ops::Range { start, end }
}
pub fn generic<T>(start: T, end: T) -> core::ops::Range<T> {
    core::ops::Range { start, end }
}
pub struct Dropping(pub usize);
impl Drop for Dropping { fn drop(&mut self) { self.0 = 0; } }
pub fn drop_range(start: Dropping, end: Dropping) -> core::ops::Range<Dropping> {
    core::ops::Range { start, end }
}
mod unsupported {
    pub trait Other { fn range(self) -> core::ops::Range<usize>; }
    impl Other for super::Span {
        fn range(self) -> core::ops::Range<usize> {
            core::ops::Range { start: self.end, end: self.start }
        }
    }
}
pub fn other(span: Span) -> core::ops::Range<usize> {
    <Span as unsupported::Other>::range(span)
}
"#;

// Other compiler contexts retain construction, selected From, getter chains,
// unsupported Range inputs/storage, and an exact matching pair.
const CONTEXT_SOURCE: &str = r#"
pub fn explicit(span: Span) -> core::ops::Range<usize> {
    <core::ops::Range<usize> as core::convert::From<Span>>::from(span)
}
pub fn input(range: core::ops::Range<usize>) -> core::ops::Range<usize> { range }
pub fn stored(start: usize, end: usize) -> core::ops::Range<usize> {
    let range = core::ops::Range { start, end }; range
}
"#;

fn capture(target: &str, no_std: bool, comprehensive: bool) -> (TempDir, Value) {
    let directory = clone_workspace();
    let prefix = if no_std { "#![no_std]\n" } else { "" };
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!(
            "{prefix}{COMMON_SOURCE}{}",
            if comprehensive {
                FULL_SOURCE
            } else {
                CONTEXT_SOURCE
            }
        ),
    )
    .unwrap();

    let (output, report) = clones(
        directory.path(),
        &["--extraction-profile", "hir-data-v3", "--target", target],
    );
    assert!(
        output.status.success(),
        "status={:?} stderr={} capture_failures={} pair_evaluation_complete={} matching_diagnostics={}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
        report["capture_failures"],
        report["pair_evaluation_complete"],
        report["matching"]["diagnostics"]
    );
    (directory, report)
}

#[test]
fn fresh_range_getters_compose_selected_from_and_owned_helper_results() {
    let mut checked_comprehensive = false;
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
            let comprehensive = !checked_comprehensive;
            let (_directory, report) = capture(target, no_std, comprehensive);
            verify_context(&report, target, comprehensive);
            if comprehensive {
                verify_strict_pairs(&report);
                reject_capture_corruptions(&report);
                checked_comprehensive = true;
            }
        }
    }
    assert!(
        checked_comprehensive,
        "at least one installed capture target"
    );
}

fn reject_capture_corruptions(report: &Value) {
    let entry = function(report, "explicit");
    let baseline = shared_evidence::wire(entry);
    rstyle_pdg::unit_store::UnitStore::from_bundle(&serde_json::to_vec(&baseline).unwrap())
        .unwrap();

    for mutation in [
        "trait_identity",
        "trait_path",
        "implementation_identity",
        "implementation_owner",
        "missing_implementation",
    ] {
        reject_bundle_edit(&baseline, entry, |bundle, index| {
            assert!(corrupt_from(
                &mut bundle["functions"][index]["body"],
                mutation
            ));
        });
    }
    let fresh = function(report, "fresh");
    for field in ["rustc_commit", "target_triple"] {
        reject_bundle_edit(&baseline, fresh, |bundle, index| {
            bundle["functions"][index]["build"][field]["value"] = "untrusted".into();
        });
    }
    reject_bundle_edit(&baseline, fresh, |bundle, index| {
        bundle["functions"][index]["source"]["digest"] =
            format!("sha256:{}", "0".repeat(64)).into();
    });
    for mutation in [
        "is_copy",
        "needs_drop",
        "has_drop_impl",
        "definition_hash",
        "crate_hash",
        "path",
        "fields",
        "arguments",
        "lang_item",
    ] {
        reject_bundle_edit(&baseline, fresh, |bundle, index| {
            assert!(corrupt_range_type(
                &mut bundle["functions"][index]["build"]["frontend_options"],
                mutation
            ));
        });
    }
    reject_constructor_graph(fresh);
}

fn reject_bundle_edit(baseline: &Value, entry: &Value, edit: impl FnOnce(&mut Value, usize)) {
    let mut bundle = baseline.clone();
    edit(&mut bundle, shared_evidence::index(entry));
    let bytes = serde_json::to_vec(&bundle).unwrap();

    assert!(rstyle_pdg::unit_store::UnitStore::from_bundle(&bytes).is_err());
    assert!(
        rstyle_pdg::verification_policy::generated(|| {
            rstyle_pdg::unit_store::UnitStore::from_bundle(&bytes)
        })
        .is_err()
    );
}

fn corrupt_range_type(value: &mut Value, mutation: &str) -> bool {
    if value["kind"] == "RangeUsize" {
        let evidence = &mut value["evidence"]["observation"];
        match mutation {
            "is_copy" | "needs_drop" | "has_drop_impl" => {
                assert_eq!(evidence[mutation], false);
                evidence[mutation] = true.into();
            }
            "fields" => evidence["fields"].as_array_mut().unwrap().swap(0, 1),
            "arguments" => evidence["arguments"][0] = "u32".into(),
            "lang_item" => evidence["lang_item"] = "range_inclusive".into(),
            _ => evidence["definition"][mutation] = "untrusted-range".into(),
        }
        return true;
    }
    match value {
        Value::Object(fields) => fields
            .values_mut()
            .any(|field| corrupt_range_type(field, mutation)),
        Value::Array(elements) => elements
            .iter_mut()
            .any(|element| corrupt_range_type(element, mutation)),
        _ => false,
    }
}

fn reject_constructor_graph(entry: &Value) {
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let prepared = rstyle_pdg::producer::PreparedVerification::new(&facts).unwrap();
    let validated = prepared.verify(original.as_view()).unwrap();
    rstyle_ccgraph::admit_validated(&validated).unwrap();

    for mutation in [
        "slots",
        "ordered_edges",
        "duplicate_slots",
        "historical_policy",
    ] {
        let mut pair = rstyle_pdg::producer::Pair {
            pdg: original.pdg.clone(),
            cfg: original.cfg.clone(),
        };
        corrupt_constructor(&mut pair, mutation);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(prepared.verify(pair.as_view()).is_err(), "{mutation}");
        if mutation == "historical_policy" {
            assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
        }
    }
}

fn corrupt_constructor(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    if mutation == "historical_policy" {
        pair.pdg.profile.version = "3.74.0".into();
        pair.cfg.profile.version = "3.74.0".into();
        return;
    }
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some("data.struct"))
        .expect("fresh Range constructor");
    if mutation == "slots" {
        node.attributes.literal = Some(rstyle_pdg::StringFact::known("[1,0]"));
        return;
    }
    let mut count = 0;
    for edge in pair
        .pdg
        .edges
        .iter_mut()
        .filter(|edge| edge.target == node.id && edge.subkind == "value_operand")
    {
        assert!(matches!(edge.operand_position, Some(0 | 1)));
        edge.operand_position = if mutation == "ordered_edges" {
            edge.operand_position.map(|position| 1 - position)
        } else {
            Some(0)
        };
        count += 1;
    }
    assert_eq!(count, 2, "exactly two Range operands");
}

fn corrupt_from(node: &mut Value, mutation: &str) -> bool {
    if node["kind"] == "expr_call"
        && let Some(description) = node["res"]["description"].as_str()
        && let Ok(mut callee) = serde_json::from_str::<Value>(description)
        && callee.get("trait_selection").unwrap_or(&callee)["path"]
            .as_str()
            .is_some_and(|path| path.ends_with("::convert::From::from"))
    {
        let selection = if callee.get("trait_selection").is_some() {
            &mut callee["trait_selection"]
        } else {
            &mut callee
        };
        match mutation {
            "trait_identity" => selection["definition_hash"] = "wrong-trait".into(),
            "trait_path" => selection["path"] = "core::convert::Into::into".into(),
            "implementation_identity" => {
                selection["implementation"]["definition_hash"] = "wrong-implementation".into();
            }
            "implementation_owner" => selection["implementation"]["crate_name"] = "core".into(),
            "missing_implementation" => selection["implementation"] = Value::Null,
            _ => unreachable!(),
        }
        node["res"]["description"] = serde_json::to_string(&callee).unwrap().into();
        return true;
    }
    node["children"].as_array_mut().is_some_and(|children| {
        children
            .iter_mut()
            .any(|child| corrupt_from(child, mutation))
    })
}

// The first eight admissions and first two exclusions occur in the compact context fixture.
const ADMITTED: &[&str] = &[
    "Span::range",
    "Match::span",
    "Match::range",
    "Input::span",
    "Input::get_range",
    "fresh",
    "same",
    "explicit",
    "reordered",
    "forward",
    "returned",
    "blocked",
];
const EXCLUDED: &[&str] = &[
    "input",
    "stored",
    "unused",
    "discarded",
    "nested_item",
    "nested_shadow",
    "nested_macro",
    "consumed",
    "narrow",
    "generic",
    "drop_range",
    "iterator",
    "other",
];

fn verify_context(report: &Value, target: &str, comprehensive: bool) {
    let names = if comprehensive {
        ADMITTED
    } else {
        &ADMITTED[..8]
    };
    for name in names {
        let entry = function(report, name);
        assert_eq!(
            entry["outcome"], "admitted",
            "{target} {name}: {}",
            entry["reasons"]
        );
    }

    verify_exclusions(report, comprehensive);
    verify_selected_implementation(report);
    verify_complete_mirror(report);
}

fn verify_exclusions(report: &Value, comprehensive: bool) {
    let names = if comprehensive {
        EXCLUDED
    } else {
        &EXCLUDED[..2]
    };
    for name in names {
        let entry = function(report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
}

fn verify_selected_implementation(report: &Value) {
    let selected = selected_from(function(report, "explicit"));
    let implementation = function(report, selected["path"].as_str().unwrap());
    assert_eq!(
        implementation["outcome"], "admitted",
        "{}",
        implementation["reasons"]
    );
    assert_eq!(
        implementation["facts"]["build"]["frontend_options"]["definition_identity"],
        selected["definition_hash"]
    );
}

fn verify_strict_pairs(report: &Value) {
    let (unit, _) = data_unit(function(report, "fresh"));
    let analysis = unit.analysis();
    for entry in report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["outcome"] == "admitted")
    {
        let index = shared_evidence::index(entry);
        let facts = analysis.function(index).unwrap();
        let validated = rstyle_pdg::verification_policy::strict(|| {
            rstyle_pdg::producer::verify(shared_evidence::pair(&unit, entry), &facts)
        })
        .unwrap();
        rstyle_ccgraph::admit_validated(&validated).unwrap();
    }
}

fn verify_complete_mirror(report: &Value) {
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    let left = &function(report, "fresh")["key"];
    let right = &function(report, "same")["key"];
    let pair = report["matching"]["pairs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pair| {
            (&pair["first"] == left && &pair["second"] == right)
                || (&pair["first"] == right && &pair["second"] == left)
        })
        .expect("complete exact-mirror comparison");
    assert_eq!(pair["decision"], "Accepted");
}

fn selected_from(entry: &Value) -> Value {
    fn find(node: &Value) -> Option<Value> {
        if node["kind"] == "expr_call" {
            let resolution: Value =
                serde_json::from_str(node["res"]["description"].as_str()?).ok()?;
            let resolution = resolution.get("trait_selection").unwrap_or(&resolution);
            if resolution["path"]
                .as_str()?
                .ends_with("::convert::From::from")
            {
                return Some(resolution["implementation"].clone());
            }
        }
        node["children"].as_array()?.iter().find_map(find)
    }
    find(&entry["facts"]["body"]).expect("exact selected local From implementation")
}

#[test]
fn fresh_range_endpoints_follow_independent_identity_expectation() {
    let directory = clone_workspace();
    let oracle = r#"
#[test] fn endpoints() {
    let values = [0, 1, 2, 127, 128, usize::MAX / 2, usize::MAX - 1, usize::MAX];
    for start in values {
        for end in values {
            let span = Span { start, end };
            let expected = (start, end);
            let outputs = [fresh(start, end), same(start, end), reordered(start, end),
                forward(span), returned(span), blocked(span), explicit(span),
                Span::range(&span), Match { span }.range(), Input { span }.get_range()];
            for range in outputs { assert_eq!((range.start, range.end), expected); }
            if start != end {
                let swapped = core::ops::Range { start: end, end: start };
                assert_ne!((swapped.start, swapped.end), expected);
            }
            if start > end {
                let clamped = core::ops::Range { start: end, end };
                assert_ne!((clamped.start, clamped.end), expected);
            }
        }
    }
}
"#;
    let source = directory.path().join("oracle.rs");
    let binary = directory.path().join("oracle");
    std::fs::write(&source, format!("{COMMON_SOURCE}{FULL_SOURCE}\n{oracle}")).unwrap();
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
