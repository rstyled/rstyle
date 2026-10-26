//! Original validator compositions exercise saved values, errors and byte reads.
use super::*;
use serde_json::json;

const KEY: &str = "rstyle.data:wire-validator-origins";

const VALIDATORS: &str = r#"
impl Error {
    pub fn version_mismatch(expected: u32, found: u32) -> Error {
        Error(ErrorKind::VersionMismatch { expected, found })
    }
    pub fn endian_mismatch(expected: u32, found: u32) -> Error {
        Error(ErrorKind::EndianMismatch { expected, found })
    }
}
pub fn write_version_len() -> usize { size_of::<u32>() }
pub fn write_endianness_check_len() -> usize { size_of::<u32>() }
pub fn read_version(slice: &[u8], expected_version: u32) -> Result<usize, Error> {
    let (n, nr) = reader(slice, "version")?;
    assert_eq!(nr, write_version_len());
    if n != expected_version {
        return Err(Error::version_mismatch(expected_version, n));
    }
    Ok(nr)
}
pub fn same_version(slice: &[u8], expected_version: u32) -> Result<usize, Error> {
    let (n, nr) = reader(slice, "version")?;
    assert_eq!(nr, write_version_len());
    if n != expected_version {
        return Err(Error::version_mismatch(expected_version, n));
    }
    Ok(nr)
}
pub fn read_endianness_check(slice: &[u8]) -> Result<usize, Error> {
    let (n, nr) = reader(slice, "endianness check")?;
    assert_eq!(nr, write_endianness_check_len());
    if n != 0xFEFF {
        return Err(Error::endian_mismatch(0xFEFF, n));
    }
    Ok(nr)
}
pub fn swapped_error(slice: &[u8], expected: u32) -> Result<usize, Error> {
    let (n, nr) = reader(slice, "version")?;
    assert_eq!(nr, write_version_len());
    if n != expected { return Err(Error::version_mismatch(n, expected)); }
    Ok(nr)
}
pub fn wrong_return(slice: &[u8], expected: u32) -> Result<usize, Error> {
    let (n, nr) = reader(slice, "version")?;
    assert_eq!(nr, write_version_len());
    if n != expected { return Err(Error::version_mismatch(expected, n)); }
    Ok(0)
}
pub fn extra_work(slice: &[u8], expected: u32) -> Result<usize, Error> {
    let (n, nr) = reader(slice, "version")?;
    let hidden = n;
    assert_eq!(nr, write_version_len());
    if n != expected { return Err(Error::version_mismatch(expected, n)); }
    Ok(nr)
}
"#;

#[test]
fn named_error_validators_compose_callable_reads_tuple_payloads_and_static_error_roots() {
    // Expectations fixed from original source: lengths0..3 return BufferTooSmall
    // before mismatch, success consumes4, scalar errors retain expected/found
    // order. The unchanged-source runtime oracle supplies independent values.
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{SOURCE}\n{}\n{VALIDATORS}", reader_composition::READERS),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in [
        "Error::version_mismatch",
        "Error::endian_mismatch",
        "read_version",
        "same_version",
        "read_endianness_check",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    for name in [
        "check_slice_len",
        "swapped_error",
        "wrong_return",
        "extra_work",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
    check_equivalent_matching(&report, ["read_version", "same_version"]);
    check_strict(&report);
}

fn check_strict(report: &Value) {
    let (raw, _) = shared_evidence::load(function(report, "read_version"));
    let original = raw.functions().to_vec();
    let store = raw.with_slice_length_instances().unwrap();
    assert_eq!(store.functions(), original);
    rstyle_pdg::verification_policy::strict(|| {
        let analysis = store.analysis();
        let mut imports = Vec::new();
        for name in ["read_version", "same_version", "read_endianness_check"] {
            let index = shared_evidence::index(function(report, name));
            let view = analysis.function(index).unwrap();
            let pair = store.pair(index).unwrap();
            rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &view).unwrap();
            reject_mutations(pair, &view);
            imports.push(rstyle_ccgraph::admit_produced(pair, &view).unwrap());
        }
        let result =
            rstyle_ccgraph::evaluate_all(&imports, &rstyle_ccgraph::CcgraphConfig::default());
        assert!(!result.incomplete);
        assert_eq!(result.pairs.len(), 3);
    });
}

fn reject_mutations(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    for change in 0..5 {
        let mut changed = rstyle_pdg::producer::Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        mutate(&mut changed.pdg.nodes, change);
        // Exercise semantic verification directly; originals cover wire integrity.
        assert!(
            rstyle_pdg::producer::verify(&changed, facts).is_err(),
            "validator evidence mutation {change} was accepted"
        );
    }
}

fn mutate(nodes: &mut [rstyle_pdg::PdgNode], change: usize) {
    let role = match change {
        0 => "static_literal",
        3 => "guarded_reader_result",
        _ => "propagated_static_error",
    };
    let node = nodes
        .iter_mut()
        .find(|node| node.annotations[KEY]["role"] == role)
        .unwrap();
    if change == 3 {
        assert!(!node.attributes.reads.is_empty());
        node.attributes.reads.clear();
        return;
    }
    let record = node.annotations.get_mut(KEY).unwrap();
    match change {
        0 => record["origins"][0]["root"]["descriptor"]["value"] = json!("wrong message"),
        1 => record["scope"]["saved_actual_pdg_node_ids"] = json!([]),
        2 => record["guard"]["outcome"] = json!("false"),
        _ => record["origins"] = json!([]),
    }
}
