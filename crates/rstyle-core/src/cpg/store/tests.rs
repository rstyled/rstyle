use super::*;

/// A bundled unit document over one file of `content`.
fn document(functions: Vec<Value>, types: Vec<Value>, content: &str) -> Value {
    json!({
        "schema_version": "0.1.0",
        "profile": {"id": "compat.cpg", "version": "0.1.0"},
        "producer": {"name": "test", "version": "0"},
        "coverage": {},
        "layout": "bundled",
        "document": "unit",
        "unit": {
            "id": "unit:t",
            "files": [{"id": "file:a.rs", "path": "a.rs", "origin": "source", "content": content}],
            "definitions": [{"id": "def:t::a", "kind": "function", "span": {"file": "file:a.rs", "start": 0, "end": 1}}],
            "types": types,
            "functions": functions,
        },
    })
}

/// A function written over `start..end` of `a.rs`, whose one node has type `ty`.
fn function(id: &str, start: u64, end: u64, ty: &str) -> Value {
    json!({
        "id": id,
        "definition": "def:t::a",
        "span": {"file": "file:a.rs", "start": start, "end": end},
        "signature": {"span": {"file": "file:a.rs", "start": start, "end": end}},
        "nodes": [{"id": "n1", "type": ty, "span": {"file": "file:a.rs", "start": start + 1, "end": end - 1}}],
    })
}

fn types() -> Vec<Value> {
    vec![
        json!({"id": "type:&Holder", "kind": "reference", "target": "type:Holder"}),
        json!({"id": "type:Holder", "kind": "record", "fields": [{"name": "name", "type": "type:u8"}]}),
        json!({"id": "type:u8", "kind": "integer"}),
        json!({"id": "type:u16", "kind": "integer"}),
    ]
}

fn entry(pack: &[u8], position: usize) -> Value {
    index(pack).unwrap()["functions"][position].clone()
}

fn packed(document: &Value) -> Vec<u8> {
    pack(document, 3).unwrap().bytes
}

#[test]
fn a_pack_gives_back_the_document_it_was_made_from() {
    let original = document(
        vec![
            function("fn:a", 10, 20, "type:&Holder"),
            function("fn:b", 30, 40, "type:u16"),
        ],
        types(),
        &"x".repeat(50),
    );
    let bytes = packed(&original);
    assert_eq!(unpack(&bytes).unwrap(), original);
}

#[test]
fn a_pack_writes_its_document_out_one_function_at_a_time() {
    let original = document(
        vec![
            function("fn:a", 10, 20, "type:&Holder"),
            function("fn:b", 30, 40, "type:u16"),
        ],
        types(),
        &"x".repeat(50),
    );
    let bytes = packed(&original);
    let mut written = Vec::new();
    write_unpacked(&bytes, &mut written).unwrap();
    assert_eq!(String::from_utf8(written).unwrap(), canonical(&original));
}

#[test]
fn each_function_decodes_from_its_own_frame() {
    let original = document(
        vec![
            function("fn:a", 10, 20, "type:u8"),
            function("fn:b", 30, 40, "type:u16"),
        ],
        types(),
        &"x".repeat(50),
    );
    let bytes = packed(&original);
    for (position, expected) in original["unit"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let entry = entry(&bytes, position);
        assert_eq!(entry["locator"], expected["id"]);
        let text = frame(&bytes, place(&entry["frame"]).unwrap()).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), *expected);
    }
}

#[test]
fn a_function_that_only_moved_keeps_its_shape() {
    let here = packed(&document(
        vec![function("fn:a", 10, 20, "type:u8")],
        types(),
        &"x".repeat(50),
    ));
    let moved = packed(&document(
        vec![function("fn:a", 25, 35, "type:u8")],
        types(),
        &"x".repeat(50),
    ));
    let (here, moved) = (entry(&here, 0), entry(&moved, 0));
    assert_ne!(here["digest"], moved["digest"]);
    assert_eq!(here["shape"], moved["shape"]);
}

#[test]
fn reads_change_with_a_type_reached_through_another() {
    let reads = |types: Vec<Value>| -> Value {
        entry(
            &packed(&document(
                vec![function("fn:a", 10, 20, "type:&Holder")],
                types,
                &"x".repeat(50),
            )),
            0,
        )["reads"]
            .clone()
    };
    let before = reads(types());
    let mut grown = types();
    grown[1]["fields"] =
        json!([{"name": "name", "type": "type:u8"}, {"name": "id", "type": "type:u16"}]);
    assert_ne!(
        before,
        reads(grown),
        "the record behind the reference gained a field"
    );
    let mut unrelated = types();
    unrelated.push(json!({"id": "type:u32", "kind": "integer"}));
    assert_eq!(before, reads(unrelated));
    let spanned = |start: u64| {
        let mut types = types();
        types[2]["span"] = json!({"file": "file:a.rs", "start": start, "end": start + 1});
        types
    };
    assert_eq!(reads(spanned(3)), reads(spanned(7)), "spans aren't read");
}

#[test]
fn context_changes_with_the_comment_line_above_the_signature() {
    let content = |above: &str| format!("{above}\nfn a() {{}}\n");
    let keys = |above: &str| -> Value {
        let text = content(above);
        let start = text.find("fn a").unwrap() as u64;
        entry(
            &packed(&document(
                vec![function("fn:a", start, start + 9, "type:u8")],
                types(),
                &text,
            )),
            0,
        )
    };
    let plain = keys("// a note");
    let allowed = keys("// rstyle-xx(rule): a reason");
    assert_ne!(plain["context"], allowed["context"]);
    assert_eq!(plain["shape"], allowed["shape"]);
    assert_eq!(plain["reads"], allowed["reads"]);
}

#[test]
fn documents_of_one_unit_merge_into_one() {
    let first = document(
        vec![function("fn:a", 10, 20, "type:u8")],
        types()[..3].to_vec(),
        &"x".repeat(50),
    );
    let second = document(
        vec![function("fn:b", 30, 40, "type:u16")],
        types()[2..].to_vec(),
        &"x".repeat(50),
    );
    let merged = merge_unit(vec![first, second]).unwrap();
    let ids = |table: &str| -> Vec<String> {
        merged["unit"][table]
            .as_array()
            .unwrap()
            .iter()
            .map(entry_key)
            .collect()
    };
    assert_eq!(ids("functions"), ["fn:a", "fn:b"]);
    assert_eq!(
        ids("types"),
        ["type:&Holder", "type:Holder", "type:u8", "type:u16"]
    );
}

#[test]
fn documents_that_disagree_on_an_entry_are_not_merged() {
    let first = document(vec![], types(), "");
    let mut differing = types();
    differing[2]["modifiers"] = json!(["rust:copy"]);
    let second = document(vec![], differing, "");
    let error = merge_unit(vec![first, second]).unwrap_err();
    assert!(error.contains("types entry type:u8"), "{error}");
}

#[test]
fn bytes_that_are_not_a_pack_are_refused() {
    assert!(index(b"not a pack at all, though long enough to have a footer").is_err());
    let mut bytes = packed(&document(vec![], types(), ""));
    let end = bytes.len() - 1;
    bytes[end] ^= 1;
    assert!(index(&bytes).is_err());
}

#[test]
fn objects_are_named_by_their_bytes_and_checked_when_read() {
    let directory = tempfile::tempdir().unwrap();
    let objects = Objects::new(directory.path());
    let name = objects.put(b"a pack").unwrap();
    assert_eq!(name, sha256(b"a pack"));
    assert_eq!(
        objects.put(b"a pack").unwrap(),
        name,
        "an object is kept once"
    );
    assert_eq!(objects.get(&name).unwrap(), b"a pack");
    std::fs::write(objects.path(&name).unwrap(), b"another").unwrap();
    assert!(objects.get(&name).unwrap_err().contains("no longer match"));
    assert!(objects.get("../outside").is_err());
}

#[test]
fn serde_json_writes_canonical_json() {
    // Members inserted out of order, nested, with escapes and numbers: what `pack` writes
    // through serde_json must be what `layout::canonical` writes.
    let value: Value =
        serde_json::from_str(r#"{"z": 1, "a": {"y": [3, 1.5, "é\n"], "b": null}, "m": true}"#)
            .unwrap();
    assert_eq!(
        String::from_utf8(canonical_bytes(&value)).unwrap(),
        canonical(&value)
    );
    let original = document(vec![function("fn:a", 10, 20, "type:&Holder")], types(), "x");
    assert_eq!(
        String::from_utf8(canonical_bytes(&original)).unwrap(),
        canonical(&original)
    );
}
