//! Expectations were selected from source before enabling the connected chain.
use super::*;

#[test]
fn architecture_readers_compose_through_strict_admission() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        include_str!("../../../../eval/coverage-development/arch-readers.rs"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 8, "{report}");
    let positive = [
        "batch_arch",
        "saved_arch",
        "result_local_arch",
        "snapshot_arch",
        "wrong_length_arch",
    ];
    let negative = ["endian_arch", "other_size_authority_arch", "caller_arch"];
    for name in positive.into_iter().chain(negative) {
        let function = functions
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        assert_eq!(
            function["outcome"],
            if positive.contains(&name) {
                "admitted"
            } else {
                "excluded"
            },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    let batch = functions
        .iter()
        .find(|function| function["facts"]["name"] == "batch_arch")
        .unwrap();
    semantics(batch);
    document_corruptions(batch);
    constant_corruptions(batch);
    let snapshot = functions
        .iter()
        .find(|function| function["facts"]["name"] == "snapshot_arch")
        .unwrap();
    saved_base(snapshot);
}

fn semantics(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = shared_evidence::pair(&unit, function);
    let call = |construct| {
        pair.pdg
            .nodes
            .iter()
            .find(|node| node.attributes.construct.as_deref() == Some(construct))
            .unwrap()
    };
    let borrow = call("data.bytes.borrow-result");
    assert!(borrow.attributes.reads.is_empty());
    assert_eq!(
        borrow.annotations["rstyle.data:byte-call"]["payload_origins_on"],
        "ok"
    );
    assert_eq!(
        borrow.annotations["rstyle.data:byte-call"]["content_read"],
        false
    );
    assert!(call("data.bytes.unwrap-borrow").attributes.reads.is_empty());
    let read = call("data.indirect-deref");
    assert_eq!(read.attributes.reads.len(), 1);
    assert_eq!(pair.pdg.memory_locations.len(), 1);
    assert_eq!(
        pair.pdg.memory_locations[0].abstraction,
        "rstyle.formal-byte-window/1"
    );
    assert_eq!(
        call("data.bytes.native-usize").annotations["rstyle.data:byte-call"]["byte_significance_bits"],
        serde_json::json!([0, 8, 16, 24, 32, 40, 48, 56])
    );
    rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    rstyle_ccgraph::admit_produced(pair, &facts).unwrap();
}

fn document_corruptions(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for change in [
        "missing-read",
        "invented-read",
        "failure",
        "origin",
        "endian",
    ] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        corrupt_document(&mut pair, change);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{change}"
        );
        assert!(
            rstyle_ccgraph::admit_produced(&pair, &facts).is_err(),
            "{change}"
        );
    }
}

fn corrupt_document(pair: &mut rstyle_pdg::producer::Pair, change: &str) {
    let read = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.attributes.construct.as_deref() == Some("data.indirect-deref"))
        .unwrap()
        .attributes
        .reads
        .clone();
    let construct = match change {
        "missing-read" => "data.indirect-deref",
        "failure" => "data.bytes.unwrap-borrow",
        "endian" => "data.bytes.native-usize",
        _ => "data.bytes.borrow-result",
    };
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some(construct))
        .unwrap();
    match change {
        "missing-read" => node.attributes.reads.clear(),
        "invented-read" => node.attributes.reads = read,
        "failure" => {
            node.annotations.get_mut("rstyle.data:byte-call").unwrap()["normal_completion_requires"] =
                "any".into()
        }
        "endian" => {
            node.annotations.get_mut("rstyle.data:byte-call").unwrap()["byte_significance_bits"]
                [0] = 56.into()
        }
        "origin" => {
            node.annotations
                .get_mut("rstyle.data:reference-origins")
                .unwrap()[0]["path"] = serde_json::json!([])
        }
        _ => unreachable!(),
    }
}

fn constant_corruptions(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for change in ["definition", "argument", "value", "initializer", "source"] {
        let mut changed = facts.clone();
        let node = &mut changed.body.children[0];
        let resolution = node.res.as_mut().unwrap();
        let mut record: Value = serde_json::from_str(&resolution.description).unwrap();
        match change {
            "definition" => record["initializer"]["definition_hash"] = "forged".into(),
            "argument" => record["argument"] = "u64".into(),
            "value" => record["constant"]["value"] = "7".into(),
            "initializer" => record["initializer"]["path"] = "custom::size_of".into(),
            "source" => {
                changed.source.bytes = changed
                    .source
                    .bytes
                    .replace("size_of::<usize>()", "size_of::< u64 >()")
                    .into();
                changed.source.digest =
                    rstyle_pdg::concrete_types::digest(changed.source.bytes.as_bytes());
            }
            _ => unreachable!(),
        }
        resolution.description = serde_json::to_string(&record).unwrap();
        assert!(rstyle_pdg::producer::produce(&changed).is_err(), "{change}");
        shared_evidence::reject_facts(function, &changed);
    }
    let mut changed = facts.clone();
    assert!(alter_size_use(&mut changed.body));
    assert!(rstyle_pdg::producer::produce(&changed).is_err());
    shared_evidence::reject_facts(function, &changed);

    let mut changed = facts.clone();
    let annotation = changed.body.children[1]
        .children
        .iter_mut()
        .find(|node| node.kind == "ty_annotation")
        .unwrap();
    let mut ty: Value =
        serde_json::from_str(annotation.ty.as_ref().unwrap().normalized.as_ref().unwrap()).unwrap();
    ty["value"]["value"]["length"] = 7.into();
    annotation.ty.as_mut().unwrap().normalized = Some(serde_json::to_string(&ty).unwrap());
    assert!(rstyle_pdg::producer::produce(&changed).is_err());
    shared_evidence::reject_facts(function, &changed);
}

fn alter_size_use(node: &mut rstyle_pdg::HirNode) -> bool {
    if let Some(resolution) = node
        .res
        .as_mut()
        .filter(|res| res.kind == rstyle_pdg::scalar_constants::RESOLUTION)
    {
        let mut constant: Value = serde_json::from_str(&resolution.description).unwrap();
        constant["value"] = "7".into();
        resolution.description = serde_json::to_string(&constant).unwrap();
        return true;
    }
    node.children.iter_mut().any(alter_size_use)
}

fn saved_base(function: &Value) {
    let (unit, _) = data_unit(function);
    let pair = shared_evidence::pair(&unit, function);
    assert_eq!(pair.pdg.memory_locations.len(), 1);
    assert_eq!(
        pair.pdg.memory_locations[0].extensions[rstyle_pdg::data_documents::LOCATION_KEY]["parameter"],
        0
    );
    let returned = pair
        .pdg
        .nodes
        .iter()
        .find(|node| node.category == rstyle_pdg::NodeCategory::Return)
        .unwrap();
    let origins = returned.annotations["rstyle.data:reference-origins"]
        .as_array()
        .unwrap();
    assert_eq!(origins.len(), 1);
    assert_eq!(origins[0]["parameter"], 1);
}

#[test]
fn architecture_views_compose_with_helpers_and_repeated_reads() {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), compositions()).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 8, "{report}");
    for function in functions {
        let positive = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "choose" | "helper_arch" | "repeated" | "decode" | "decoded"
        );
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{function}"
        );
    }
    check_aggregate_documents(&report);
    let repeated = functions
        .iter()
        .find(|function| function["facts"]["name"] == "repeated")
        .unwrap();
    repeated_reads(repeated);
    let helper = functions
        .iter()
        .find(|function| function["facts"]["name"] == "helper_arch")
        .unwrap();
    let (unit, _) = data_unit(helper);
    let roots: Vec<_> = shared_evidence::pair(&unit, helper)
        .pdg
        .memory_locations
        .iter()
        .map(|location| {
            location.extensions[rstyle_pdg::data_documents::LOCATION_KEY]["parameter"].clone()
        })
        .collect();
    assert_eq!(roots, vec![serde_json::json!(0), serde_json::json!(1)]);
}

fn repeated_reads(function: &Value) {
    let (unit, _) = data_unit(function);
    let pair = shared_evidence::pair(&unit, function);
    let reads: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some("data.indirect-deref"))
        .collect();
    assert_eq!(reads.len(), 2);
    assert_eq!(pair.pdg.memory_locations.len(), 1);
    assert_ne!(
        rstyle_pdg::data_documents::read_keys(reads[0], &pair.pdg.memory_locations),
        rstyle_pdg::data_documents::read_keys(reads[1], &pair.pdg.memory_locations)
    );
}

fn compositions() -> &'static str {
    r#"
pub fn choose<'a>(a: &'a [u8], b: &'a [u8], pick: bool) -> &'a [u8] { if pick {a} else {b} }
pub fn helper_arch(a: &[u8], b: &[u8], pick: bool, n: usize) -> usize {
    const WIDTH: usize = core::mem::size_of::<usize>();
    let array: &[u8;WIDTH] = choose(a,b,pick)[n..n+WIDTH].try_into().unwrap();
    usize::from_ne_bytes(*array)
}
pub fn repeated(input: &[u8]) -> (usize, usize) {
    let a: &[u8;8] = input[0..8].try_into().unwrap();
    let b: &[u8;8] = input[1..9].try_into().unwrap();
    (usize::from_ne_bytes(*a),usize::from_ne_bytes(*b))
}
pub fn decode(bytes: [u8;8]) -> usize { usize::from_ne_bytes(bytes) }
pub fn decoded(input: &[u8]) -> usize {
    let array: &[u8;8] = input[0..8].try_into().unwrap(); decode(*array)
}
pub fn nested(input: &[u8]) -> usize {
    { const WIDTH: usize = core::mem::size_of::<usize>();
      let array: &[u8;WIDTH] = input[0..WIDTH].try_into().unwrap(); usize::from_ne_bytes(*array) }
}
pub fn literal_constant(input: &[u8]) -> usize {
    const WIDTH: usize = 8;
    let array: &[u8;WIDTH] = input[0..WIDTH].try_into().unwrap(); usize::from_ne_bytes(*array)
}
pub fn length_expression(input: &[u8]) -> usize {
    const WIDTH: usize = core::mem::size_of::<usize>();
    let array: &[u8;WIDTH-0] = input[0..WIDTH].try_into().unwrap(); usize::from_ne_bytes(*array)
}
"#
}

#[test]
fn installed_ci_targets_preserve_reader_admission() {
    for target in [
        "aarch64-unknown-linux-gnu",
        "x86_64-unknown-linux-gnu",
        "aarch64-apple-darwin",
    ] {
        let output = std::process::Command::new("rustc")
            .args(["--print", "target-libdir", "--target", target])
            .output()
            .unwrap();
        assert!(output.status.success());
        let directory = String::from_utf8(output.stdout).unwrap();
        if !std::path::Path::new(directory.trim()).is_dir() {
            eprintln!("target std not installed: {target}");
            continue;
        }
        check_target_readers(target);
    }
}

fn check_target_readers(target: &str) {
    let directory = clone_workspace();
    let source = format!(
        "{}\n{}",
        include_str!("../../../../eval/coverage-development/arch-readers.rs"),
        r#"
        pub fn window(a: &[u8], lo: usize, hi: usize) -> &[u8] { &a[lo..hi] }
        pub fn snapshot<'a>(a: &'a [u8], b: &'a [u8]) -> &'a [u8] {
            let mut selected = a; &selected[{ selected = b; 0 }..1]
        }
        pub fn copied(a: &[u8], n: usize) -> u32 {
            u32::from_ne_bytes(a[n..n+4].try_into().unwrap())
        }
        pub fn slice_len(a: &[u8]) -> usize { a.len() }
        pub fn slice_empty(a: &[u8]) -> bool { a.is_empty() }
        pub fn str_len(a: &str) -> usize { a.len() }
        pub fn str_empty(a: &str) -> bool { a.is_empty() }
        "#
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(
        directory.path(),
        &["--target", target, "--extraction-profile", "hir-data-v3"],
    );
    assert!(output.status.success(), "{target}: {report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 15, "{target}: {report}");
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        let excluded = ["endian_arch", "other_size_authority_arch", "caller_arch"].contains(&name);
        assert_eq!(
            function["outcome"],
            if excluded { "excluded" } else { "admitted" },
            "{target}: {function}"
        );
    }
    check_aggregate_documents(&report);
    let batch = functions
        .iter()
        .find(|function| function["facts"]["name"] == "batch_arch")
        .unwrap();
    reject_mismatched_authorities(batch);
}

fn reject_mismatched_authorities(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for target in rstyle_pdg::data_library::registries()
        .iter()
        .map(|registry| registry.target.as_str())
        .chain(["unsupported"])
    {
        if facts.build.target_triple.known_value() == Some(target) {
            continue;
        }
        let mut changed = facts.clone();
        changed.build.target_triple = rstyle_pdg::StringFact::known(target);
        assert!(rstyle_pdg::producer::produce(&changed).is_err(), "{target}");
        shared_evidence::reject_facts(function, &changed);
    }
    let mut changed = facts.clone();
    changed.build.rustc_commit = rstyle_pdg::StringFact::known("unsupported");
    assert!(rstyle_pdg::producer::produce(&changed).is_err());
    shared_evidence::reject_facts(function, &changed);
}
