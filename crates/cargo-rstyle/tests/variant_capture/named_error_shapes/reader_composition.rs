//! Complete checked reader composition, with source expectations fixed before capture.
use super::*;
use serde_json::json;

const KEY: &str = "rstyle.data:fallible-reader-origins";
pub(super) const READERS: &str = r#"
use core::mem::size_of;
impl Error {
    pub fn buffer_too_small(what: &'static str) -> Error {
        Error(ErrorKind::BufferTooSmall { what })
    }
}
pub fn check_slice_len<T>(slice: &[T], minimum: usize, what: &'static str) -> Result<(), Error> {
    if slice.len() < minimum { return Err(Error::buffer_too_small(what)); }
    Ok(())
}
pub fn read_u32(slice: &[u8]) -> u32 {
    let bytes: [u8; 4] = slice[..size_of::<u32>()].try_into().unwrap();
    u32::from_ne_bytes(bytes)
}
pub fn reader(slice: &[u8], what: &'static str) -> Result<(u32, usize), Error> {
    check_slice_len(slice, size_of::<u32>(), what)?;
    Ok((read_u32(slice), size_of::<u32>()))
}
pub fn same(slice: &[u8], what: &'static str) -> Result<(u32, usize), Error> {
    check_slice_len(slice, size_of::<u32>(), what)?;
    Ok((read_u32(slice), size_of::<u32>()))
}
pub fn opaque_loop(slice: &[u8], what: &'static str) -> Result<(), Error> {
    for _ in 0..1 { check_slice_len(slice, 4, what)?; }
    Ok(())
}
pub fn changed_comparison<T>(slice: &[T], minimum: usize, what: &'static str) -> Result<(), Error> {
    if slice.len() <= minimum { return Err(Error::buffer_too_small(what)); }
    Ok(())
}
pub fn changed_caller(slice: &[u8], what: &'static str) -> Result<(u32, usize), Error> {
    changed_comparison(slice, size_of::<u32>(), what)?;
    Ok((read_u32(slice), size_of::<u32>()))
}
pub fn wrong_width(slice: &[u8], what: &'static str) -> Result<(u16, usize), Error> {
    check_slice_len(slice, size_of::<u32>(), what)?;
    Ok((0, size_of::<u32>()))
}
pub fn reversed(slice: &[u8], what: &'static str) -> Result<(usize, u32), Error> {
    check_slice_len(slice, size_of::<u32>(), what)?;
    Ok((size_of::<u32>(), read_u32(slice)))
}
pub fn wrong_slice(slice: &[u8], other: &[u8], what: &'static str) -> Result<(u32, usize), Error> {
    check_slice_len(other, size_of::<u32>(), what)?;
    Ok((read_u32(slice), size_of::<u32>()))
}
pub fn early_read(slice: &[u8], what: &'static str) -> Result<(u32, usize), Error> {
    let value = read_u32(slice);
    check_slice_len(slice, size_of::<u32>(), what)?;
    Ok((value, size_of::<u32>()))
}
"#;

#[test]
fn named_error_reader_chain_preserves_native_reads_guarded_error_origins_and_strict_matching() {
    // Expected semantics: short input returns the same static message in Err
    // before reading bytes; success returns the first native u32 and consumed4.
    // Width/order changes, checking another slice and reading before the check
    // are outside the bounded family. The whole original-crate runtime oracle
    // independently tests byte values, boundaries and message pointer/length.
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{SOURCE}\n{READERS}"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["Error::buffer_too_small", "read_u32", "reader", "same"] {
        let entry = function(&report, name);
        assert_eq!(
            entry["outcome"],
            "admitted",
            "{name}: {}; instance: {}",
            entry["reasons"],
            instance_diagnostic(&report),
        );
    }
    for name in [
        "changed_comparison",
        "changed_caller",
        "wrong_width",
        "reversed",
        "wrong_slice",
        "early_read",
        "opaque_loop",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
    // Concrete activation must retain the unsupported original generic owner.
    assert_eq!(function(&report, "check_slice_len")["outcome"], "excluded");
    check_equivalent_matching(&report, ["reader", "same"]);
    check_strict_reconstruction(&report);
}

fn instance_diagnostic(report: &Value) -> String {
    let (raw, ordinal) = shared_evidence::load(function(report, "check_slice_len"));
    match rstyle_pdg::unit_store::UnitStore::capture_slice_length_instance(
        std::sync::Arc::new(raw),
        ordinal,
    ) {
        Ok(_) => "certified".into(),
        Err(error) => error,
    }
}

fn check_strict_reconstruction(report: &Value) {
    let entry = function(report, "reader");
    rstyle_pdg::verification_policy::strict(|| {
        let (raw, index) = shared_evidence::load(entry);
        let originals = raw.functions().to_vec();
        let retained = retained_pairs(&raw);
        // This Cargo fixture has no external dependencies. Reconstruct the local
        // concrete instance closure once; the original raw root stays excluded.
        let closure = rstyle_pdg::unit_store::DependencyClosure::from_units_with(
            vec![raw],
            &[],
            |units, associations| {
                assert_eq!(units.len(), 1);
                assert!(associations.is_empty());
                Ok(())
            },
        )
        .unwrap();
        let store = closure.unit(0).unwrap();
        assert_eq!(store.functions(), originals);
        for (ordinal, serialized) in retained {
            let replay = store.pair(ordinal).unwrap().serialize().unwrap();
            assert_eq!(replay.pdg_bytes, serialized.pdg_bytes);
            assert_eq!(replay.cfg_bytes, serialized.cfg_bytes);
        }
        let analysis = store.analysis();
        let facts = analysis.function(index).unwrap();
        let pair = store.pair(index).unwrap();
        check_documents(pair, &facts);
        reject_mutations(pair, &facts);
        check_strict_matching(store, report);
    });
}

fn retained_pairs(
    store: &rstyle_pdg::unit_store::UnitStore,
) -> Vec<(usize, rstyle_pdg::producer::SerializedPair)> {
    (0..store.functions().len())
        .filter_map(|ordinal| {
            store
                .pair(ordinal)
                .map(|pair| (ordinal, pair.serialize().unwrap()))
        })
        .collect()
}

fn check_strict_matching(store: &rstyle_pdg::unit_store::UnitStore, report: &Value) {
    let analysis = store.analysis();
    let imported: Vec<_> = ["reader", "same"]
        .map(|name| {
            let index = shared_evidence::index(function(report, name));
            let facts = analysis.function(index).unwrap();
            rstyle_ccgraph::admit_produced(store.pair(index).unwrap(), &facts).unwrap()
        })
        .into_iter()
        .collect();
    let matching =
        rstyle_ccgraph::evaluate_all(&imported, &rstyle_ccgraph::CcgraphConfig::default());
    assert!(!matching.incomplete, "{:?}", matching.diagnostics);
    assert_eq!(matching.pairs.len(), 1);
    assert_eq!(
        matching.pairs[0].decision,
        rstyle_ccgraph::Decision::Accepted
    );
    assert_eq!(matching.pairs[0].displayed_score.as_deref(), Some("1"));
}

fn check_documents(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    rstyle_pdg::verification_policy::strict(|| {
        rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), facts).unwrap();
        rstyle_ccgraph::admit_produced(pair, facts).unwrap();
    });
    let error = role(&pair.pdg.nodes, "propagated_error");
    assert_eq!(error.annotations[KEY]["origins"][0]["root"]["parameter"], 1);
    assert_eq!(error.annotations[KEY]["guard"]["outcome"], "true");
    let length = role(&pair.pdg.nodes, "length_result");
    assert_eq!(
        length.annotations[KEY]["scope"]["actuals"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(length.annotations[KEY]["cases"][0]["origins"], json!([]));
    let native = role(&pair.pdg.nodes, "native_scalar");
    assert!(!native.attributes.reads.is_empty());
    assert!(error.attributes.reads.is_empty());
    for node in &pair.pdg.nodes {
        assert!(node.annotations.contains_key(KEY));
        assert!(!node.annotations.contains_key(rstyle_pdg::data_origins::KEY));
        if matches!(
            node.annotations[KEY]["role"].as_str(),
            Some("ok_result" | "unit_success")
        ) {
            assert_eq!(node.annotations[KEY]["origins"], json!([]));
        }
    }
}

fn role<'a>(nodes: &'a [rstyle_pdg::PdgNode], expected: &str) -> &'a rstyle_pdg::PdgNode {
    nodes
        .iter()
        .find(|node| node.annotations[KEY]["role"] == expected)
        .unwrap()
}

fn reject_mutations(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    for change in 0..4 {
        let mut changed = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        mutate(&mut changed.pdg.nodes, change);
        assert!(
            rstyle_pdg::producer::verify(&changed, facts).is_err(),
            "fallible reader guarded component mutation {change} was accepted"
        );
    }
}

fn mutate(nodes: &mut [rstyle_pdg::PdgNode], change: usize) {
    let role = if change == 1 {
        "ok_result"
    } else {
        "propagated_error"
    };
    let node = nodes
        .iter_mut()
        .find(|node| node.annotations[KEY]["role"] == role)
        .unwrap();
    let document = node.annotations.get_mut(KEY).unwrap();
    match change {
        0 => document["origins"][0]["root"]["parameter"] = json!(0),
        1 => document["origins"] = json!([{"root":{"parameter":1},"route":[]}]),
        2 => document["guard"]["outcome"] = json!("false"),
        _ => document["scope"]["saved_actual_pdg_node_ids"] = json!([]),
    }
}
