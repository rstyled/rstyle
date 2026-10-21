//! A separate macro-bearing type-dependency scope; the getter scope stays closed.
use super::*;

pub(super) fn source() -> String {
    let (wrappers, owner) = SOURCE.split_once("#[derive(Clone, Copy, Eq").unwrap();
    let owner = owner.replace(
        "    pub fn get_anchored",
        r#"
    pub fn get_span(&self) -> Span { self.span }
    pub fn start(&self) -> usize { self.get_span().start }
    pub fn end(&self) -> usize { self.get_span().end }
    pub fn is_done(&self) -> bool { self.get_span().start > self.get_span().end }
    pub fn get_anchored"#,
    );
    let wrappers = wrappers.replace(
        "#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, PartialOrd, Ord)]\n#[repr(transparent)]\npub struct PatternID",
        "#[derive(Clone, Copy, Default, Eq, Hash, PartialEq, PartialOrd, Ord)]\n#[repr(transparent)]\npub struct PatternID",
    );
    format!(
        r#"
mod ids {{
    {wrappers}
    macro_rules! implementations {{ ($name:ident) => {{
        impl core::fmt::Debug for $name {{
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{
                f.debug_tuple("PatternID").field(&self.0.0).finish()
            }}
        }}
        impl $name {{ pub fn effect(&self) -> u32 {{ core::hint::black_box(self.0.0) }} }}
    }}; }}
    implementations!(PatternID);
}}
use ids::PatternID;
#[derive(Clone, Copy, Eq{owner}
pub fn exposed(value: &ids::PatternID) -> &ids::PatternID {{ value }}
pub fn invoked(value: &ids::PatternID) -> u32 {{ value.effect() }}
"#
    )
}

#[test]
fn passive_copy_children_do_not_import_neighboring_macro_effects() {
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
        if Path::new(String::from_utf8_lossy(&library.stdout).trim()).is_dir() {
            check_passive_context(target);
        }
    }
}

fn check_passive_context(target: &str) {
    for no_std in [false, true] {
        let (_directory, report) = capture_context(&source(), no_std, Some(target));
        for name in [
            "Input::get_earliest",
            "Input::get_haystack",
            "Input::get_span",
            "Input::start",
            "Input::end",
            "Input::is_done",
            "caller",
            "same",
        ] {
            let entry = function(&report, name);
            assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
            reject_previous(entry);
        }
        for name in ["exposed", "invoked", "owned", "construct"] {
            let entry = function(&report, name);
            assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
        }
        super::super::word_classifiers::verify_admissions(&report);
        check_getter_matching(&report);
    }
}

fn check_getter_matching(report: &Value) {
    for flag in [
        "capture_complete",
        "assessment_complete",
        "pair_enumeration_complete",
        "pair_evaluation_complete",
    ] {
        assert_eq!(report[flag], true, "{flag}");
    }
    let pairs = report["matching"]["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), 28);
    let left = &function(report, "caller")["key"];
    let right = &function(report, "same")["key"];
    assert!(
        pairs
            .iter()
            .any(|pair| ((&pair["first"] == left && &pair["second"] == right)
                || (&pair["first"] == right && &pair["second"] == left))
                && pair["decision"] == "Accepted")
    );
}

fn reject_previous(entry: &Value) {
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let captured = unit.pair(index).unwrap();
    let mut pair = rstyle_pdg::producer::Pair {
        pdg: captured.pdg.clone(),
        cfg: captured.cfg.clone(),
    };
    pair.pdg.profile.version = "3.67.0".into();
    pair.cfg.profile.version = "3.67.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

#[test]
fn passive_children_keep_owning_scopes_and_complete_copy_proofs_closed() {
    for source in [
        format!(
            "{}\nmacro_rules! hidden {{ () => {{ pub struct Hidden; }}; }} hidden!();",
            source()
        ),
        format!(
            "{}\nmacro_rules! hidden {{ () => {{ pub struct Hidden; }}; }} hidden!();",
            source().replace(
                "#[derive(Clone)]\npub struct Input",
                "#[derive(Clone, Copy)]\npub struct Input"
            )
        ),
        format!(
            "{}\nimpl Drop for Input<'_> {{ fn drop(&mut self) {{ core::hint::black_box(self.earliest); }} }}",
            source()
        ),
        source().replace("#[repr(transparent)]", "#[repr(C)]"),
        source()
            .replace(", Copy", "")
            .replace("{ self.anchored }", "{ self.anchored.clone() }")
            .replace("{ self.span }", "{ self.span.clone() }"),
    ] {
        let (_directory, report) = capture(&source, false);
        let entry = function(&report, "Input::get_earliest");
        assert_eq!(entry["outcome"], "excluded", "{}", entry["reasons"]);
    }
}

#[test]
fn passive_dependency_origin_and_copy_authority_cannot_be_resealed_away() {
    let (_directory, report) = capture(&source(), false);
    let entry = function(&report, "Input::get_earliest");
    assert_eq!(entry["outcome"], "admitted", "{}", entry["reasons"]);
    for mutation in [
        "copy_missing",
        "copy_hash",
        "copy_span",
        "origin_missing",
        "owner_field",
    ] {
        let mut bundle = shared_evidence::wire(entry);
        let index = shared_evidence::index(entry);
        let options = &mut bundle["functions"][index]["build"]["frontend_options"];
        if matches!(mutation, "origin_missing" | "owner_field") {
            corrupt_owner(options, mutation);
        } else {
            corrupt_dependency(options, mutation);
        }
        super::super::float_kernels::reject_resealed(bundle, mutation);
    }
}

fn corrupt_dependency(options: &mut Value, mutation: &str) {
    let origin = options["concrete_declaration_origins"]["value"]["declarations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|origin| nominal_name(origin) == Some("PatternID"))
        .unwrap();
    match mutation {
        "copy_missing" => {
            origin.as_object_mut().unwrap().remove("builtin_copy");
        }
        "copy_hash" => {
            origin["builtin_copy"]["trait_definition"]["definition_hash"] = "unrelated".into()
        }
        "copy_span" => {
            origin["builtin_copy"]["derive_span"]["start"] = (origin["builtin_copy"]["derive_span"]
                ["start"]
                .as_u64()
                .unwrap()
                + 1)
            .into()
        }
        _ => unreachable!(),
    }
}
