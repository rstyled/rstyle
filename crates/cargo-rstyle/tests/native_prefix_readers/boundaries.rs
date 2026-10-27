use super::*;

#[test]
fn scalar_sizes_keep_bounded_source_and_instantiation_requirements() {
    for (source, expected) in [
        (
            "pub fn size()->usize { core::mem::size_of::<u16>() }",
            "admitted",
        ),
        (
            "pub fn size()->usize { std::mem::size_of::<u128>() }",
            "admitted",
        ),
        (
            "pub fn size()->usize { core::mem::size_of::<u8>() }",
            "excluded",
        ),
        (
            "pub fn size()->usize { core::mem::size_of::<[u8;2]>() }",
            "excluded",
        ),
        (
            "type Word = u16; pub fn size()->usize { core::mem::size_of::<Word>() }",
            "excluded",
        ),
        (
            "pub fn size()->usize { fn size_of<T>() -> usize { 7 } size_of::<u16>() }",
            "excluded",
        ),
    ] {
        let directory = clone_workspace();
        std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
        let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
        assert!(output.status.success(), "{source}: {report}");
        let function = report["functions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|function| function["facts"]["name"] == "size")
            .unwrap();
        assert_eq!(function["outcome"], expected, "{source}: {function}");
        if expected == "admitted" {
            reject_policy_downgrade(function);
        }
    }
}

#[test]
fn native_u16_alone_requires_its_document_policy() {
    let (_directory, report) = captured(false, None);
    let function = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["facts"]["name"] == "native_u16")
        .unwrap();
    reject_policy_downgrade(function);
}

fn reject_policy_downgrade(function: &Value) {
    assert_eq!(function["outcome"], "admitted");
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
    pair.pdg.profile.version = "3.43.0".into();
    pair.cfg.profile.version = "3.43.0".into();
    assert!(rstyle_pdg::data_documents::verify(&pair, &facts).is_err());
    assert!(rstyle_pdg::producer::verify(&pair, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&pair, &facts).is_err());
}
