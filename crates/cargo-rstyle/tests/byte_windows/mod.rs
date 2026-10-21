//! Connected real-Cargo conversion, failure and endian semantics.
use super::*;

mod architecture;

#[test]
fn copied_byte_windows_compose_through_strict_admission_and_matching() {
    let directory = workspace();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 16, "{report}");
    for function in functions {
        let positive = matches!(
            function["facts"]["name"].as_str().unwrap(),
            "batch"
                | "same"
                | "saved"
                | "choose"
                | "helper_base"
                | "locals"
                | "wrong_window"
                | "wrong_length"
                | "snapshot"
                | "decode"
                | "decoded"
                | "caller"
        );
        assert_eq!(
            function["outcome"],
            if positive { "admitted" } else { "excluded" },
            "{function}"
        );
    }
    shared_evidence::check_partial_helper(function_named(functions, "caller"));
    check_aggregate_documents(&report);
    check_equivalent_matching(&report, ["batch", "same"]);
    let batch = functions
        .iter()
        .find(|function| function["facts"]["name"] == "batch")
        .unwrap();
    check_semantics(batch);
    check_corruption(batch);
    check_authority_corruption(batch);
    check_source_endian(batch);
    check_reference_snapshots(&report);
}

fn check_source_endian(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let mut changed = facts.clone();
    changed.source.bytes = changed
        .source
        .bytes
        .replace("from_ne_bytes", "from_be_bytes")
        .into();
    changed.source.digest = rstyle_pdg::concrete_types::digest(changed.source.bytes.as_bytes());
    let call = &changed.body.children[0];

    assert!(rstyle_pdg::call_summaries::resolve_for(call, &changed).is_err());
    shared_evidence::reject_facts(function, &changed);
}

fn workspace() -> TempDir {
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), r#"
        pub fn batch(input: &[u8], n: usize) -> u32 { u32::from_ne_bytes(input[n..n+4].try_into().unwrap()) }
        pub fn same(input: &[u8], n: usize) -> u32 { u32::from_ne_bytes(input[n..n+4].try_into().unwrap()) }
        pub fn saved(input: &[u8], n: usize) -> u32 { let view = &input[n..n+4]; u32::from_ne_bytes(view.try_into().unwrap()) }
        pub fn choose<'a>(a: &'a [u8], b: &'a [u8], pick: bool) -> &'a [u8] { if pick { a } else { b } }
        pub fn helper_base(a: &[u8], b: &[u8], pick: bool, n: usize) -> u32 { u32::from_ne_bytes(choose(a,b,pick)[n..n+4].try_into().unwrap()) }
        pub fn locals(input: &[u8], n: usize) -> u32 { let result: Result<[u8;4], _> = input[n..n+4].try_into(); let array = result.unwrap(); u32::from_ne_bytes(array) }
        pub fn wrong_window(input: &[u8], n: usize) -> u32 { u32::from_ne_bytes(input[n+1..n+5].try_into().unwrap()) }
        pub fn wrong_length(input: &[u8], n: usize) -> u32 { u32::from_ne_bytes(input[n..n+3].try_into().unwrap()) }
        pub fn snapshot<'a>(a: &'a [u8], b: &'a [u8]) -> u32 { let mut selected = a; u32::from_ne_bytes(selected[{selected=b;0}..4].try_into().unwrap()) }
        pub fn endian(input: &[u8], n: usize) -> u32 { u32::from_be_bytes(input[n..n+4].try_into().unwrap()) }
        pub fn wider(input: &[u8], n: usize) -> u64 { u64::from_ne_bytes(input[n..n+8].try_into().unwrap()) }
        pub fn borrowed(input: &[u8]) -> u32 { let array: &[u8;4] = input.try_into().unwrap(); u32::from_ne_bytes(*array) }
        pub fn caller(input: &[u8], n: usize) -> u32 { batch(input,n) }
        pub fn generic<T: TryInto<[u8;4]>>(input: T) -> [u8;4] where T::Error: std::fmt::Debug { input.try_into().unwrap() }
        pub fn decode(bytes: [u8;4]) -> u32 { u32::from_ne_bytes(bytes) }
        pub fn decoded(input: &[u8], n: usize) -> u32 { decode(input[n..n+4].try_into().unwrap()) }
    "#).unwrap();
    directory
}

fn check_reference_snapshots(report: &Value) {
    let functions = report["functions"].as_array().unwrap();
    let (unit, _) = data_unit(&functions[0]);
    for (name, expected) in [("snapshot", vec![0]), ("helper_base", vec![0, 1])] {
        let function = functions
            .iter()
            .find(|function| function["facts"]["name"] == name)
            .unwrap();
        let pair = shared_evidence::pair(&unit, function);
        let roots: Vec<_> = pair
            .pdg
            .memory_locations
            .iter()
            .map(|location| {
                location.extensions[rstyle_pdg::data_documents::LOCATION_KEY]["parameter"]
                    .as_u64()
                    .unwrap()
            })
            .collect();
        assert_eq!(roots, expected, "{name}");
    }
    let generic = functions
        .iter()
        .find(|function| function["facts"]["name"] == "generic")
        .unwrap();
    let conversion = &generic["facts"]["body"]["children"][0]["children"][0];
    assert_eq!(
        conversion["dispatch"]["op"],
        "std::convert::TryInto::try_into"
    );
    assert_eq!(conversion["res"]["kind"], "resolved_callee");
}

fn check_semantics(function: &Value) {
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
    assert_eq!(call("data.bytes.copy-result").attributes.reads.len(), 1);
    assert!(call("data.bytes.unwrap").attributes.reads.is_empty());
    assert!(call("data.bytes.native-u32").attributes.reads.is_empty());
    assert_eq!(pair.pdg.memory_locations.len(), 1);
    assert_eq!(
        pair.pdg.memory_locations[0].abstraction,
        "rstyle.formal-byte-window/1"
    );
    assert!(
        pair.pdg
            .extraction_evidence
            .diagnostics
            .iter()
            .any(|reason| reason.message.contains("unwrap"))
    );
    assert!(
        pair.pdg
            .extraction_evidence
            .diagnostics
            .iter()
            .any(|reason| reason.message.contains("+"))
    );
    rstyle_pdg::data_documents::verify(pair, &facts).unwrap();
    rstyle_ccgraph::admit_produced(pair, &facts).unwrap();
}

fn check_corruption(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    for mutation in ["read", "endian", "failure", "operand", "build", "summary"] {
        let mut pair = rstyle_pdg::producer::produce(&facts).unwrap();
        corrupt(&mut pair, mutation);
        assert!(
            rstyle_pdg::data_documents::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
        assert!(
            rstyle_pdg::producer::verify(&pair, &facts).is_err(),
            "{mutation}"
        );
    }
}

fn corrupt(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    if matches!(mutation, "build" | "summary") {
        return corrupt_summary(pair, mutation);
    }
    let construct = match mutation {
        "endian" => "data.bytes.native-u32",
        "failure" => "data.bytes.unwrap",
        _ => "data.bytes.copy-result",
    };
    let node = pair
        .pdg
        .nodes
        .iter_mut()
        .find(|node| node.attributes.construct.as_deref() == Some(construct))
        .unwrap();
    match mutation {
        "read" => node.attributes.reads.clear(),
        "endian" => {
            node.annotations.get_mut("rstyle.data:byte-call").unwrap()["endian"] = "big".into()
        }
        "failure" => {
            node.annotations.remove("rstyle.data:byte-call");
        }
        _ => {
            pair.pdg
                .edges
                .iter_mut()
                .find(|edge| edge.target == node.id && edge.subkind == "value_operand")
                .unwrap()
                .operand_position = Some(1)
        }
    }
}

fn corrupt_summary(pair: &mut rstyle_pdg::producer::Pair, mutation: &str) {
    let records = pair
        .pdg
        .extensions
        .get_mut(rstyle_pdg::call_summaries::KEY)
        .unwrap();
    let record = records
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|record| record["summary"]["id"] == "rstyle.data.bytes.unwrap")
        .unwrap();
    if mutation == "build" {
        record["panic_strategy"] = Value::Null;
    } else {
        record["summary"]["may_panic"] = false.into();
    }

    pair.cfg
        .extensions
        .insert(rstyle_pdg::call_summaries::KEY.into(), records.clone());
}

fn check_authority_corruption(function: &Value) {
    for field in ["definition_hash", "implementation"] {
        shared_evidence::reject_edit(function, |bundle, index| {
            let call = &mut bundle["functions"][index]["body"]["children"][0]["children"][0]["children"]
                [0];
            assert_eq!(call["kind"], "expr_method_call");
            let mut callee: Value =
                serde_json::from_str(call["res"]["description"].as_str().unwrap()).unwrap();
            if field == "implementation" {
                callee[field]["definition_hash"] = "invented".into();
            } else {
                callee[field] = "invented".into();
            }
            call["res"]["description"] = callee.to_string().into();
        });
    }
}
