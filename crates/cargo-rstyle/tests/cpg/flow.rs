//! The control flow the Rust frontend emits and what core derives from it, read back in
//! terms of the source each node is written as.

use super::*;
use std::collections::BTreeSet;

const SOURCE: &str = r#"
pub fn sink(_value: u32) {}
pub fn source() -> u32 { 1 }

pub fn early(flag: bool) -> u32 {
    if flag {
        return 11;
    }
    source()
}

pub fn short(left: bool, right: bool) -> u32 {
    if left && right { source() } else { 22 }
}

pub fn repeating(limit: u32) -> u32 {
    let mut total = 0;
    let mut index = 0;
    while index < limit {
        if index == 3 { index += 2; continue; }
        if index == 9 { break; }
        total += index;
        index += 1;
    }
    total
}

pub fn nested(items: &[u32]) -> u32 {
    let mut total = 0;
    'all: for item in items {
        for other in items {
            if other == item { continue 'all; }
            total += *other;
        }
    }
    total
}

pub fn selecting(value: Option<u32>) -> u32 {
    match value {
        Some(0) => source(),
        Some(n) if n > 3 => n,
        Some(n) => n + 1,
        None => 33,
    }
}

pub fn question(value: Result<u32, String>) -> Result<u32, String> {
    let inner = value?;
    Ok(inner + 1)
}

pub fn elsewise(value: Option<u32>) -> u32 {
    let Some(inner) = value else { return 44 };
    inner
}

pub fn chained(value: Option<u32>, flag: bool) -> u32 {
    if let Some(inner) = value && flag { inner } else { 55 }
}

pub fn panicking(flag: bool) -> u32 {
    if flag {
        panic!("no");
    }
    source()
}

pub fn endless() -> ! {
    loop { sink(source()); }
}

pub fn labeled(flag: bool) -> u32 {
    let value = 'done: {
        if flag { break 'done 66; }
        77
    };
    value
}

pub fn overwriting() -> u32 {
    let mut value = 1;
    value = source();
    value
}

pub fn joining(flag: bool) -> u32 {
    let mut value = 0;
    if flag {
        value = 1;
    }
    value
}

pub fn updating() -> u32 {
    let mut value = 0;
    value += 1;
    value
}

pub fn borrowing() -> u32 {
    let mut value = 0;
    let held = &mut value;
    *held = 3;
    value
}

pub fn pushing() -> usize {
    let mut items = Vec::new();
    items.push(1u32);
    items.len()
}

pub fn fields() -> u32 {
    let mut pair = (0u32, 0u32);
    pair.0 = 1;
    pair.1
}

pub fn closing() -> u32 {
    let mut count = 0;
    let mut bump = || count += 1;
    bump();
    count
}

pub async fn sleeping(flag: bool) -> u32 {
    if flag { return 88; }
    other().await
}

pub async fn other() -> u32 { 2 }

pub fn swapping() -> u32 {
    let (mut a, mut b) = (1, 2);
    (a, b) = (b, a);
    a + b
}

pub fn leaving(limit: u32) -> Result<u32, String> {
    let mut index = 0;
    while index < limit {
        if index == 5 { return Ok(index); }
        let parsed: Result<u32, String> = Ok(index);
        index += parsed?;
    }
    let done = 'block: {
        loop {
            if index == 9 { break 'block index; }
            index += 1;
        }
    };
    Ok(done)
}

pub fn appending(mut text: String) -> String {
    text += "x";
    text
}
"#;

struct Emitted {
    document: Value,
    flow: Value,
    source: String,
    _directory: TempDir,
}

fn emitted() -> &'static Emitted {
    static EMITTED: OnceLock<Emitted> = OnceLock::new();
    EMITTED.get_or_init(|| {
        let directory = clone_workspace();
        std::fs::write(directory.path().join("src/lib.rs"), SOURCE).unwrap();
        // A let chain needs the current edition.
        let manifest = directory.path().join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest).unwrap();
        std::fs::write(&manifest, text.replace("edition='2021'", "edition='2024'")).unwrap();
        let out = directory.path().join("cpg");
        let emit = rstyle(
            directory.path(),
            &["rstyle", "cpg", "emit", "--out", out.to_str().unwrap()],
        );
        assert_eq!(emit.status.code(), Some(0), "{emit:?}");
        let paths = documents_in(&out);
        let [path] = paths.as_slice() else {
            panic!("one document, not {paths:?}");
        };
        let flow = rstyle(
            directory.path(),
            &["rstyle", "--format", "json", "cpg", "flow", path],
        );
        assert_eq!(flow.status.code(), Some(0), "{flow:?}");
        let document: Value = rstyle_core::cpg::transport::read_json(Path::new(path)).unwrap();
        let source = document["unit"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .find_map(|file| file["content"].as_str())
            .unwrap()
            .to_owned();
        Emitted {
            document,
            flow: serde_json::from_slice(&flow.stdout).unwrap(),
            source,
            _directory: directory,
        }
    })
}

/// One function: its document record and what core derived of it.
struct Function {
    record: &'static Value,
    derived: &'static Value,
}

fn function(name: &str) -> Function {
    let emitted = emitted();
    let record = emitted.document["unit"]["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|function| function["name"] == name)
        .unwrap_or_else(|| panic!("no function {name}"));
    let derived = emitted.flow["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .map(|row| &row["flow"])
        .unwrap();
    Function { record, derived }
}

impl Function {
    /// What a node is written as, or its kind where the compiler wrote it.
    fn text(&self, id: &str) -> String {
        if matches!(id, "entry" | "exit") {
            return id.to_owned();
        }
        let node = self.record["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["id"] == id)
            .unwrap_or_else(|| panic!("no node {id}"));
        match node["span"].as_object() {
            Some(span) => {
                let (start, end) = (
                    span["start"].as_u64().unwrap(),
                    span["end"].as_u64().unwrap(),
                );
                emitted().source[start as usize..end as usize].to_owned()
            }
            None => node["kind"].as_str().unwrap().to_owned(),
        }
    }

    fn standing(&self) -> &str {
        self.derived["standing"].as_str().unwrap()
    }

    fn reasons(&self) -> String {
        self.derived["reasons"].to_string()
    }

    fn control(&self) -> BTreeSet<(String, String, String)> {
        self.derived["control"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| {
                (
                    self.text(edge["source"].as_str().unwrap()),
                    edge["kind"].as_str().unwrap().to_owned(),
                    self.text(edge["target"].as_str().unwrap()),
                )
            })
            .collect()
    }

    fn depends(&self, source: &str, kind: &str, target: &str) -> bool {
        self.control()
            .contains(&(source.to_owned(), kind.to_owned(), target.to_owned()))
    }

    fn data(&self) -> BTreeSet<(String, String)> {
        self.derived["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| {
                (
                    self.text(edge["source"].as_str().unwrap()),
                    self.text(edge["target"].as_str().unwrap()),
                )
            })
            .collect()
    }

    /// The transfers leaving the vertex written as `source`.
    fn transfers(&self, source: &str) -> BTreeSet<(String, String)> {
        self.record["cfg"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|edge| self.text(edge["source"].as_str().unwrap()) == source)
            .map(|edge| {
                (
                    edge["kind"].as_str().unwrap().to_owned(),
                    self.text(edge["target"].as_str().unwrap()),
                )
            })
            .collect()
    }

    fn edges_with(&self, kind: &str) -> Vec<&'static Value> {
        self.record["cfg"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|edge| edge["kind"] == kind)
            .collect()
    }

    fn loops(&self) -> Vec<(String, usize)> {
        self.derived["loops"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(head, sources)| (self.text(head), sources.as_array().unwrap().len()))
            .collect()
    }
}

fn pair(source: &str, target: &str) -> (String, String) {
    (source.to_owned(), target.to_owned())
}

#[test]
fn what_follows_an_early_return_depends_on_the_branch_it_did_not_take() {
    let early = function("early");

    assert!(early.depends("flag", "true", "return 11"));
    assert!(early.depends("flag", "false", "source()"));
    assert_eq!(early.standing(), "known", "{}", early.reasons());
    assert!(early.data().contains(&pair("entry", "flag")));
}

#[test]
fn both_operands_of_a_short_circuit_branch() {
    let short = function("short");

    assert!(short.depends("left", "true", "right"));
    assert!(short.depends("right", "true", "source()"));
    assert!(short.depends("left", "false", "22"));
    assert!(short.depends("right", "false", "22"));
}

#[test]
fn continue_and_the_end_of_the_body_repeat_a_loop_and_break_leaves_it() {
    let repeating = function("repeating");
    // The loop repeats from what it tests first, and rejoins at its own vertex.
    assert_eq!(repeating.loops(), vec![("index".to_owned(), 2)]);
    assert!(repeating.depends("index == 9", "true", "break"));
    assert!(repeating.depends("index < limit", "true", "index == 3"));
    let leaving: Vec<_> = repeating
        .edges_with("break")
        .into_iter()
        .filter(|edge| edge["loop_role"] == "loop_exit")
        .collect();
    assert_eq!(leaving.len(), 1);
}

#[test]
fn a_labeled_continue_repeats_the_loop_it_names() {
    let nested = function("nested");
    let continues = nested.edges_with("continue");
    let [continued] = continues.as_slice() else {
        panic!("one continue, not {continues:?}");
    };

    assert_eq!(continued["loop_role"], "back_edge");
    assert!(
        nested
            .text(continued["target"].as_str().unwrap())
            .starts_with("'all: for")
    );
    assert_eq!(nested.loops().len(), 2);
    assert_eq!(nested.standing(), "known", "{}", nested.reasons());
}

#[test]
fn a_failed_guard_falls_to_the_next_arm() {
    let selecting = function("selecting");
    let cases: Vec<_> = selecting.edges_with("case");

    assert_eq!(cases.len(), 4);
    assert!(
        cases
            .iter()
            .all(|edge| selecting.text(edge["source"].as_str().unwrap()) == "value")
    );
    let failed: Vec<_> = selecting
        .edges_with("false")
        .into_iter()
        .filter(|edge| selecting.text(edge["source"].as_str().unwrap()) == "n > 3")
        .collect();
    let [failed] = failed.as_slice() else {
        panic!("one way out of a failed guard, not {failed:?}");
    };
    // The next arm's pattern binds `n` first.
    assert_eq!(selecting.text(failed["target"].as_str().unwrap()), "n");
    assert!(selecting.depends("n > 3", "true", "n"));
}

#[test]
fn a_question_mark_returns_or_continues() {
    let question = function("question");
    let leaving = question.transfers("value?");

    assert!(leaving.contains(&("return".to_owned(), "exit".to_owned())));
    assert!(leaving.iter().any(|(kind, _)| kind == "normal"));
    assert!(question.depends("value?", "normal", "Ok(inner + 1)"));
}

#[test]
fn a_let_else_binds_or_diverges_and_a_let_chain_binds_only_when_it_holds() {
    let elsewise = function("elsewise");
    assert!(elsewise.depends(
        "let Some(inner) = value else { return 44 };",
        "false",
        "return 44"
    ));
    assert!(elsewise.depends(
        "let Some(inner) = value else { return 44 };",
        "true",
        "inner"
    ));

    let chained = function("chained");
    assert!(chained.depends("let Some(inner) = value", "true", "flag"));
    assert!(chained.depends("flag", "true", "inner"));
    assert!(chained.depends("let Some(inner) = value", "false", "55"));
}

#[test]
fn whatever_leaves_a_loop_is_marked_as_leaving_it() {
    let leaving = function("leaving");
    let roles = |kind: &str| -> Vec<&Value> {
        leaving
            .edges_with(kind)
            .into_iter()
            .map(|edge| &edge["loop_role"])
            .collect()
    };

    assert_eq!(
        roles("return"),
        vec!["loop_exit", "loop_exit"],
        "return and ?"
    );
    assert_eq!(
        roles("break"),
        vec!["loop_exit"],
        "a break out of a block around a loop"
    );
    assert_eq!(
        function("labeled").edges_with("break")[0].get("loop_role"),
        None,
        "a break out of a block with no loop in it"
    );
}

#[test]
fn a_panic_ends_its_path_without_reaching_the_exit() {
    let panicking = function("panicking");

    assert!(panicking.depends("flag", "false", "source()"));
    assert!(
        panicking.record["cfg"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .all(|edge| !(edge["target"] == "exit"
                && panicking
                    .text(edge["source"].as_str().unwrap())
                    .starts_with("panic!")))
    );
    assert_eq!(panicking.edges_with("return").len(), 0);
}

#[test]
fn a_loop_nothing_leaves_never_reaches_the_exit() {
    let endless = function("endless");
    let exits = endless.record["cfg"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|edge| edge["target"] == "exit")
        .count();

    assert_eq!(exits, 0);
    assert_eq!(endless.loops().len(), 1);
}

#[test]
fn a_labeled_block_is_where_its_break_lands() {
    let labeled = function("labeled");
    let breaks = labeled.edges_with("break");
    let [broke] = breaks.as_slice() else {
        panic!("one break, not {breaks:?}");
    };

    assert!(
        labeled
            .text(broke["target"].as_str().unwrap())
            .starts_with("'done: {")
    );
    assert!(broke.get("loop_role").is_none());
}

#[test]
fn a_read_sees_the_last_write_of_each_path() {
    let overwriting = function("overwriting");
    assert!(overwriting.data().contains(&pair("value", "value")));
    assert!(!overwriting.data().contains(&pair("mut value", "value")));

    let joining = function("joining");
    assert!(joining.data().contains(&pair("mut value", "value")));
    assert!(joining.data().contains(&pair("value", "value")));

    let updating = function("updating");
    assert!(updating.data().contains(&pair("mut value", "value")));
    assert_eq!(updating.standing(), "known", "{}", updating.reasons());
}

#[test]
fn what_a_borrow_or_capture_can_change_is_marked_approximate() {
    for (name, symbol) in [
        ("borrowing", "value"),
        ("pushing", "items"),
        ("closing", "count"),
    ] {
        let function = function(name);
        assert_eq!(function.standing(), "approximated", "{name}");
        assert!(
            function.reasons().contains(symbol),
            "{}",
            function.reasons()
        );
        assert!(
            function.derived["data"]
                .as_array()
                .unwrap()
                .iter()
                .any(|edge| edge["approximate"] == true),
            "{name}"
        );
    }
    let fields = function("fields");
    assert_eq!(fields.standing(), "approximated");
    assert!(fields.reasons().contains("in part"), "{}", fields.reasons());
    // Only the frontend's own statement of unwinding is left to assume.
    assert_eq!(fields.derived["assumptions"].as_array().unwrap().len(), 1);
}

#[test]
fn an_async_function_has_a_flow_and_its_return_stays_inside_it() {
    let sleeping = function("sleeping");

    assert!(sleeping.depends("flag", "true", "return 88"));
    assert!(sleeping.depends("flag", "false", "other()"));
    let returns = sleeping.edges_with("return");
    assert!(!returns.is_empty());
    assert!(returns.iter().all(|edge| edge["target"] != "exit"));
    // `.await` polls until the future is ready.
    assert_eq!(sleeping.loops().len(), 1);
}

#[test]
fn a_destructuring_assignment_writes_each_place_after_its_value() {
    let swapping = function("swapping");

    assert!(
        swapping
            .transfers("(b, a)")
            .contains(&("normal".to_owned(), "a".to_owned()))
    );
    assert!(swapping.data().contains(&pair("a", "a")));
    assert!(swapping.data().contains(&pair("mut b", "b")));
}

#[test]
fn an_overloaded_compound_assignment_runs_its_operands_in_written_order() {
    let appending = function("appending");

    assert!(
        appending
            .transfers("text")
            .contains(&("normal".to_owned(), "\"x\"".to_owned()))
    );
    assert!(appending.data().contains(&pair("entry", "text")));
}

#[test]
fn every_read_in_the_fixture_has_a_definition_that_reaches_it() {
    let emitted = emitted();
    for row in emitted.flow["functions"].as_array().unwrap() {
        let name = row["name"].as_str().unwrap();
        let record = function(name).record;
        let vertices: BTreeSet<&str> = record["cfg"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|edge| {
                [
                    edge["source"].as_str().unwrap(),
                    edge["target"].as_str().unwrap(),
                ]
            })
            .collect();
        let seen: BTreeSet<(&str, &str)> = row["flow"]["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| {
                (
                    edge["target"].as_str().unwrap(),
                    edge["symbol"].as_str().unwrap(),
                )
            })
            .collect();
        for access in record["accesses"].as_array().unwrap() {
            let (node, symbol) = (
                access["node"].as_str().unwrap(),
                access["symbol"].as_str().unwrap(),
            );
            if access["mode"] != "write" && vertices.contains(node) {
                assert!(
                    seen.contains(&(node, symbol)),
                    "{name}: {node} reads {symbol}"
                );
            }
        }
    }
}

#[test]
fn function_selection_narrows_the_report_and_text_states_each_standing() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        "pub fn only() {}\npub fn other() {}\n",
    )
    .unwrap();
    let out = directory.path().join("cpg");
    let emit = rstyle(
        directory.path(),
        &["rstyle", "cpg", "emit", "--out", out.to_str().unwrap()],
    );
    assert_eq!(emit.status.code(), Some(0), "{emit:?}");
    let paths = documents_in(&out);
    let mut arguments = vec!["rstyle", "cpg", "flow", "--function", "only"];
    arguments.extend(paths.iter().map(String::as_str));

    let output = rstyle(directory.path(), &arguments);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("Known  only"), "{text}");
    assert!(text.contains("1 functions"), "{text}");
    assert!(!text.contains("other"), "{text}");
}
