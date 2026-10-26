//! Original ID readers compose generated decoders with owned error callbacks.
use super::*;

const KEY: &str = "rstyle.data:id-reader-origins";
const GENERATED: &str = r#"
#![no_std]
mod generated {
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Attempt { attempted: u64 }
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct SmallIndex(u32);
impl SmallIndex {
    pub const SIZE: usize = core::mem::size_of::<SmallIndex>();
    pub fn from_ne_bytes(bytes: [u8; 4]) -> Result<SmallIndex, Attempt> {
        let value = u32::from_ne_bytes(bytes);
        if value > 100 { return Err(Attempt { attempted: u64::from(value) }); }
        Ok(SmallIndex(usize::try_from(value).expect("u32 overflowed usize") as u32))
    }
}
#[derive(Clone, Copy)]
pub struct FirstID(SmallIndex);
#[derive(Clone, Copy)]
pub struct SecondID(SmallIndex);
macro_rules! identifiers {
    ($name:ident, $err:ident, $iter:ident, $withiter:ident) => {
        impl $name {
            pub const SIZE: usize = SmallIndex::SIZE;
            pub fn from_ne_bytes(bytes: [u8; 4]) -> Result<$name, $err> {
                SmallIndex::from_ne_bytes(bytes).map($name).map_err($err)
            }
        }
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $err(Attempt);
    };
}
identifiers!(FirstID, FirstError, FirstIter, WithFirstIter);
identifiers!(SecondID, SecondError, SecondIter, WithSecondIter);
}
"#;

const READERS: &str = r#"
use generated::{FirstID, SecondID};
impl Error {
    pub fn first(err: FirstError, what: &'static str) -> Error {
        Error(ErrorKind::First { err, what })
    }
    pub fn second(err: SecondError, what: &'static str) -> Error {
        Error(ErrorKind::Second { err, what })
    }
    pub fn discarded(err: FirstError, what: &'static str) -> Error {
        Error(ErrorKind::Generic { msg: what })
    }
    pub fn buffer_too_small(what: &'static str) -> Error {
        Error(ErrorKind::BufferTooSmall { what })
    }
}
pub fn read_first(slice: &[u8], what: &'static str) -> Result<(FirstID, usize), Error> {
    let bytes: [u8; FirstID::SIZE] = slice[..FirstID::SIZE].try_into().unwrap();
    let id = FirstID::from_ne_bytes(bytes).map_err(|err| Error::first(err, what))?;
    Ok((id, FirstID::SIZE))
}
pub fn read_first_same(bytes: &[u8], label: &'static str) -> Result<(FirstID, usize), Error> {
    let input: [u8; FirstID::SIZE] = bytes[..FirstID::SIZE].try_into().unwrap();
    let value = FirstID::from_ne_bytes(input).map_err(|error| Error::first(error, label))?;
    Ok((value, FirstID::SIZE))
}
pub fn read_second(slice: &[u8], what: &'static str) -> Result<(SecondID, usize), Error> {
    let bytes: [u8; SecondID::SIZE] = slice[..SecondID::SIZE].try_into().unwrap();
    let id = SecondID::from_ne_bytes(bytes).map_err(|err| Error::second(err, what))?;
    Ok((id, SecondID::SIZE))
}
pub fn try_read_second(slice: &[u8], what: &'static str) -> Result<(SecondID, usize), Error> {
    if slice.len() < SecondID::SIZE { return Err(Error::buffer_too_small(what)); }
    read_second(slice, what)
}
pub fn wrong_error(slice: &[u8], what: &'static str) -> Result<(FirstID, usize), Error> {
    let bytes: [u8; FirstID::SIZE] = slice[..FirstID::SIZE].try_into().unwrap();
    let id = FirstID::from_ne_bytes(bytes).map_err(|err| Error::discarded(err, what))?;
    Ok((id, FirstID::SIZE))
}
pub fn changed_message(slice: &[u8], what: &'static str) -> Result<(FirstID, usize), Error> {
    let bytes: [u8; FirstID::SIZE] = slice[..FirstID::SIZE].try_into().unwrap();
    let id = FirstID::from_ne_bytes(bytes).map_err(|err| Error::first(err, "changed"))?;
    Ok((id, FirstID::SIZE))
}
pub fn wrong_count(slice: &[u8], what: &'static str) -> Result<(FirstID, usize), Error> {
    let bytes: [u8; FirstID::SIZE] = slice[..FirstID::SIZE].try_into().unwrap();
    let id = FirstID::from_ne_bytes(bytes).map_err(|err| Error::first(err, what))?;
    Ok((id, 0))
}
pub fn wrong_guard(slice: &[u8], what: &'static str) -> Result<(SecondID, usize), Error> {
    if slice.len() <= SecondID::SIZE { return Err(Error::buffer_too_small(what)); }
    read_second(slice, what)
}
"#;

fn source() -> String {
    let (_, rest) = SOURCE.split_once("mod api {").unwrap();
    format!("{GENERATED}\nmod api {{{rest}\n{READERS}")
}

#[test]
fn checked_id_readers_preserve_generated_decoders_owned_errors_and_exact_counts() {
    // Independently expected: reads consume four bytes; decoded values0..100
    // return the ID/count, other values preserve the original owned error and
    // static message. The checked wrapper rejects lengths0..3 before any read.
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in [
        "read_first",
        "read_first_same",
        "read_second",
        "try_read_second",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
    }
    for name in [
        "wrong_error",
        "changed_message",
        "wrong_count",
        "wrong_guard",
    ] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
    strict_replay(&report);
}

fn strict_replay(report: &Value) {
    let (raw, _) = shared_evidence::load(function(report, "read_first"));
    let originals = raw.functions().to_vec();
    let store = raw.with_slice_length_instances().unwrap();
    assert_eq!(store.functions(), originals);
    rstyle_pdg::verification_policy::strict(|| {
        let analysis = store.analysis();
        let mut imports = Vec::new();
        for name in [
            "read_first",
            "read_first_same",
            "read_second",
            "try_read_second",
        ] {
            let index = shared_evidence::index(function(report, name));
            let facts = analysis.function(index).unwrap();
            let pair = store.pair(index).unwrap();
            rstyle_pdg::producer::verify_serialized(&pair.serialize().unwrap(), &facts).unwrap();
            let imported = rstyle_ccgraph::admit_produced(pair, &facts).unwrap();
            if name == "read_first" {
                reject_mutations(pair, &facts);
            }
            if matches!(name, "read_first" | "try_read_second") {
                reject_cfg_bypasses(pair, &facts, name == "read_first");
            }
            if matches!(name, "read_first" | "read_first_same") {
                imports.push(imported);
            }
        }
        let matching =
            rstyle_ccgraph::evaluate_all(&imports, &rstyle_ccgraph::CcgraphConfig::default());
        assert!(!matching.incomplete, "{:?}", matching.diagnostics);
        assert_eq!(matching.pairs.len(), 1);
        assert_eq!(
            matching.pairs[0].decision,
            rstyle_ccgraph::Decision::Accepted
        );
        assert_eq!(matching.pairs[0].displayed_score.as_deref(), Some("1"));
    });
}

fn reject_cfg_bypasses(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
    map: bool,
) {
    rstyle_pdg::id_error_maps::verify_documents(pair, facts).unwrap();
    let selector = if map { "id-map.result-is-err" } else { "if" };
    for outcome in [
        rstyle_pdg::CfgEdgeKind::True,
        rstyle_pdg::CfgEdgeKind::False,
    ] {
        let mut changed = pair.clone();
        bypass(&mut changed, selector, outcome);
        assert!(
            rstyle_pdg::id_error_maps::verify_documents(&changed, facts).is_err(),
            "independent {selector} {outcome:?} bypass"
        );
    }
    if map {
        let mut changed = pair.clone();
        bypass(
            &mut changed,
            "id-map.wrap-error",
            rstyle_pdg::CfgEdgeKind::Normal,
        );
        assert!(
            rstyle_pdg::id_error_maps::verify_documents(&changed, facts).is_err(),
            "independent mapped error-to-exit bypass"
        );
    }
}

fn bypass(pair: &mut rstyle_pdg::producer::Pair, construct: &str, kind: rstyle_pdg::CfgEdgeKind) {
    let node = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some(construct))
        .unwrap();
    let mapping = rstyle_pdg::read_operation_map(&pair.pdg.extensions).unwrap();
    let source = mapping
        .iter()
        .find(|entry| entry.pdg_node_id == node.id)
        .unwrap();
    pair.cfg.edges.push(rstyle_pdg::CfgEdge {
        id: format!("forged-{construct}-{kind:?}"),
        source: source.cfg_node_id.clone(),
        target: pair.cfg.exit_node_ids.first().unwrap().clone(),
        kind,
        case_value: None,
        loop_role: None,
        reason: None,
    });
}

fn reject_mutations(
    pair: &rstyle_pdg::producer::Pair,
    facts: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) {
    for change in ["count", "message", "transfer", "read", "origins", "policy"] {
        let mut changed = pair.clone();
        mutate(&mut changed, change);
        assert!(
            rstyle_pdg::producer::verify(&changed, facts).is_err(),
            "ID reader mutation {change}"
        );
    }
}

fn mutate(pair: &mut rstyle_pdg::producer::Pair, change: &str) {
    match change {
        "policy" => {
            pair.pdg.profile.version = "3.124.0".into();
            pair.cfg.profile.version = "3.124.0".into();
        }
        "count" => change_count(pair),
        "read" => {
            pair.pdg
                .nodes
                .iter_mut()
                .find(|node| {
                    node.attributes
                        .reads
                        .iter()
                        .any(|read| read.location_id.is_some())
                })
                .unwrap()
                .attributes
                .reads
                .clear();
        }
        "origins" => {
            let node = pair
                .pdg
                .nodes
                .iter_mut()
                .find(|node| node.annotations.contains_key(KEY))
                .unwrap();
            node.annotations.remove(KEY);
        }
        _ => {
            let construct = if change == "message" {
                "id-map.captured-message"
            } else {
                "try.id-reader-result-error"
            };
            let node = pair
                .pdg
                .nodes
                .iter_mut()
                .find(|node| node.attributes.construct.as_deref() == Some(construct))
                .unwrap();
            node.attributes.construct = Some("id-map.success-payload".into());
        }
    }
}

fn change_count(pair: &mut rstyle_pdg::producer::Pair) {
    let tuple = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.original_kind.known_value() == Some("expr_tup"))
        .unwrap();
    let edge = pair
        .pdg
        .edges
        .iter_mut()
        .find(|edge| {
            edge.target == tuple.id
                && edge.subkind == "value_operand"
                && edge.operand_position == Some(1)
        })
        .unwrap();

    edge.operand_position = Some(0);
}

#[test]
fn id_readers_do_not_hide_drop_in_an_unused_generated_error_variant() {
    let directory = clone_workspace();
    let source = source()
        + "\nmod hidden { use crate::FirstError as Alias; impl Drop for Alias { fn drop(&mut self) {} } }";
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["read_first", "read_second", "try_read_second"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "excluded", "{name}: {}", entry["reasons"]);
    }
}
