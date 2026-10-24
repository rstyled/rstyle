//! Source-first Input-shaped diagnostic. Every original owner field and every
//! mixed Anchored variant is retained; useful acceptance is a bool-field getter.
//! Original-package acceptance is measured separately from this development case.
mod borrowed;
mod passive;
use super::float_kernels::function as exact_function;
use super::*;

const SOURCE: &str = r#"
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct SmallIndex(u32);
// Original PatternID's Debug impl is macro generated. This diagnostic uses the
// builtin derive; its field, repr and other derives follow the original owner.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PatternID(SmallIndex);
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct Span { pub start: usize, pub end: usize }
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Anchored { No, Yes, Pattern(PatternID) }
#[derive(Clone)]
pub struct Input<'h> {
    haystack: &'h [u8],
    span: Span,
    anchored: Anchored,
    earliest: bool,
}
impl<'h> Input<'h> {
    #[inline]
    pub fn get_earliest(&self) -> bool { self.earliest }
    pub fn get_haystack(&self) -> &'h [u8] { self.haystack }
    pub fn get_anchored(&self) -> Anchored { self.anchored }
    pub fn inspect(&self) -> bool { matches!(self.anchored, Anchored::Yes) }
}
pub fn caller(input: &Input<'_>) -> bool { input.get_earliest() }
pub fn same(input: &Input<'_>) -> bool { Input::get_earliest(input) }
pub fn owned(input: Input<'_>) -> bool { input.earliest }
pub fn construct(haystack: &[u8], earliest: bool) -> Input<'_> {
    Input { haystack, span: Span { start: 0, end: 0 }, anchored: Anchored::No, earliest }
}
pub fn make_anchored() -> Anchored { Anchored::No }
"#;

fn function<'a>(report: &'a Value, name: &str) -> &'a Value {
    let Some(method) = name.strip_prefix("Input::") else {
        return exact_function(report, name);
    };
    let suffix = format!("::{method}");
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["facts"]["name"]
                .as_str()
                .is_some_and(|name| name.starts_with("Input::") && name.ends_with(&suffix))
        })
        .unwrap()
}

fn capture(source: &str, no_std: bool) -> (TempDir, Value) {
    capture_context(source, no_std, None)
}

fn capture_context(source: &str, no_std: bool, target: Option<&str>) -> (TempDir, Value) {
    let directory = clone_workspace();
    let prefix = if no_std { "#![no_std]\n" } else { "" };
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{prefix}{source}"),
    )
    .unwrap();
    let mut arguments = vec!["--extraction-profile", "hir-data-v3"];
    if let Some(target) = target {
        arguments.extend(["--target", target]);
    }
    let (output, report) = clones(directory.path(), &arguments);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    (directory, report)
}

#[test]
fn complete_input_owner_retains_unused_slice_span_mixed_enum_and_wrapper_evidence() {
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
        check_owner_context(target);
    }
}

fn check_owner_context(target: &str) {
    for no_std in [false, true] {
        let (_directory, report) = capture_context(SOURCE, no_std, Some(target));
        for name in [
            "Input::get_earliest",
            "Input::get_haystack",
            "caller",
            "same",
        ] {
            let entry = function(&report, name);
            assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
        }
        for name in [
            "Input::get_anchored",
            "Input::inspect",
            "owned",
            "construct",
            "make_anchored",
        ] {
            let entry = function(&report, name);
            assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
        }
        super::word_classifiers::verify_admissions(&report);
        check_matching(&report);
        reject_historical(function(&report, "Input::get_earliest"));
    }
}

fn reject_historical(entry: &Value) {
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let original = unit.pair(index).unwrap();
    let mut pair = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    pair.pdg.profile.version = "3.66.0".into();
    pair.cfg.profile.version = "3.66.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

fn check_matching(report: &Value) {
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    let left = &function(report, "caller")["key"];
    let right = &function(report, "same")["key"];
    let pairs = report["matching"]["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 6);
    assert!(
        pairs
            .iter()
            .any(|pair| ((&pair["first"] == left && &pair["second"] == right)
                || (&pair["first"] == right && &pair["second"] == left))
                && pair["decision"] == "Accepted")
    );
}

#[test]
fn unused_owner_children_derive_identity_and_source_effects_cannot_be_omitted() {
    let sources = [
        SOURCE
            .replace(", Copy", "")
            .replace("{ self.anchored }", "{ self.anchored.clone() }"),
        SOURCE.replace("Pattern(PatternID)", "Pattern(PatternID, u8)"),
        SOURCE.replace("Pattern(PatternID)", "Pattern(&'static u8)"),
        SOURCE
            .replace(
                "haystack: &'h [u8],",
                "haystack: &'h [u8],\n    unused: NonCopy,",
            )
            .replace(
                "#[derive(Clone)]\npub struct Input",
                "#[derive(Clone)] pub struct NonCopy(u8);\n#[derive(Clone)]\npub struct Input",
            )
            .replace(
                "Input { haystack, span:",
                "Input { haystack, unused: NonCopy(0), span:",
            ),
        format!(
            "{SOURCE}\nimpl Drop for Input<'_> {{ fn drop(&mut self) {{ core::hint::black_box(self.earliest); }} }}"
        ),
        format!("{SOURCE}\nmacro_rules! inject {{ () => {{ pub struct Hidden; }}; }} inject!();"),
    ];
    for source in sources {
        let (_directory, report) = capture(&source, false);
        let entry = function(&report, "Input::get_earliest");
        assert_eq!(entry["outcome"], "excluded", "{}", entry["reasons"]);
    }
}

#[test]
fn lifetime_specialization_method_generics_and_unproved_repr_remain_closed() {
    for source in [
        SOURCE
            .replace("impl<'h> Input<'h>", "impl Input<'static>")
            .replace("-> &'h [u8]", "-> &'static [u8]")
            .replace("&Input<'_>", "&Input<'static>"),
        SOURCE.replace(
            "pub fn get_earliest(&self)",
            "pub fn get_earliest<'method>(&self)",
        ),
        SOURCE.replace("#[repr(transparent)]", "#[repr(C)]"),
        SOURCE.replace("#[repr(transparent)]", "#[repr(C, align(8))]"),
    ] {
        let (_directory, report) = capture(&source, false);
        let entry = function(&report, "Input::get_earliest");
        assert_eq!(entry["outcome"], "excluded", "{}", entry["reasons"]);
    }
}

#[test]
fn input_bool_getter_runtime_depends_only_on_earliest() {
    let directory = clone_workspace();
    let oracle = r#"
#[test] fn independent_fields() {
    let bytes = [0, 255, 0xc3, 0xa9];
    for haystack in [&bytes[..0], &bytes[..], &bytes[1..3]] {
        for span in [Span { start: 0, end: 0 }, Span { start: usize::MAX, end: 1 }] {
            for anchored in [Anchored::No, Anchored::Yes, Anchored::Pattern(PatternID(SmallIndex(u32::MAX))) ] {
                for earliest in [false, true] {
                    let input = Input { haystack, span, anchored, earliest };
                    assert_eq!(input.get_earliest(), earliest);
                    assert_eq!(caller(&input), earliest);
                    assert_eq!(same(&input), earliest);
                }
            }
        }
    }
}
"#;
    let source = directory.path().join("oracle.rs");
    let binary = directory.path().join("oracle");
    std::fs::write(&source, format!("{SOURCE}\n{oracle}")).unwrap();
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
    let output = Command::new(binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn mixed_owner_evidence_cannot_omit_unused_variants_payloads_or_derive_authority() {
    let (_directory, report) = capture(SOURCE, false);
    let entry = function(&report, "Input::get_earliest");
    for mutation in [
        "variant_missing",
        "variant_order",
        "variant_name",
        "unit_payload",
        "shape_missing",
        "payload_type",
        "owner_field",
        "copy_missing",
        "derive_authority",
        "origin_missing",
    ] {
        let mut bundle = shared_evidence::wire(entry);
        let index = shared_evidence::index(entry);
        let options = &mut bundle["functions"][index]["build"]["frontend_options"];
        corrupt_owner(options, mutation);
        super::float_kernels::reject_resealed(bundle, mutation);
    }
}

fn corrupt_owner(options: &mut Value, mutation: &str) {
    if mutation.starts_with("variant_")
        || ["unit_payload", "shape_missing", "payload_type"].contains(&mutation)
    {
        let mixed = find_tree(
            &mut options["concrete_type_observations"],
            "MixedEnum",
            "Anchored",
        )
        .unwrap();
        corrupt_variants(&mut mixed["evidence"]["variants"], mutation);
    } else if mutation == "owner_field" {
        let owner = find_tree(
            &mut options["concrete_type_observations"],
            "Struct",
            "Input",
        )
        .unwrap();
        let fields = owner["evidence"]["fields"].as_array_mut().unwrap();
        let removed = fields
            .iter()
            .position(|field| field["name"] == "haystack")
            .unwrap();
        fields.remove(removed);
    } else {
        corrupt_origins(
            &mut options["concrete_declaration_origins"]["value"]["declarations"],
            mutation,
        );
    }
}

fn find_tree<'a>(value: &'a mut Value, kind: &str, name: &str) -> Option<&'a mut Value> {
    if value["kind"] == kind
        && value["evidence"]["nominal"]["definition_path"]
            .as_str()
            .is_some_and(|path| path.rsplit("::").next() == Some(name))
    {
        return Some(value);
    }
    match value {
        Value::Object(object) => object
            .values_mut()
            .find_map(|child| find_tree(child, kind, name)),
        Value::Array(array) => array
            .iter_mut()
            .find_map(|child| find_tree(child, kind, name)),
        _ => None,
    }
}

fn corrupt_variants(variants: &mut Value, mutation: &str) {
    let variants = variants.as_array_mut().unwrap();
    assert_eq!(variants.len(), 3);
    match mutation {
        "variant_missing" => {
            variants.remove(1);
        }
        "variant_order" => variants.swap(0, 1),
        "variant_name" => variants[1]["name"] = "Other".into(),
        "unit_payload" => variants[0]["shape"] = variants[2]["shape"].clone(),
        "shape_missing" => {
            variants[0].as_object_mut().unwrap().remove("shape");
        }
        "payload_type" => {
            variants[2]["shape"]["field"]["ty"] =
                serde_json::json!({"kind":"Scalar","evidence":"u64"})
        }
        _ => unreachable!(),
    }
}

fn corrupt_origins(origins: &mut Value, mutation: &str) {
    let origins = origins.as_array_mut().unwrap();
    if mutation == "origin_missing" {
        let index = origins
            .iter()
            .position(|origin| nominal_name(origin) == Some("SmallIndex"))
            .unwrap();
        origins.remove(index);
        return;
    }
    let anchored = origins
        .iter_mut()
        .find(|origin| nominal_name(origin) == Some("Anchored"))
        .unwrap();
    match mutation {
        "copy_missing" => {
            anchored.as_object_mut().unwrap().remove("builtin_copy");
        }
        "derive_authority" => {
            anchored["builtin_copy"]["derive_definition"]["definition_hash"] = "unrelated".into()
        }
        _ => unreachable!(),
    }
}

fn nominal_name(value: &Value) -> Option<&str> {
    value["nominal"]["definition_path"]
        .as_str()?
        .rsplit("::")
        .next()
}
