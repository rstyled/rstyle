//! Original checked widths share error ownership without granting blanket conversion.
use super::*;
use rstyle_pdg::{producer::Pair, unit_store::UnitStore};
use serde_json::json;

const KEY: &str = "rstyle.data:fallible-reader-origins";
const NAME: &str = "try_read_u16";

fn selection() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../eval/coverage-development/wire-usize-readers/selection.json"
    ))
    .unwrap()
}

fn selected<'a>(selection: &'a Value, name: &str) -> &'a str {
    selection["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == format!("util::wire::{name}"))
        .unwrap()["source"]
        .as_str()
        .unwrap()
}

fn source() -> String {
    let selection = selection();
    let original = selected(&selection, NAME);
    let schema = SOURCE
        .split_once("pub fn arithmetic_overflow")
        .unwrap()
        .0
        .replace("ErrorKind", "DeserializeErrorKind")
        .replace("pub struct Error(", "pub struct DeserializeError(");
    let constructors = ["buffer_too_small", "invalid_usize"]
        .map(|name| selected(&selection, &format!("DeserializeError::{name}")));
    let prelude = format!(
        "use core::mem::size_of;\nimpl DeserializeError {{ {} {} }}",
        constructors[0], constructors[1]
    );
    let dependencies = [
        "check_slice_len",
        "read_u16",
        "read_u32",
        NAME,
        "try_read_u32",
        "try_read_u16_as_usize",
    ]
    .map(|name| selected(&selection, name).to_owned());
    [schema, prelude]
        .into_iter()
        .chain(dependencies)
        .chain([original.replace(NAME, "same_u16")])
        .chain(negatives(original))
        .chain([
            "fn wrong_reader(_: &[u8]) -> u16 { 0 }".into(),
            selected(&selection, "read_u16")
                .replace("read_u16", "big_endian_reader")
                .replace("from_ne_bytes", "from_be_bytes"),
        ])
        .collect::<Vec<_>>()
        .join("\n")
}

fn negatives(original: &str) -> Vec<String> {
    let precheck = "check_slice_len(slice, size_of::<u16>(), what)";
    [
        (
            "wrong_precheck",
            precheck,
            "check_slice_len(slice, size_of::<u32>(), what)",
        ),
        (
            "wrong_consumed",
            "Ok((read_u16(slice), size_of::<u16>()))",
            "Ok((read_u16(slice), size_of::<u32>()))",
        ),
        ("wrong_helper", "read_u16(slice)", "wrong_reader(slice)"),
        (
            "wrong_endian",
            "read_u16(slice)",
            "big_endian_reader(slice)",
        ),
    ]
    .map(|(name, before, after)| {
        assert!(
            original.contains(before),
            "source mutation must apply: {name}"
        );
        original.replace(NAME, name).replace(before, after)
    })
    .into_iter()
    .collect()
}

#[test]
fn original_u16_reader_preserves_two_byte_bounds_count_and_named_error() {
    // Independently fixed by the exhaustive original-source oracle: lengths0/1
    // return BufferTooSmall carrying the exact static message before reading;
    // lengths>=2 decode the first native two bytes and return consumed2.
    // Trailing bytes never affect the result. Blanket/Infallible widening is
    // separately unsupported, even though its checked-reader dependency admits.
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "status={} stderr={} handoff={} capture={}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
        report["handoff_failures"],
        report["capture_failures"]
    );
    check_frontier(&report);
    let (raw, index) = shared_evidence::load(function(&report, NAME));
    let originals = raw.functions().to_vec();
    let store = raw.with_slice_length_instances().unwrap();
    assert_eq!(store.functions(), originals);
    strict_matching(&store, &report);
    check_width_evidence(&store, index);
    reject_graphs(&store, index);
    check_prior_profile(
        &store,
        shared_evidence::index(function(&report, "try_read_u32")),
    );
}

fn check_frontier(report: &Value) {
    for name in ["read_u16", "read_u32", NAME, "same_u16", "try_read_u32"] {
        let entry = function(report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    for name in [
        "wrong_precheck",
        "wrong_consumed",
        "wrong_helper",
        "wrong_endian",
        "try_read_u16_as_usize",
        "check_slice_len",
    ] {
        let entry = function(report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
}

fn strict_matching(store: &UnitStore, report: &Value) {
    rstyle_pdg::verification_policy::strict(|| {
        let analysis = store.analysis();
        let imported: Vec<_> = [NAME, "same_u16"]
            .map(|name| {
                let index = shared_evidence::index(function(report, name));
                let facts = analysis.function(index).unwrap();
                let pair = store.pair(index).unwrap();
                rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts)
                    .unwrap();
                rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
                rstyle_ccgraph::admit_produced(pair, &facts).unwrap()
            })
            .into_iter()
            .collect();
        let matches =
            rstyle_ccgraph::evaluate_all(&imported, &rstyle_ccgraph::CcgraphConfig::default());
        assert!(!matches.incomplete, "{:?}", matches.diagnostics);
        assert_eq!(matches.pairs.len(), 1);
        assert_eq!(
            matches.pairs[0].decision,
            rstyle_ccgraph::Decision::Accepted
        );
        assert_eq!(matches.pairs[0].displayed_score.as_deref(), Some("1"));
    });
}

fn role<'a>(pair: &'a Pair, expected: &str) -> &'a rstyle_pdg::PdgNode {
    let matches: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.annotations[KEY]["role"] == expected
                && node.category != rstyle_pdg::NodeCategory::Return
        })
        .collect();
    assert_eq!(matches.len(), 1, "unique role: {expected}");
    matches[0]
}

fn check_width_evidence(store: &UnitStore, index: usize) {
    let facts = &store.functions()[index];
    assert_eq!(facts.signature.params.len(), 2);
    assert!(
        facts
            .source
            .bytes
            .contains("check_slice_len(slice, size_of::<u16>(), what)?")
    );
    assert!(
        facts
            .source
            .bytes
            .contains("Ok((read_u16(slice), size_of::<u16>()))")
    );
    let pair = store.pair(index).unwrap();
    let error = role(pair, "propagated_error");
    assert_eq!(error.annotations[KEY]["origins"][0]["root"]["parameter"], 1);
    assert_eq!(error.annotations[KEY]["guard"]["outcome"], "true");
    assert!(error.attributes.reads.is_empty());
    assert!(!role(pair, "native_scalar").attributes.reads.is_empty());
    assert_eq!(
        role(pair, "length_result").annotations[KEY]["cases"][0]["origins"],
        json!([])
    );
    let sizes: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.annotations[KEY]["role"] == "scalar_size")
        .collect();
    assert_eq!(sizes.len(), 2, "saved precheck bound and consumed count");
    for node in sizes {
        assert_eq!(node.annotations[KEY]["origins"], json!([]));
    }
}

fn reject_graphs(store: &UnitStore, index: usize) {
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    let original = store.pair(index).unwrap();
    for change in [
        "origin",
        "guard",
        "read",
        "saved-arguments",
        "historical",
        "entry-bypass",
        "repeat-read",
        "exit-kind",
        "source",
    ] {
        let mut changed = original.clone();
        mutate_graph(&mut changed, change);
        let serialized = changed.serialize().unwrap();
        rstyle_pdg::verification_policy::strict(|| {
            assert!(
                rstyle_pdg::producer::verify_serialized(&serialized, &facts).is_err(),
                "{change}"
            );
            if change != "source" {
                assert!(
                    rstyle_pdg::data_documents::verify(&changed, &facts).is_err(),
                    "{change}"
                );
            }
        });
    }
}

fn mutate_graph(pair: &mut Pair, change: &str) {
    if mutate_flow(pair, change) {
        return;
    }
    if change == "historical" {
        pair.pdg.profile.version = "3.127.0".into();
        pair.cfg.profile.version = "3.127.0".into();
        return;
    }
    if change == "source" {
        pair.pdg.sources[0].content_identity = rstyle_pdg::StringFact::known(
            &rstyle_pdg::source_identity::checksum(b"different original reader"),
        );
        return;
    }
    let expected = match change {
        "read" => "native_scalar",
        "saved-arguments" => "length_result",
        _ => "propagated_error",
    };
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.annotations[KEY]["role"] == expected)
        .unwrap();
    match change {
        "read" => node.attributes.reads.clear(),
        "origin" => {
            node.annotations.get_mut(KEY).unwrap()["origins"][0]["root"]["parameter"] = 0.into()
        }
        "guard" => node.annotations.get_mut(KEY).unwrap()["guard"]["outcome"] = "false".into(),
        "saved-arguments" => {
            node.annotations.get_mut(KEY).unwrap()["scope"]["saved_actual_pdg_node_ids"] = json!([])
        }
        _ => unreachable!(),
    }
}

fn mutate_flow(pair: &mut Pair, change: &str) -> bool {
    if change == "exit-kind" {
        let exit = &pair.cfg.exit_node_ids[0];
        let node = pair
            .cfg
            .nodes
            .iter_mut()
            .find(|node| &node.id == exit)
            .unwrap();
        node.exit_kind = Some(rstyle_pdg::CfgExitKind::Exceptional);
    } else if matches!(change, "entry-bypass" | "repeat-read") {
        let mut edge = pair
            .cfg
            .edges
            .iter()
            .find(|edge| edge.source == pair.cfg.entry_node_id)
            .unwrap()
            .clone();
        edge.id = "adversarial-extra-transfer".into();
        if change == "entry-bypass" {
            edge.target = pair.cfg.exit_node_ids[0].clone();
        } else {
            edge.source = edge.target.clone();
        }
        pair.cfg.edges.push(edge);
    } else {
        return false;
    }
    true
}

fn check_prior_profile(store: &UnitStore, index: usize) {
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    let original = store.pair(index).unwrap();
    let mut historical = original.clone();
    historical.pdg.profile.version = "3.127.0".into();
    historical.cfg.profile.version = "3.127.0".into();
    rstyle_pdg::verification_policy::strict(|| {
        // Superseded Data3 policy IDs were not registered by this pipeline.
        // Keep that closed; unchanged u32 admission is checked under current policy.
        assert!(!rstyle_pdg::extraction_profile::registered(
            &historical.pdg.profile
        ));
        assert!(
            rstyle_pdg::producer::verify_serialized(&historical.serialize().unwrap(), &facts)
                .is_err()
        );
        assert!(rstyle_pdg::data_documents::verify(&historical, &facts).is_err());
    });
}
