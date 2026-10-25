//! Exact source/cleanup receipts remain distinct from caller graph admission.
use super::*;
use rstyle_pdg::{
    concrete_types::{InterfaceObservations, Observation, TypeTree, source_proof::PathResolution},
    unit_store::EvidenceFacts,
};
use serde_json::json;

const SOURCE: &str = r#"
use core::num::NonZero;
#[derive(Debug)]
pub struct ParsedItem<'a, T>(&'a [u8], T);
impl<'a, T> ParsedItem<'a, T> {
    pub(crate) fn flat_map<U>(self, f: impl FnOnce(T) -> Option<U>) -> Option<ParsedItem<'a, U>> {
        Some(ParsedItem(self.0, f(self.1)?))
    }
}
pub struct ExactlyNDigits<const N: u8>;
impl ExactlyNDigits<2> {
    pub const fn parse(input: &[u8]) -> Option<ParsedItem<'_, u8>> {
        match input {
            [a @ b'0'..=b'9', b @ b'0'..=b'9', remaining @ ..] => {
                let a = *a - b'0'; let b = *b - b'0';
                Some(ParsedItem(remaining, a * 10 + b))
            }
            _ => None,
        }
    }
}
impl ExactlyNDigits<3> {
    pub const fn parse(input: &[u8]) -> Option<ParsedItem<'_, u16>> {
        match input {
            [a @ b'0'..=b'9', b @ b'0'..=b'9', c @ b'0'..=b'9', remaining @ ..] => {
                let a = (*a - b'0') as u16; let b = (*b - b'0') as u16; let c = (*c - b'0') as u16;
                Some(ParsedItem(remaining, a * 100 + b * 10 + c))
            }
            _ => None,
        }
    }
}
pub fn week(input: &[u8]) -> Option<ParsedItem<'_, NonZero<u8>>> {
    ExactlyNDigits::<2>::parse(input).and_then(|parsed| parsed.flat_map(NonZero::new))
}
pub fn dayo(input: &[u8]) -> Option<ParsedItem<'_, NonZero<u16>>> {
    ExactlyNDigits::<3>::parse(input).and_then(|parsed| parsed.flat_map(NonZero::new))
}
"#;

fn capture(source: &str) -> Value {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}; capture failures {}; handoff failures {}; diagnostics {}",
        String::from_utf8_lossy(&output.stderr),
        report["capture_failures"],
        report["handoff_failures"],
        report["matching"]["diagnostics"]
    );
    report
}

#[test]
fn nonzero_caller_composition_preserves_complete_guarded_results() {
    let report = capture(SOURCE);
    let store = store(&report);
    let analysis = store.analysis();
    for (name, scalar) in [("week", "u8"), ("dayo", "u16")] {
        let index = store
            .functions()
            .iter()
            .position(|facts| facts.name == name)
            .unwrap();
        let view = analysis.function(index).unwrap();
        let native = view
            .build
            .target_triple
            .known_value()
            .is_some_and(|target| target == "aarch64-unknown-linux-gnu");
        let resolution = view.nominal_resolution();
        if !native {
            assert!(resolution.is_err());
            continue;
        }
        verify_result_source(
            &view,
            &resolution.unwrap_or_else(|error| panic!("{name}: {error}")),
            scalar,
        );
        let pair = rstyle_pdg::producer::produce(&view).unwrap_or_else(|errors| {
            panic!(
                "{name}: {errors:?}; findings {:?}; complete {}",
                view.source_diagnostics, view.source_scan_complete
            )
        });
        verify_document_attacks(&view, &pair);
    }
}

fn verify_result_source(
    view: &dyn EvidenceFacts,
    resolution: &rstyle_pdg::nominal_boundary::resolution::Resolution,
    scalar: &str,
) {
    let path = syn::parse_str(&format!("ParsedItem<'_, NonZero<{scalar}>>")).unwrap();
    let interface = view
        .build
        .frontend_options
        .data::<InterfaceObservations>()
        .unwrap();
    let Observation::Captured {
        ty: TypeTree::Option(payload),
    } = &interface.result
    else {
        panic!("complete result");
    };
    assert_eq!(resolution.resolve(&path).unwrap(), **payload);
    for other in [
        "ParsedItem<'_, u8>",
        "ParsedItem<'_, u16>",
        if scalar == "u8" {
            "ParsedItem<'_, NonZero<u16>>"
        } else {
            "ParsedItem<'_, NonZero<u8>>"
        },
    ] {
        let path = syn::parse_str(other).unwrap();
        assert!(resolution.resolve(&path).is_err(), "{other}");
    }
}

#[test]
fn nonzero_source_receipts_reject_alias_imports_and_uncertified_complete_bodies() {
    for (old, new) in [
        (
            "use core::num::NonZero;",
            "mod aliases { pub use core::num::NonZero; } use aliases::NonZero;",
        ),
        (
            ".and_then(|parsed| parsed.flat_map(NonZero::new))",
            ".and_then(|parsed| { let saved = parsed; saved.flat_map(NonZero::new) })",
        ),
        (
            ".and_then(|parsed| parsed.flat_map(NonZero::new))",
            ".and_then(|parsed| parsed.flat_map(|value| NonZero::new(value)))",
        ),
        (
            ".and_then(|parsed| parsed.flat_map(NonZero::new))",
            ".and_then(|parsed| parsed.flat_map(NonZero::new)).or(None)",
        ),
        (
            "use core::num::NonZero;",
            "use core::num::NonZero; type Option<T> = core::option::Option<T>;",
        ),
    ] {
        let changed = SOURCE.replace(old, new);
        assert_ne!(changed, SOURCE);
        let report = capture(&changed);
        let store = store(&report);
        let analysis = store.analysis();
        for name in ["week", "dayo"] {
            let index = store
                .functions()
                .iter()
                .position(|facts| facts.name == name)
                .unwrap();
            let view = analysis.function(index).unwrap();
            assert!(view.nominal_resolution().is_err(), "{new}");
            assert!(rstyle_pdg::producer::produce(&view).is_err(), "{new}");
        }
    }
}

fn verify_document_attacks(view: &dyn EvidenceFacts, original: &rstyle_pdg::producer::Pair) {
    rstyle_pdg::data_documents::verify(original, view).unwrap();
    for attack in 0..7 {
        let mut pair = rstyle_pdg::producer::Pair {
            pdg: original.pdg.clone(),
            cfg: original.cfg.clone(),
        };
        corrupt_document(&mut pair, attack);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, view).is_err(),
            "document attack {attack}"
        );
    }
}

fn corrupt_document(pair: &mut rstyle_pdg::producer::Pair, attack: usize) {
    let key = "rstyle.data:nonzero-digit-composition";
    match attack {
        0 => pair.pdg.extensions.get_mut(key).unwrap()["semantics"] = json!("zero succeeds"),
        1 => pair.pdg.extensions.get_mut(key).unwrap()["callee"] = Value::Null,
        2 => pair.pdg.profile.version = "3.115.0".into(),
        3 => {
            let scalar = operation_id(pair, "callback.nonzero-scalar");
            pair.pdg
                .edges
                .retain(|edge| edge.target != scalar || edge.subkind != "value_operand");
        }
        4 => {
            let node = pair
                .pdg
                .nodes
                .iter_mut()
                .find(|node| {
                    node.attributes.construct.as_deref() == Some("callback.nonzero-zero-none")
                })
                .unwrap();
            node.attributes.literal = Some(rstyle_pdg::StringFact::known("42"));
        }
        5 => {
            let success = operation_id(pair, "callback.nonzero-success");
            let sites = pair
                .pdg
                .extensions
                .get_mut("compat.rust.hir-r0:value-sites")
                .unwrap()
                .as_array_mut()
                .unwrap();
            for site in sites {
                for alternative in site["alternatives"].as_array_mut().unwrap() {
                    if alternative["value_pdg_node_id"] == success {
                        alternative["value_pdg_node_id"] = json!("missing");
                    }
                }
            }
        }
        6 => remove_nonzero_success_edge(pair),
        _ => unreachable!(),
    }
}

fn operation_id(pair: &rstyle_pdg::producer::Pair, construct: &str) -> String {
    pair.pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some(construct))
        .unwrap()
        .id
        .clone()
}

fn store(report: &Value) -> rstyle_pdg::unit_store::UnitStore {
    let facts = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            serde_json::from_value::<rstyle_pdg::HirFnFacts>(entry["facts"].clone()).unwrap()
        })
        .collect();
    rstyle_pdg::unit_store::UnitStore::capture(facts).unwrap()
}

#[test]
fn nonzero_composition_rejects_changed_member_semantics_and_shadowed_builtins() {
    for changed in [
        SOURCE.replace(
            "Some(ParsedItem(self.0, f(self.1)?))",
            "Some(ParsedItem(&[], f(self.1)?))",
        ),
        SOURCE.replace(
            "Some(ParsedItem(self.0, f(self.1)?))",
            "Some(ParsedItem(self.0, f(self.1).or(None)?))",
        ),
        format!(
            "{SOURCE} const fn Some<T>(value: T) -> Option<T> {{ core::option::Option::Some(value) }}"
        ),
    ] {
        let report = capture(&changed);
        let store = store(&report);
        let analysis = store.analysis();
        for name in ["week", "dayo"] {
            let index = store
                .functions()
                .iter()
                .position(|facts| facts.name == name)
                .unwrap();
            let view = analysis.function(index).unwrap();
            assert!(rstyle_pdg::producer::produce(&view).is_err(), "{name}");
        }
    }
}

fn remove_nonzero_success_edge(pair: &mut rstyle_pdg::producer::Pair) {
    let selector = operation_id(pair, "callback.nonzero-nonzero");
    let mappings = rstyle_pdg::read_operation_map(&pair.pdg.extensions).unwrap();
    let cfg = &mappings
        .iter()
        .find(|entry| entry.pdg_node_id == selector)
        .unwrap()
        .cfg_node_id;
    pair.cfg
        .edges
        .retain(|edge| &edge.source != cfg || edge.kind != rstyle_pdg::CfgEdgeKind::True);
}
