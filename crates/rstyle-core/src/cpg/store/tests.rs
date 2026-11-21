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
    assert!(objects.has(&name));
    assert!(!objects.has(&sha256(b"never kept")));
    assert!(!objects.has("not a name"));
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

#[test]
fn a_table_only_a_later_document_holds_is_kept() {
    let mut first = document(vec![function("fn:a", 10, 20, "type:u8")], types(), "");
    first["unit"].as_object_mut().unwrap().remove("types");
    let second = document(vec![], types(), "");
    let merged = merge_unit(vec![first, second]).unwrap();
    assert_eq!(
        merged["unit"]["types"].as_array().unwrap().len(),
        types().len()
    );
}

#[test]
fn documents_of_two_units_are_not_merged() {
    let first = document(vec![], types(), "");
    let mut second = document(vec![], types(), "");
    second["unit"]["id"] = json!("unit:other");
    assert!(
        merge_unit(vec![first, second])
            .unwrap_err()
            .contains("different units")
    );
    assert!(merge_unit(Vec::new()).is_err());
}

#[test]
fn a_pack_of_another_format_is_refused() {
    let mut bytes = packed(&document(vec![], types(), ""));
    bytes[PACK_MAGIC.len()] = 9;
    assert!(index(&bytes).unwrap_err().contains("format 9"));
    let mut written = Vec::new();
    assert!(write_unpacked(&bytes, &mut written).is_err());
    assert!(unpack(&bytes).is_err());
}

#[test]
fn a_frame_is_named_by_its_offset_and_length() {
    assert_eq!(place(&json!([3, 4])).unwrap(), [3, 4]);
    assert!(place(&json!(["3", 4])).is_err());
    assert!(frame(b"short", [2, 100]).is_err());
}

#[test]
fn a_unit_document_that_is_not_an_object_has_no_tables() {
    assert!(tables(&json!("a unit")).is_empty());
    assert!(pack(&json!({"unit": {"id": "unit:t"}}), 3).is_err());
}

#[test]
fn context_without_a_signature_or_a_listed_file_still_reads() {
    let mut bare = function("fn:a", 10, 20, "type:u8");
    bare.as_object_mut().unwrap().remove("signature");
    let mut elsewhere = bare.clone();
    elsewhere["span"]["file"] = json!("file:unlisted.rs");
    let keys = entry(
        &packed(&document(vec![bare, elsewhere], types(), &"x".repeat(50))),
        0,
    );
    let other = entry(
        &packed(&document(
            vec![function("fn:a", 10, 20, "type:u8")],
            types(),
            &"x".repeat(50),
        )),
        0,
    );
    assert_eq!(
        keys["context"], other["context"],
        "the function's own span stands in for its signature's"
    );
    let unlisted = index(&packed(&document(vec![], types(), ""))).unwrap();
    assert_eq!(unlisted["functions"], json!([]));
}

// The copies the digests used to be taken of, which reading spans while serializing has to
// match byte for byte.
/// The value with every span left out.
fn unspanned(value: &Value) -> Value {
    match value {
        Value::Object(fields) if is_span(fields) => Value::Null,
        Value::Object(fields) => fields
            .iter()
            .map(|(key, item)| (key.clone(), unspanned(item)))
            .collect::<serde_json::Map<_, _>>()
            .into(),
        Value::Array(items) => items.iter().map(unspanned).collect(),
        scalar => scalar.clone(),
    }
}

/// The value with each span in `file` made relative to `base`.
fn rebased(value: &Value, file: &str, base: u64) -> Value {
    match value {
        Value::Object(fields) if is_span(fields) && fields["file"] == file => {
            let mut span = fields.clone();
            for key in ["start", "end"] {
                if let Some(offset) = span[key].as_u64() {
                    span[key] = json!(offset as i64 - base as i64);
                }
            }
            span.into()
        }
        Value::Object(fields) => fields
            .iter()
            .map(|(key, item)| (key.clone(), rebased(item, file, base)))
            .collect::<serde_json::Map<_, _>>()
            .into(),
        Value::Array(items) => items.iter().map(|item| rebased(item, file, base)).collect(),
        scalar => scalar.clone(),
    }
}

#[test]
fn digests_read_spans_as_rewritten_copies_of_the_value_would() {
    let value = json!({
        "id": "fn:a",
        "span": {"file": "file:a.rs", "start": 12, "end": 30},
        "other": {"file": "file:b.rs", "start": 12, "end": 30},
        "named": {"file": "file:a.rs", "start": "here", "end": null},
        "wide": {"file": "file:a.rs", "start": 3, "end": 4, "line": 1, "column": 2},
        "nodes": [
            {"id": "n1", "span": {"file": "file:a.rs", "start": 14, "end": 15, "line": 2}},
            [1.5, -2, true, null, "é\n"],
        ],
    });
    for base in [0, 10, 40] {
        assert_eq!(
            read_digest(&value, Spans::From("file:a.rs", base)),
            digest(&rebased(&value, "file:a.rs", base))
        );
    }
    assert_eq!(
        read_digest(&value, Spans::Dropped),
        digest(&unspanned(&value))
    );
    let span = json!({"file": "file:a.rs", "start": "here", "end": null});
    assert_eq!(rebased(&span, "file:a.rs", 10), span);
}

#[test]
fn the_comment_line_is_found_from_where_lines_start() {
    let mut content = String::from("// é first\n#[attr]\nfn a() {}\r\n");
    for line in 0..45 {
        content.push_str(&format!("// note {line}\nfn f{line}() {{}}\n"));
    }
    let starts = line_starts(&content);
    for start in 0..content.len() + 3 {
        let before = content.get(..start).unwrap_or(&content);
        let line = 1 + before.matches('\n').count();
        assert_eq!(
            comment_line_at(&content, &starts, start),
            comment_line_above(&content, line),
            "at byte {start}"
        );
    }
}

#[test]
fn sha256_names_bytes_in_lowercase_hex() {
    assert_eq!(
        sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn a_document_without_a_unit_is_not_merged() {
    let first = document(vec![], types(), "");
    for broken in [json!({"unit": "unit:t"}), json!("a document")] {
        let error = merge_unit(vec![first.clone(), broken.clone()]).unwrap_err();
        assert!(error.contains("holds its unit"), "{error}");
        assert!(merge_unit(vec![broken]).is_err());
    }
}

#[test]
fn a_span_past_i64_s_range_is_rebased_without_wrapping() {
    let read = |start: u64, end: u64, base: u64| {
        let value = json!({"span": {"file": "file:a.rs", "start": start, "end": end}});
        serde_json::to_string(&Read(&value, Spans::From("file:a.rs", base))).unwrap()
    };
    let past = i64::MAX as u64 + 1;
    assert_eq!(
        read(0, past, 0),
        format!(r#"{{"span":{{"end":{past},"file":"file:a.rs","start":0}}}}"#)
    );
    assert_eq!(
        read(0, 1, u64::MAX),
        format!(
            r#"{{"span":{{"end":-{},"file":"file:a.rs","start":-{}}}}}"#,
            u64::MAX - 1,
            u64::MAX
        )
    );
}
