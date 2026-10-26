//! Complete generated TryFrom maps retain independent source, cleanup and callee proof.
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
fn generated_conversion_source_proofs_connect_complete_selected_callee_chain() {
    let directory = clone_workspace();
    let source = fixture_source();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let first = function(
        &report,
        "<model::First as core::convert::TryFrom<u16>>::try_from",
    );
    attacks::selected_body(first);
    let (store, _) = data_unit(first);
    let analysis = store.analysis();
    for owner in ["First", "Second"] {
        for input in ["u16", "u32", "u64", "usize"] {
            let name = format!("<model::{owner} as core::convert::TryFrom<{input}>>::try_from");
            let entry = function(&report, &name);
            assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
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
            attacks::calls(&raw);
            verify_pair(store.pair(index).unwrap(), &view);
        }
    }
}

fn fixture_source() -> String {
    let source = super::unsigned_index_conversions::INPUTS;
    let start = source.find("impl TryFrom<u16> for Index").unwrap();
    let helpers = source[..start].replace("trait U", "pub trait U");
    let body = super::checked_index_construction::SOURCE.replace("#![no_std]", "");
    format!(
        "#![no_std]\nmod int {{ {helpers} }}\nmod model {{ use crate::int::{{U16,U32,U64}};\n{body}\n{}\n{WRAPPERS} }}",
        &source[start..]
    )
}

fn verify_pair(pair: &rstyle_pdg::producer::Pair, facts: &dyn EvidenceFacts) {
    rstyle_pdg::producer::verify(pair, facts).unwrap();
    rstyle_ccgraph::admit_produced(pair, facts).unwrap();
    let mut wire = serde_json::to_value(pair).unwrap();
    for graph in ["pdg", "cfg"] {
        wire[graph]["profile"]["version"] = "3.117.0".into();
    }
    let historical: rstyle_pdg::producer::Pair = serde_json::from_value(wire).unwrap();
    assert!(rstyle_ccgraph::admit_produced(&historical, facts).is_err());
}

#[test]
fn generated_conversions_keep_unproved_neighbor_macro_barriers() {
    let directory = clone_workspace();
    let source = format!(
        "{}\n{}\n{WRAPPERS}",
        super::checked_index_construction::SOURCE,
        super::unsigned_index_conversions::INPUTS
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success());
    for owner in ["First", "Second"] {
        for input in ["u16", "u32", "u64"] {
            let name = format!("<{owner} as core::convert::TryFrom<{input}>>::try_from");
            let entry = function(&report, &name);
            assert_eq!(entry["outcome"], "excluded");
            assert!(entry["reasons"].as_array().unwrap().iter().any(|reason| {
                reason
                    .as_str()
                    .unwrap()
                    .contains("Unit helper is unsupported")
            }));
        }
    }
}
