use super::*;
use crate::cpg::layout::{self, Parsed};
use serde_json::Value;

const ALL: Demand = Demand {
    graphs: true,
    bounds: true,
    flow: true,
    defects: true,
};

fn example() -> Value {
    serde_json::from_str(include_str!("../../testdata/example.cpg.json")).unwrap()
}

fn document(value: &Value) -> Document {
    layout::bundle(vec![value.clone()]).unwrap()
}

fn keys(value: &Value) -> Vec<Option<String>> {
    let document = document(value);
    let unit = document.unit.as_ref().unwrap();
    Keys::new(&document, unit, "scope").each(unit.functions.as_deref().unwrap())
}

fn texts(records: &[Record]) -> Vec<String> {
    records.iter().map(|record| format!("{record:?}")).collect()
}

/// The example's file with `from` written as `to`, which must be as long.
fn rewritten(value: &Value, from: &str, to: &str) -> Value {
    assert_eq!(from.len(), to.len());
    let mut value = value.clone();
    let content = value["unit"]["files"][0]["content"].as_str().unwrap();
    assert_eq!(content.matches(from).count(), 1, "{from}");
    value["unit"]["files"][0]["content"] = content.replace(from, to).into();
    value
}

#[test]
fn every_fixture_s_records_read_back_as_they_were_derived() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cpg/testdata");
    let mut read = 0;
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if !path.to_string_lossy().ends_with(".cpg.json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let Ok(Parsed::Bundled(document)) = layout::parse(&text) else {
            continue;
        };
        let Ok(records) = records::records_with_gaps(&document, ALL) else {
            continue;
        };
        for record in records {
            let back = decoded(&encoded(&record).unwrap(), record.wording).unwrap();
            assert_eq!(
                format!("{back:?}"),
                format!("{record:?}"),
                "{}",
                path.display()
            );
            read += 1;
        }
    }
    assert!(read > 100, "{read}");
}

#[test]
fn a_second_run_reads_every_record_back() {
    let store = tempfile::tempdir().unwrap();
    let document = document(&example());
    let derived = records::records(&document, ALL).unwrap();

    let first = FunctionFacts::new(store.path(), "build", ALL, false);
    assert_eq!(texts(&first.records(&document).unwrap()), texts(&derived));
    let counts = first.finish().unwrap();
    assert_eq!((counts.reused, counts.derived), (0, derived.len()));

    for verify in [false, true] {
        let again = FunctionFacts::new(store.path(), "build", ALL, verify);
        assert_eq!(texts(&again.records(&document).unwrap()), texts(&derived));
        assert_eq!(again.finish().unwrap().reused, derived.len());
    }
}

#[test]
fn another_build_or_demand_reads_none_of_them() {
    let store = tempfile::tempdir().unwrap();
    let document = document(&example());
    let first = FunctionFacts::new(store.path(), "build", ALL, false);
    first.records(&document).unwrap();
    first.finish().unwrap();

    let fewer = Demand {
        defects: false,
        ..ALL
    };
    for (build, demand) in [("another", ALL), ("build", fewer)] {
        let other = FunctionFacts::new(store.path(), build, demand, false);
        let records = other.records(&document).unwrap();
        assert_eq!(
            texts(&records),
            texts(&records::records(&document, demand).unwrap())
        );
        assert_eq!(other.finish().unwrap().reused, 0);
    }
}

#[test]
fn a_function_whose_facts_fail_fails_its_document_as_deriving_it_does() {
    let mut value = example();
    let function = &mut value["unit"]["functions"][0];
    function["span"]["end"] = json!(function["signature"]["span"]["start"].as_u64().unwrap() - 1);
    let document = document(&value);
    let store = tempfile::tempdir().unwrap();
    let facts = FunctionFacts::new(store.path(), "build", Demand::default(), false);

    let expected = records::records(&document, Demand::default()).unwrap_err();
    assert_eq!(facts.records(&document).unwrap_err(), expected);
}

#[test]
fn a_store_that_lost_or_spoiled_its_packs_derives_again() {
    let store = tempfile::tempdir().unwrap();
    let document = document(&example());
    let derived = texts(&records::records(&document, ALL).unwrap());
    let kept = |store: &Path| {
        let facts = FunctionFacts::new(store, "build", ALL, false);
        let records = facts.records(&document).unwrap();
        (texts(&records), facts.finish().unwrap())
    };
    kept(store.path());
    let packs: Vec<PathBuf> = walk(&store.path().join("objects"));
    assert_eq!(packs.len(), 1);

    // A frame that no longer reads is derived again; the others still read.
    let mut bytes = std::fs::read(&packs[0]).unwrap();
    bytes[MAGIC.len() + FORMAT.len() + 8] ^= 0xff;
    std::fs::write(&packs[0], &bytes).unwrap();
    let (records, counts) = kept(store.path());
    assert_eq!(records, derived);
    assert_eq!((counts.reused, counts.derived), (2, 1));
    // The pack made again is the one that was spoiled, and is written again.
    assert_eq!(walk(&store.path().join("objects")), packs);
    assert_eq!(kept(store.path()).1.reused, 3);

    // A pack that's gone, or isn't one, reads as holding nothing.
    for spoiled in [None, Some(b"not a pack".to_vec())] {
        for pack in walk(&store.path().join("objects")) {
            match &spoiled {
                Some(bytes) => std::fs::write(&pack, bytes).unwrap(),
                None => std::fs::remove_file(&pack).unwrap(),
            }
        }
        let (records, counts) = kept(store.path());
        assert_eq!(records, derived);
        assert_eq!(counts.reused, 0);
    }
}

#[test]
fn a_changed_document_over_a_spoiled_pack_keeps_a_pack_of_its_own() {
    let store = tempfile::tempdir().unwrap();
    let kept = |document: &Document| {
        let facts = FunctionFacts::new(store.path(), "build", ALL, false);
        let records = facts.records(document).unwrap();
        (texts(&records), facts.finish().unwrap())
    };
    kept(&document(&example()));
    let packs = walk(&store.path().join("objects"));
    let mut bytes = std::fs::read(&packs[0]).unwrap();
    bytes[MAGIC.len() + FORMAT.len() + 8] ^= 0xff;
    std::fs::write(&packs[0], &bytes).unwrap();

    // The first function's frame no longer reads and the last one changed, so the pack the
    // document makes is another than the spoiled one, which the map no longer names.
    let mut value = example();
    value["unit"]["functions"][2]["nodes"][0]["original_kind"] = "Changed".into();
    let changed = document(&value);
    let (records, counts) = kept(&changed);
    assert_eq!(records, texts(&records::records(&changed, ALL).unwrap()));
    assert_eq!((counts.reused, counts.derived), (1, 2));
    assert_eq!(walk(&store.path().join("objects")).len(), 2);
    assert_eq!(kept(&changed).1.reused, 3);
}

#[test]
fn a_document_with_no_function_to_keep_writes_no_pack() {
    let mut value = example();
    let functions = value["unit"]["functions"].as_array_mut().unwrap();
    functions.truncate(1);
    let mut twin = functions[0].clone();
    twin["id"] = "fn:twin".into();
    functions.push(twin);
    let document = document(&value);
    let derived = texts(&records::records(&document, ALL).unwrap());
    let store = tempfile::tempdir().unwrap();
    for _ in 0..2 {
        let facts = FunctionFacts::new(store.path(), "build", ALL, false);
        assert_eq!(texts(&facts.records(&document).unwrap()), derived);
        let counts = facts.finish().unwrap();
        assert_eq!((counts.reused, counts.derived), (0, 2));
    }
    assert!(!store.path().join("objects").exists());
}

#[test]
fn a_function_short_of_its_syntax_reads_back_with_why() {
    let mut value = example();
    let nodes = value["unit"]["functions"][2]["nodes"]
        .as_array_mut()
        .unwrap();
    let ten = nodes
        .iter_mut()
        .find(|node| node["id"] == "count.ten")
        .unwrap();
    ten["kind"] = "unsupported".into();
    let document = document(&value);
    let record = records::records(&document, ALL).unwrap().remove(2);
    assert!(record.syntax_gap.is_some());
    let text = encoded(&record).unwrap();
    let back = decoded(&text, record.wording).unwrap();
    assert_eq!(format!("{back:?}"), format!("{record:?}"));

    // A reason this build doesn't give doesn't read.
    let mut unknown: Value = serde_json::from_slice(&text).unwrap();
    unknown["syntax_gap"] = "A reason no build gives".into();
    let error = decoded(unknown.to_string().as_bytes(), record.wording).unwrap_err();
    assert!(
        error.contains("no syntax gap reads A reason no build gives"),
        "{error}"
    );
}

fn walk(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        match path.is_dir() {
            true => found.extend(walk(&path)),
            false => found.push(path),
        }
    }
    found
}

#[test]
fn verifying_refuses_a_kept_record_that_differs_and_says_where() {
    let document = document(&example());
    let records = records::records(&document, ALL).unwrap();
    let mut changed = records[0].clone();
    changed.root = "elsewhere".into();

    assert!(same(&records[0], &records[0].clone(), "key").is_ok());
    let error = same(&changed, &records[0], "key").unwrap_err();
    assert!(
        error.contains("differ from what deriving it again gives"),
        "{error}"
    );
    assert!(error.contains("elsewhere"), "{error}");
}

#[test]
fn a_key_is_the_same_for_the_same_document() {
    let keys = keys(&example());
    assert!(keys.iter().all(Option::is_some));
    assert_eq!(keys, self::keys(&example()));
    let distinct: BTreeSet<&Option<String>> = keys.iter().collect();
    assert_eq!(distinct.len(), keys.len());
}

#[test]
fn a_function_s_own_change_changes_its_key_alone() {
    let mut value = example();
    value["unit"]["functions"][2]["nodes"][0]["original_kind"] = "Changed".into();
    let (before, after) = (keys(&example()), keys(&value));
    assert_eq!(before[..2], after[..2]);
    assert_ne!(before[2], after[2]);
}

#[test]
fn a_change_to_the_tables_or_the_document_changes_every_key() {
    let mut types = example();
    types["unit"]["types"][1]["spelling"] = "u64".into();
    let mut files = example();
    files["unit"]["files"][0]["path"] = "src/other.rs".into();
    let mut producer = example();
    producer["producer"]["version"] = "2".into();
    let before = keys(&example());
    for changed in [types, files, producer] {
        let after = keys(&changed);
        assert!(before.iter().zip(&after).all(|(a, b)| a != b), "{after:?}");
    }
}

#[test]
fn where_a_definition_is_written_is_not_read() {
    let mut moved = example();
    moved["unit"]["definitions"][0]["span"]["start"] = json!(1);
    assert_eq!(keys(&moved), keys(&example()));
}

#[test]
fn a_function_reads_its_own_text_and_the_lines_above_it_but_nothing_below() {
    let before = keys(&example());
    // Inside `count`, the last function: only its key reads it.
    let inside = keys(&rewritten(
        &example(),
        "let mut total = 0;",
        "let mut total = 0 ",
    ));
    assert_eq!(before[..2], inside[..2]);
    assert_ne!(before[2], inside[2]);
    // Inside `larger`, the first function: `top` and `count` start within the lines searched
    // above them for a comment, so theirs read it too.
    let above = keys(&rewritten(&example(), "a.max(b)", "a.max(c)"));
    assert!(before.iter().zip(&above).all(|(a, b)| a != b));
    // Past the end of the last function, nothing reads it.
    let mut appended = example();
    let content = appended["unit"]["files"][0]["content"]
        .as_str()
        .unwrap()
        .to_owned();
    appended["unit"]["files"][0]["content"] = format!("{content}\n// later\n").into();
    assert_eq!(keys(&appended), before);
}

#[test]
fn a_file_with_no_text_is_read_as_having_none() {
    let value = example();
    let document = document(&value);
    let unit = document.unit.as_ref().unwrap();
    let keys = Keys::new(&document, unit, "scope");
    assert!(
        keys.window("file:lib", None, (0, 1))
            .contains("has no content")
    );
}

#[test]
fn a_span_in_a_file_the_document_does_not_hold_reads_no_text() {
    let mut value = example();
    value["unit"]["functions"][2]["nodes"][0]["span"]["file"] = "file:elsewhere".into();
    let (before, after) = (keys(&example()), keys(&value));
    assert_eq!(before[..2], after[..2]);
    assert!(after[2].is_some());
    assert_ne!(before[2], after[2]);
}

#[test]
fn functions_that_share_an_identity_are_never_kept() {
    let mut value = example();
    let functions = value["unit"]["functions"].as_array_mut().unwrap();
    let mut twin = functions[0].clone();
    twin["id"] = "fn:twin".into();
    functions.push(twin);
    let keys = keys(&value);
    assert_eq!(keys[0], None);
    assert_eq!(keys[3], None);
    assert!(keys[1].is_some() && keys[2].is_some());

    let mut unnamed = example();
    unnamed["unit"]["functions"][0]["definition"] = "def:nothing".into();
    let document = layout::bundle(vec![unnamed]).unwrap();
    let unit = document.unit.as_ref().unwrap();
    let function = &unit.functions.as_deref().unwrap()[0];
    assert_eq!(Keys::new(&document, unit, "scope").key(function), None);
}

#[test]
fn a_facts_pack_lists_where_each_key_s_frame_lies() {
    let frames = vec![("a".to_owned(), vec![1, 2, 3]), ("b".to_owned(), vec![4])];
    let bytes = pack("unit:t", frames).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("pack");
    std::fs::write(&path, &bytes).unwrap();
    let index = pack_index(&path).unwrap();
    assert_eq!(index, [("a".to_owned(), [8, 3]), ("b".to_owned(), [11, 1])]);

    let mut unended = bytes.clone();
    let at = unended.len() - 1;
    unended[at] = b'?';
    std::fs::write(&path, &unended).unwrap();
    assert!(pack_index(&path).unwrap_err().contains("index's place"));

    let other = json!({"schema": "another", "functions": []});
    assert!(index_entries(other.to_string().as_bytes()).is_err());
    let unplaced = json!({"schema": FACTS_SCHEMA, "functions": [["a", 1]]});
    assert!(index_entries(unplaced.to_string().as_bytes()).is_err());
    let unlisted = json!({"schema": FACTS_SCHEMA});
    assert!(index_entries(unlisted.to_string().as_bytes()).is_err());
}

#[test]
fn a_run_with_another_demand_than_its_facts_is_refused() {
    let store = tempfile::tempdir().unwrap();
    let facts = FunctionFacts::new(store.path(), "build", ALL, false);
    let pool = crate::cpg::Pool {
        workers: 1,
        demand: Demand::default(),
        on_failure: &|| {},
        on_document: &|_| {},
        on_records: &crate::cpg::keep_records,
        order: crate::cpg::Order::Canonical,
        facts: Some(&facts),
    };
    let jobs = [Ok(example())];
    let error = crate::cpg::pooled_records(jobs, |_, d| Parsed::from_json(&d), &pool).unwrap_err();
    assert!(error.contains("the store's facts are for"), "{error}");
}

#[test]
fn a_pool_with_a_store_gives_what_one_without_gives() {
    let store = tempfile::tempdir().unwrap();
    let pooled = |facts: Option<&FunctionFacts>| {
        let pool = crate::cpg::Pool {
            workers: 2,
            demand: ALL,
            on_failure: &|| {},
            on_document: &|_| {},
            on_records: &crate::cpg::keep_records,
            order: crate::cpg::Order::Canonical,
            facts,
        };
        let mut other = example();
        other["unit"]["id"] = "unit:other".into();
        let jobs = [Ok(example()), Ok(other)];
        let records = crate::cpg::pooled_records(jobs, |_, d| Parsed::from_json(&d), &pool);
        texts(&records.unwrap())
    };
    let expected = pooled(None);
    for reused in [0, expected.len()] {
        let facts = FunctionFacts::new(store.path(), "build", ALL, false);
        assert_eq!(pooled(Some(&facts)), expected);
        assert_eq!(facts.finish().unwrap().reused, reused);
    }
}
