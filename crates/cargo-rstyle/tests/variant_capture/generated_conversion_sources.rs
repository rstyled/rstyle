//! Source/instance proof is deliberately independent of admission and cleanup.
use super::{float_kernels::function, *};
use rstyle_pdg::{
    HirFnFacts, nominal_boundary::resolution::probe_generated_conversion_source as probe,
    unit_store::EvidenceFacts,
};
use std::sync::Arc;
mod attacks;

const WRAPPERS: &str = r#"
#[derive(Clone, Copy)] pub struct First(Index);
#[derive(Clone, Copy)] pub struct Second(Index);
macro_rules! conversions {
    ($name:ident, $err:ident) => {
        pub struct $err(LimitError);
        impl TryFrom<u16> for $name {
            type Error = $err;
            fn try_from(value: u16) -> Result<$name, $err> {
                Index::try_from(value).map($name).map_err($err)
            }
        }
        impl TryFrom<u32> for $name {
            type Error = $err;
            fn try_from(value: u32) -> Result<$name, $err> {
                Index::try_from(value).map($name).map_err($err)
            }
        }
        impl TryFrom<u64> for $name {
            type Error = $err;
            fn try_from(value: u64) -> Result<$name, $err> {
                Index::try_from(value).map($name).map_err($err)
            }
        }
        impl TryFrom<usize> for $name {
            type Error = $err;
            fn try_from(value: usize) -> Result<$name, $err> {
                Index::try_from(value).map($name).map_err($err)
            }
        }
    };
}
conversions!(First, FirstError);
conversions!(Second, SecondError);
"#;

#[test]
fn generated_conversion_source_proofs_cover_complete_chain_without_granting_admission() {
    let directory = clone_workspace();
    let source = format!(
        "{}\n{}\n{WRAPPERS}",
        super::checked_index_construction::SOURCE,
        super::unsigned_index_conversions::INPUTS
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let first = function(&report, "<First as core::convert::TryFrom<u16>>::try_from");
    let (store, _) = data_unit(first);
    let analysis = store.analysis();
    for owner in ["First", "Second"] {
        for input in ["u16", "u32", "u64", "usize"] {
            let name = format!("<{owner} as core::convert::TryFrom<{input}>>::try_from");
            let entry = function(&report, &name);
            assert_eq!(
                entry["outcome"], "excluded",
                "diagnostic source proof is not admission"
            );
            let index = crate::shared_evidence::index(entry);
            let view = analysis.function(index).unwrap();
            assert_eq!(probe(&view).unwrap(), 8);
            let mut raw = view.facts().clone();
            raw.build
                .frontend_options
                .insert_data(view.generated_source_capture().unwrap().unwrap());
            assert_eq!(probe(&raw).unwrap(), 8, "standalone positive control");
            attacks::member(&raw);
            attacks::source(&raw);
            assert!(store.pair(index).is_none());
        }
    }
}
