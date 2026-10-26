//! Whole named errors compose through checked arithmetic and exact helper origins.
use super::{float_kernels::function, *};
mod attacks;
mod library_macros;
mod owned_composition;
mod product_composition;
mod reader_composition;
mod reader_observations;
mod validator_composition;
use rstyle_pdg::concrete_types::{InterfaceObservations, Observation, TypeTree, VariantShape};

const SOURCE: &str = r#"
#![no_std]
mod generated {
pub struct Attempt { attempted: u64 }
macro_rules! errors { ($name:ident) => { pub struct $name(Attempt); }; }
errors!(FirstError);
errors!(SecondError);
}
mod api {
    pub use super::generated::FirstError as First;
    pub use super::generated::SecondError as Second;
}
use api::{First as FirstError, Second as SecondError};
pub enum ErrorKind {
    Generic { msg: &'static str },
    BufferTooSmall { what: &'static str },
    InvalidUsize { what: &'static str },
    VersionMismatch { expected: u32, found: u32 },
    EndianMismatch { expected: u32, found: u32 },
    AlignmentMismatch { alignment: usize, address: usize },
    LabelMismatch { expected: &'static str },
    ArithmeticOverflow { what: &'static str },
    First { err: FirstError, what: &'static str },
    Second { err: SecondError, what: &'static str },
}
pub struct Error(ErrorKind);
pub fn arithmetic_overflow(what: &'static str) -> Error {
    Error(ErrorKind::ArithmeticOverflow { what })
}
pub fn add(
    a: usize,
    b: usize,
    what: &'static str,
) -> Result<usize, Error> {
    match a.checked_add(b) {
        Some(c) => Ok(c),
        None => Err(arithmetic_overflow(what)),
    }
}
pub fn mul(a: usize, b: usize, what: &'static str) -> Result<usize, Error> {
    match a.checked_mul(b) {
        Some(c) => Ok(c),
        None => Err(arithmetic_overflow(what)),
    }
}
pub enum ScalarOnly { First { value: usize }, Second { value: u8 } }
pub struct Borrowed { inner: ScalarOnly }
pub fn scalar(value: &Borrowed) -> bool { false }
pub struct Dropping;
impl Drop for Dropping { fn drop(&mut self) {} }
pub enum HiddenDrop { First { payload: Dropping }, Second { value: usize } }
pub fn hidden_drop(value: HiddenDrop) -> usize { 0 }
pub enum HiddenMemory { First { value: &'static [u8] }, Second { value: usize } }
pub fn hidden_memory(value: HiddenMemory) -> usize { 0 }
"#;

#[test]
fn named_error_payloads_compose_checked_arithmetic_without_omitting_cleanup() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for name in ["arithmetic_overflow", "add", "mul"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{name}: {}", entry["reasons"]);
        let (store, index) = shared_evidence::load(entry);
        let interface = store.functions()[index]
            .build
            .frontend_options
            .data::<InterfaceObservations>()
            .unwrap();
        interface.validate().unwrap();
        let Observation::Captured { ty } = &interface.result else {
            panic!(
                "Missing complete error observation for {name}: {:?}",
                interface.result
            );
        };
        verify_complete_pair(&store, index);
        attacks::reject(entry);
        check_error(ty);
        if name == "arithmetic_overflow" {
            check_generated_sources(&store, index, ty);
        }
        check_canonical_shape(ty);
        check_inventory_mutations(ty);
    }
    check_negative_storage(&report);
}

fn check_inventory_mutations(ty: &TypeTree) {
    for mutation in 0..6 {
        let mut changed = ty.clone();
        let error = match &mut changed {
            TypeTree::Result { err, .. } => err.as_mut(),
            error => error,
        };
        let TypeTree::Struct { fields, .. } = error else {
            unreachable!()
        };
        let TypeTree::MixedEnum { variants, .. } = &mut fields[0].ty else {
            unreachable!()
        };
        match mutation {
            0 => {
                variants.pop();
            }
            1 => variants.swap(0, 1),
            2 => variants[9].name = variants[8].name.clone(),
            _ => {
                let VariantShape::Named { fields } = &mut variants[3].shape else {
                    unreachable!()
                };
                match mutation {
                    3 => {
                        fields.pop();
                    }
                    4 => fields.swap(0, 1),
                    _ => fields[0].1.ty = TypeTree::Scalar("bool".into()),
                }
            }
        }
        assert!(
            rstyle_pdg::concrete_types::validate(&changed).is_err(),
            "mutation {mutation}"
        );
    }
}

fn check_error(result: &TypeTree) {
    let error = match result {
        TypeTree::Result { ok, err } => {
            assert_eq!(ok.as_ref(), &TypeTree::Scalar("usize".into()));
            err.as_ref()
        }
        error => error,
    };
    let TypeTree::Struct { fields, .. } = error else {
        panic!("Missing error wrapper")
    };
    assert_eq!(fields.len(), 1);
    let TypeTree::MixedEnum { variants, .. } = &fields[0].ty else {
        panic!("Missing complete enum")
    };
    assert_eq!(variants.len(), 10);
    assert_eq!(
        variants
            .iter()
            .map(|v| v.shape.fields().count())
            .sum::<usize>(),
        15
    );
    for (position, name) in [(8, "First"), (9, "Second")] {
        assert_eq!(variants[position].name, name);
        let VariantShape::Named { fields } = &variants[position].shape else {
            panic!("Missing named payload")
        };
        assert!(matches!(fields[0].1.ty, TypeTree::GeneratedStruct { .. }));
        assert_eq!(fields[1].1.ty, TypeTree::Shared(Box::new(TypeTree::Str)));
    }
}

fn check_canonical_shape(ty: &TypeTree) {
    let text = rstyle_pdg::data_types::token(ty).unwrap();
    rstyle_pdg::data_types::parse(&text).unwrap();
}

fn check_negative_storage(report: &Value) {
    for name in ["scalar", "hidden_drop", "hidden_memory"] {
        let entry = function(report, name);
        assert_eq!(entry["outcome"], "excluded");
        let (store, index) = shared_evidence::load(entry);
        let interface = store.functions()[index]
            .build
            .frontend_options
            .data::<InterfaceObservations>()
            .unwrap();
        match (&interface.parameters[0], name) {
            (Observation::Captured { ty }, "scalar") => check_canonical_shape(ty),
            (Observation::Unsupported { .. }, "hidden_drop" | "hidden_memory") => {}
            other => panic!("Unexpected storage observation: {other:?}"),
        }
    }
}

fn check_generated_sources(
    store: &rstyle_pdg::unit_store::UnitStore,
    index: usize,
    tree: &TypeTree,
) {
    let TypeTree::Struct { fields, .. } = tree else {
        panic!("Expected error owner")
    };
    let TypeTree::MixedEnum { variants, .. } = &fields[0].ty else {
        panic!("Expected complete error")
    };
    let trees: Vec<_> = variants[8..]
        .iter()
        .map(|variant| {
            let VariantShape::Named { fields } = &variant.shape else {
                panic!("Expected named payload")
            };
            &fields[0].1.ty
        })
        .collect();
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    assert_eq!(
        rstyle_pdg::nominal_boundary::resolution::probe_named_error_cleanup(&facts).unwrap(),
        5
    );
    assert_eq!(
        rstyle_pdg::nominal_boundary::resolution::probe_generated_error_imports(&facts).unwrap(),
        2
    );
    assert_eq!(
        rstyle_pdg::nominal_boundary::resolution::probe_generated_declaration_sources(
            &facts, &trees
        )
        .unwrap(),
        2
    );
}

fn verify_complete_pair(store: &rstyle_pdg::unit_store::UnitStore, index: usize) {
    let analysis = store.analysis();
    let facts = analysis.function(index).unwrap();
    let original = store.pair(index).unwrap();
    rstyle_pdg::producer::verify(original, &facts).unwrap();
    rstyle_ccgraph::admit_produced(original, &facts).unwrap();
    let mut pair = rstyle_pdg::producer::Pair {
        pdg: original.pdg.clone(),
        cfg: original.cfg.clone(),
    };
    pair.pdg.profile.version = "3.118.0".into();
    pair.cfg.profile.version = "3.118.0".into();
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}

#[test]
fn named_error_cleanup_rejects_drop_in_an_unused_generated_variant() {
    for destructor in [
        "impl Drop for FirstError { fn drop(&mut self) {} }",
        "mod hidden { use crate::FirstError as Alias; impl Drop for Alias { fn drop(&mut self) {} } }",
    ] {
        let directory = clone_workspace();
        std::fs::write(
            directory.path().join("src/lib.rs"),
            format!("{SOURCE}\n{destructor}"),
        )
        .unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        for name in ["arithmetic_overflow", "add", "mul"] {
            let entry = function(&report, name);
            assert_eq!(entry["outcome"], "excluded", "{name}: {destructor}");
        }
    }
}
