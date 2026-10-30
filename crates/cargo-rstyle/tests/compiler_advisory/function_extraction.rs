use super::propagation::checked;
use super::*;

const OPTIONS: &[&str] = &[
    "--compiler-patterns",
    "--compiler-plans",
    "--review-file",
    "reviews.json",
];
const SOURCE: &str = r#"
#[derive(Clone,Copy)] pub struct Inner {pub count:u32}
#[derive(Clone,Copy)] pub struct Outer {pub inner:Inner,pub extra:usize}
pub fn retained(value:Outer)->Outer {value}
impl Outer {pub fn length(&self,data:&[u8])->usize {data.len()}}
pub fn field(value:&Outer)->u32 {value.inner.count}
pub fn update(value:&mut Outer,scalar:&mut u32,index:&mut usize) {
    value.inner.count+=1; *scalar+=2; *index+=3;
    loop {if *index>3 {break;} *index+=1;}
}
impl From<u32> for Inner {fn from(count:u32)->Self {Self {count}}}
impl Default for Outer {fn default()->Self {Self {inner:Inner {count:0},extra:0}}}
pub fn identity<T>(value:T)->T {value}
pub fn callback(value:Option<u32>)->Option<u32> {value.and_then(|n| Some(identity(n)))}
pub fn leaf(a:u128,b:u128,c:u128,d:u128,e:u128) {
    std::fs::write("/unused",b"leaf").ok();
}
pub fn caller(a:u128,b:u128,c:u128,d:u128,e:u128) {leaf(a,b,c,d,e);}
"#;
const STRICT_KEYS: &[&str] = &[
    rstyle_pdg::data_receivers::KEY,
    rstyle_pdg::declaration_origins::KEY,
    rstyle_pdg::inherent_calls::KEY,
    rstyle_pdg::field_locations::KEY,
    rstyle_pdg::consumer_facts::formal_types::KEY,
    rstyle_pdg::consumer_facts::control_types::KEY,
    "rstyle.rust.concrete-conversion-member",
    "rstyle.rust.concrete-default-member",
];

fn originals(report: &Value) -> std::collections::BTreeMap<&str, &Value> {
    report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|function| {
            let facts = &function["facts"];
            (facts["name"].as_str().unwrap(), facts)
        })
        .collect()
}

fn native_body(combined: &Value, raw: &Value) {
    for key in [
        "hir",
        "kind",
        "span",
        "snippet",
        "binding",
        "binding_mut",
        "lit",
    ] {
        assert_eq!(combined[key], raw[key], "native body {key}");
    }
    let before = combined["children"].as_array().unwrap();
    let after = raw["children"].as_array().unwrap();
    assert_eq!(before.len(), after.len());
    for (before, after) in before.iter().zip(after) {
        native_body(before, after);
    }
}

fn native_bindings(combined: &Value, raw: &Value) {
    let before = combined.as_array().unwrap();
    let after = raw.as_array().unwrap();
    assert_eq!(before.len(), after.len());
    for (before, after) in before.iter().zip(after) {
        for key in [
            "symbol",
            "name",
            "decl_span",
            "scope",
            "hir",
            "is_param",
            "is_mut",
        ] {
            assert_eq!(before[key], after[key], "native binding {key}");
        }
        assert_eq!(before["ty"]["observed"], after["ty"]["observed"]);
    }
}

fn retained_interfaces(combined: &Value, raw: &Value) {
    let combined = originals(combined);
    let raw = originals(raw);
    assert_eq!(
        combined.keys().collect::<Vec<_>>(),
        raw.keys().collect::<Vec<_>>()
    );
    for (name, before) in combined {
        let after = raw[name];
        for key in ["signature", "source", "fn_span"] {
            assert_eq!(before[key], after[key], "{name}: {key}");
        }
        let key = rstyle_pdg::concrete_types::KEY;
        assert_eq!(
            before["build"]["frontend_options"][key],
            after["build"]["frontend_options"][key]
        );
        native_bindings(&before["bindings"], &after["bindings"]);
        native_body(&before["body"], &after["body"]);
    }
    let retained = raw
        .iter()
        .find(|(name, _)| name.ends_with("retained"))
        .unwrap()
        .1;
    assert!(
        retained["signature"]["return_ty"]["normalized"]
            .as_str()
            .is_some()
    );
    assert!(!retained["body"]["children"].as_array().unwrap().is_empty());
}

fn strict_metadata(combined: &Value, raw: &Value) {
    let combined = originals(combined);
    let raw = originals(raw);
    for key in STRICT_KEYS {
        assert!(
            combined
                .values()
                .any(|facts| facts["build"]["frontend_options"].get(*key).is_some()),
            "{key}"
        );
        assert!(
            raw.values()
                .all(|facts| facts["build"]["frontend_options"].get(*key).is_none()),
            "{key}"
        );
    }
    for (key, roster) in [
        (rstyle_pdg::data_receivers::KEY, "methods"),
        (rstyle_pdg::field_locations::KEY, "operations"),
        (rstyle_pdg::consumer_facts::formal_types::KEY, "mutable_u32"),
        (rstyle_pdg::consumer_facts::control_types::KEY, "never"),
    ] {
        assert!(
            combined.values().any(|facts| {
                facts["build"]["frontend_options"][key][roster]
                    .as_array()
                    .is_some_and(|rows| !rows.is_empty())
            }),
            "{key}: {roster}"
        );
    }
}

fn independent_measurements(report: &Value) {
    let unit = &report["compiler_advisory"]["units"][0];
    assert_eq!(unit["evaluation"]["summary"]["effect_counts"]["io"], 2);
    let update = unit["observations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"].as_str().unwrap().ends_with("update"))
        .unwrap();
    assert_eq!(update["direct_mutations"], 4);
    assert!(!unit["unavailable"].as_array().unwrap().is_empty());
    assert_eq!(report["compiler_advisory"]["complete"], false);
    assert!(
        !report["compiler_advisory"]["planning"]["groups"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

fn review_index(report: &Value) -> usize {
    let line = SOURCE
        .lines()
        .position(|line| line.starts_with("pub fn leaf"))
        .unwrap()
        + 1;
    report["compiler_advisory"]["units"][0]["evaluation"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .position(|row| row["line"] == line && row["rule_id"] == "fn_params")
        .unwrap()
}

fn source_free_review(directory: &Path, original: &Value) {
    let index = review_index(original).to_string();
    std::fs::remove_dir_all(directory.join("src")).unwrap();
    std::fs::remove_file(directory.join("Cargo.toml")).unwrap();
    let mut options = OPTIONS.to_vec();
    options.extend_from_slice(&[
        "--compiler-advisory-only",
        "--dismiss-compiler-diagnostic",
        "0",
        &index,
        "--review-reason",
        "Reviewed original parameter boundary after capture",
    ]);
    let (output, replayed) = super::reviews::replay(directory, original, &options);
    checked(&output, 2);
    let advisory = &replayed["compiler_advisory"];
    assert_eq!(
        advisory["diagnostic_review"]["decisions"][index.parse::<usize>().unwrap()]["state"],
        "hidden_exact"
    );
    assert_eq!(
        advisory["units"][0]["evaluation"],
        original["compiler_advisory"]["units"][0]["evaluation"]
    );
    assert_eq!(
        advisory["planning"],
        original["compiler_advisory"]["planning"]
    );
}

#[test]
fn advisory_keeps_concrete_signatures_and_original_bodies_without_strict_enrichment() {
    let (directory, output, combined) = capture(SOURCE, OPTIONS);
    checked(&output, 2);
    let mut options = OPTIONS.to_vec();
    options.push("--compiler-advisory-only");
    let (output, raw) = super::propagation::capture(directory.path(), &options);
    checked(&output, 2);
    assert_eq!(raw["compiler_advisory"], combined["compiler_advisory"]);
    assert_eq!(
        raw["owned_source_inventory"],
        combined["owned_source_inventory"]
    );
    retained_interfaces(&combined, &raw);
    strict_metadata(&combined, &raw);
    independent_measurements(&raw);
    source_free_review(directory.path(), &raw);
}
