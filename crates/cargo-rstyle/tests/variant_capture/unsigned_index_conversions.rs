//! Complete unsigned conversion chains, including exact local/core helpers.
use super::float_kernels::function;
use super::*;

const INPUTS: &str = r#"
trait U16 { fn as_usize(self) -> usize; }
impl U16 for u16 { fn as_usize(self) -> usize { usize::from(self) } }
trait U32 { fn as_usize(self) -> usize; }
impl U32 for u32 { fn as_usize(self) -> usize { usize::try_from(self).expect("u32 overflowed usize") } }
trait U64 { fn as_usize(self) -> usize; }
impl U64 for u64 { fn as_usize(self) -> usize { usize::try_from(self).expect("u64 overflowed usize") } }
impl TryFrom<u16> for Index {
    type Error = LimitError;
    fn try_from(value: u16) -> Result<Index, LimitError> {
        if u32::from(value) > Index::LIMIT.value() as u32 {
            return Err(LimitError { attempted: u64::from(value) });
        }
        Ok(Index::make(value.as_usize()))
    }
}
impl TryFrom<u32> for Index {
    type Error = LimitError;
    fn try_from(value: u32) -> Result<Index, LimitError> {
        if value > Index::LIMIT.value() as u32 {
            return Err(LimitError { attempted: u64::from(value) });
        }
        Ok(Index::make(value.as_usize()))
    }
}
impl TryFrom<u64> for Index {
    type Error = LimitError;
    fn try_from(value: u64) -> Result<Index, LimitError> {
        if value > Index::LIMIT.value() as u64 {
            return Err(LimitError { attempted: value });
        }
        Ok(Index::make(value.as_usize()))
    }
}
pub fn from_u16(value: u16) -> Result<Index, LimitError> { Index::try_from(value) }
pub fn from_u32(value: u32) -> Result<Index, LimitError> { Index::try_from(value) }
pub fn from_u64(value: u64) -> Result<Index, LimitError> { Index::try_from(value) }
"#;

#[test]
fn unsigned_index_family_retains_complete_input_helpers_errors_and_strict_policy() {
    let directory = clone_workspace();
    let source = format!("{}\n{INPUTS}", super::checked_index_construction::SOURCE);
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for input in ["u16", "u32", "u64", "usize"] {
        let name = format!("<Index as core::convert::TryFrom<{input}>>::try_from");
        let entry = function(&report, &name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
        reject_older_policy(entry);
    }
    for name in ["from_u16", "from_u32", "from_u64"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
        reject_older_policy(entry);
    }
}

fn reject_older_policy(entry: &Value) {
    if entry["facts"]["signature"]["params"][0]["ty"]["normalized"] == "usize" {
        return;
    }
    let (unit, index) = data_unit(entry);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = unit.pair(index).unwrap();
    let mut wire = serde_json::to_value(pair).unwrap();
    for graph in ["pdg", "cfg"] {
        wire[graph]["profile"]["version"] = "3.116.0".into();
    }
    let forged: rstyle_pdg::producer::Pair = serde_json::from_value(wire).unwrap();
    assert!(rstyle_ccgraph::admit_produced(&forged, &facts).is_err());
}
