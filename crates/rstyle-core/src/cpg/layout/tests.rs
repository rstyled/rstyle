//! Reading a document straight into the model reads what reading it through a JSON tree read,
//! and rejects the rest, saying where.
use super::*;
use serde_json::json;
use std::path::Path;

const EXAMPLE: &str = include_str!("../testdata/example.cpg.json");

fn example() -> Value {
    serde_json::from_str(EXAMPLE).unwrap()
}

fn written(document: &Value) -> String {
    serde_json::to_string_pretty(document).unwrap()
}

fn error_of(text: &str) -> String {
    parse(text).unwrap_err()
}

#[test]
fn every_fixture_reads_into_the_model_as_it_did_through_a_json_tree() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cpg/testdata");
    let mut read = 0;
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if !path.to_string_lossy().ends_with(".cpg.json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).unwrap();
        let tree = bundle(vec![serde_json::from_str(&text).unwrap()]).unwrap();
        let typed = parse(&text).unwrap();
        assert_eq!(
            format!("{typed:?}"),
            format!("Bundled({tree:?})"),
            "{}",
            path.display()
        );
        read += 1;
    }
    assert!(read > 40, "{read}");
}

#[test]
fn a_document_the_model_cannot_hold_is_rejected_where_it_breaks() {
    let whole = EXAMPLE.trim_end();
    let truncated = &whole[..whole.len() - 1];
    assert!(
        error_of(truncated)
            .starts_with("Invalid compat.cpg document: EOF while parsing an object at line ")
    );
    let trailing = format!("{whole} {{}}");
    assert!(
        error_of(&trailing)
            .starts_with("Invalid compat.cpg document: trailing characters at line ")
    );

    let mut extra = example();
    extra["unit"]["functions"][0]["nodes"][0]["role"] = json!("tail");
    let error = error_of(&written(&extra));
    assert!(
        error.starts_with("Invalid compat.cpg document: unknown field `role`, expected one of "),
        "{error}"
    );
    assert!(error.contains(" at line "), "{error}");

    let mut mistyped = example();
    mistyped["unit"]["target"]["pointer_width"] = json!("64");
    let error = error_of(&written(&mistyped));
    assert!(
        error.contains("invalid type: string \"64\", expected u32 at line "),
        "{error}"
    );

    let mut neither = example();
    neither["layout"] = json!("neither");
    assert!(error_of(&written(&neither)).starts_with(
        "Invalid compat.cpg document: unknown variant `neither`, expected `bundled` or `split` at line "
    ));

    let mut newer = example();
    newer["profile"]["version"] = json!("0.2.0");
    assert_eq!(
        error_of(&written(&newer)),
        "Unsupported document: schema 0.1.0 of profile compat.cpg/0.2.0; this rstyle reads compat.cpg/0.1.0"
    );
}

/// A JSON tree kept the last of a member written twice. The model keeps neither, as no frontend
/// writes one and the profile can't say which would be meant.
#[test]
fn a_member_written_twice_is_rejected() {
    let twice = EXAMPLE.replacen(
        "\"layout\": \"bundled\",",
        "\"layout\": \"bundled\",\n  \"layout\": \"split\",",
        1,
    );
    assert!(
        error_of(&twice)
            .starts_with("Invalid compat.cpg document: duplicate field `layout` at line 8 column ")
    );
}

#[test]
fn a_split_document_is_kept_as_written_once_the_model_has_checked_it() {
    for document in split(&example()) {
        let read = parse(&written(&document));
        assert!(matches!(read, Ok(Parsed::Split(kept)) if kept == document));
    }

    let mut documents = split(&example());
    documents[1]["function"]["nodes"][0]["role"] = json!("tail");
    assert!(error_of(&written(&documents[1])).contains("unknown field `role`"));
}
