//! Parameter lists, signature shapes and mutated types derived from the example.

use super::run_facts;
use crate::facts::{FnFacts, ParamFacts, Receiver, SigFacts, TyShape};
use serde_json::{Value, json};

const EXAMPLE: &str = include_str!("testdata/example.cpg.json");

fn example() -> Value {
    serde_json::from_str(EXAMPLE).unwrap()
}

fn facts_of(document: Value) -> Vec<FnFacts> {
    run_facts(vec![document]).unwrap()
}

fn function<'d>(document: &'d mut Value, id: &str) -> &'d mut Value {
    let functions = document["unit"]["functions"].as_array_mut().unwrap();
    functions.iter_mut().find(|f| f["id"] == id).unwrap()
}

fn add_types(document: &mut Value, types: Value) {
    let table = document["unit"]["types"].as_array_mut().unwrap();
    for mut ty in types.as_array().unwrap().iter().cloned() {
        ty["spelling"] = ty["id"].clone();
        table.push(ty);
    }
}

fn add_definitions(document: &mut Value, definitions: &[(&str, &str, &str)]) {
    let table = document["unit"]["definitions"].as_array_mut().unwrap();
    for (id, kind, scope) in definitions {
        table.push(json!({"id": id, "kind": kind, "name": id, "stable_identity": id.trim_start_matches("def:"),
                          "scope": scope}));
    }
}

/// A workspace record `Point<A>`, a library record `Vec<T>` and a workspace trait `Shape`.
fn nominal(document: &mut Value) {
    add_definitions(
        document,
        &[
            ("def:geo::Point", "type", "workspace"),
            ("def:std::Vec", "type", "library"),
            ("def:geo::Shape", "trait", "workspace"),
            ("def:std::Box", "type", "library"),
        ],
    );
    add_types(
        document,
        json!([
            {"id": "point", "kind": "record", "definition": "def:geo::Point", "arguments": ["type:u32"]},
            {"id": "&point", "kind": "reference", "target": "point", "mode": "shared"},
            {"id": "&mut point", "kind": "reference", "target": "point", "mode": "exclusive"},
            {"id": "&&mut point", "kind": "reference", "target": "&mut point", "mode": "shared"},
            {"id": "box point", "kind": "pointer", "mode": "owned", "target": "point",
             "definition": "def:std::Box", "arguments": ["point"]},
            {"id": "&box point", "kind": "reference", "target": "box point", "mode": "shared"},
            {"id": "vec", "kind": "record", "definition": "def:std::Vec", "arguments": ["type:u32"]},
        ]),
    );
}

#[test]
fn parameters_list_their_names_written_types_and_uses() {
    let facts = facts_of(example());
    let param = |name: &str, ty: &str, uses| ParamFacts {
        name: name.into(),
        ty: ty.into(),
        uses,
    };
    assert_eq!(
        facts[0].param_list,
        [param("a", "T", 1), param("b", "T", 1)]
    );
    assert_eq!(facts[2].param_list, [param("items", "&[u32]", 1)]);

    let mut document = example();
    let parameter = &mut function(&mut document, "fn:top")["signature"]["parameters"][0];
    parameter.as_object_mut().unwrap().remove("annotation");
    assert_eq!(facts_of(document)[1].param_list[0], param("a", "", 1));
}

#[test]
fn written_types_collapse_whitespace() {
    let mut document = example();
    let file = &mut document["unit"]["files"][0];
    let content = file["content"]
        .as_str()
        .unwrap()
        .replace("(items: &[u32])", "(items: &[\n    u32])");
    file["content"] = json!(content);
    file["digest"] = json!(content_digest(&content));
    // Shift every span after the edit by the inserted text.
    let edit = content.find("&[\n").unwrap();
    shift_spans(&mut document, edit + 2, 5);
    let facts = facts_of(document);
    assert_eq!(facts[2].param_list[0].ty, "&[ u32]");
}

fn content_digest(content: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(content.as_bytes());
    let hex: String = hash.iter().map(|byte| format!("{byte:02x}")).collect();
    format!("sha256:{hex}")
}

fn shift_spans(value: &mut Value, from: usize, by: usize) {
    match value {
        Value::Object(map) => {
            if map.contains_key("file") {
                for key in ["start", "end"] {
                    if let Some(offset) = map.get(key).and_then(Value::as_u64)
                        && offset as usize >= from
                    {
                        map.insert(key.into(), json!(offset as usize + by));
                    }
                }
            }
            map.values_mut()
                .for_each(|inner| shift_spans(inner, from, by));
        }
        Value::Array(items) => items
            .iter_mut()
            .for_each(|inner| shift_spans(inner, from, by)),
        _ => {}
    }
}

#[test]
fn free_functions_have_no_signature_shape() {
    assert!(facts_of(example()).iter().all(|facts| facts.sig.is_none()));
}

#[test]
fn methods_shape_their_owner_receiver_parameters_and_modifiers() {
    let mut document = example();
    nominal(&mut document);
    let signature = &mut function(&mut document, "fn:top")["signature"];
    signature["owner_type"] = json!("point");
    signature["receiver"] = json!({"symbol": "top.a", "type": "&mut point"});
    signature["modifiers"] = json!(["async", "const"]);
    signature["generics"] = json!([{"name": "T", "definition": "def:larger.T", "bounds": []}]);
    let sig = facts_of(document)[1].sig.clone().unwrap();
    let u32 = || TyShape::Prim("u32".into());
    assert_eq!(
        sig,
        SigFacts {
            self_ty: Some("geo::Point".into()),
            self_args: vec![u32()],
            receiver: Receiver::RefMut,
            params: vec![u32(), u32()],
            ret: u32(),
            is_async: true,
            is_unsafe: false,
            is_const: true,
            generics: 1,
        }
    );
}

#[test]
fn receivers_are_classified_against_the_owner_type() {
    let receiver = |owner: &str, receiver: Option<&str>| {
        let mut document = example();
        nominal(&mut document);
        let signature = &mut function(&mut document, "fn:top")["signature"];
        signature["owner_type"] = json!(owner);
        if let Some(receiver) = receiver {
            signature["receiver"] = json!({"symbol": "top.a", "type": receiver});
        }
        facts_of(document)[1].sig.clone().unwrap()
    };
    assert_eq!(receiver("point", None).receiver, Receiver::None);
    assert_eq!(receiver("point", Some("point")).receiver, Receiver::Value);
    assert_eq!(receiver("point", Some("&point")).receiver, Receiver::Ref);
    assert_eq!(
        receiver("point", Some("box point")).receiver,
        Receiver::Other
    );
    assert_eq!(
        receiver("point", Some("&&mut point")).receiver,
        Receiver::Other
    );
    // An implementation for a type that is not nominal has no owner to compare with.
    let unowned = receiver("type:slice_ref", Some("type:slice_ref"));
    assert_eq!((unowned.self_ty, unowned.receiver), (None, Receiver::Other));
}

/// One type of each kind, by ID, after the example's tables and `nominal`.
const KINDS: [&str; 19] = [
    "pair",
    "unit",
    "array",
    "type:u32_slice",
    "usize",
    "i8",
    "f64",
    "char",
    "str",
    "never",
    "type:T",
    "callback",
    "dyn shape",
    "dyn send",
    "future",
    "type:iter",
    "raw",
    "box point",
    "vec",
];

fn every_kind(document: &mut Value) {
    nominal(document);
    add_definitions(document, &[("def:std::Future", "trait", "library")]);
    add_types(
        document,
        json!([
            {"id": "pair", "kind": "tuple", "items": ["type:u32", "type:bool"]},
            {"id": "unit", "kind": "unit"},
            {"id": "array", "kind": "sequence", "element": "type:u32", "fixed": true},
            {"id": "usize", "kind": "integer", "bits": 64, "signed": false, "pointer_sized": true},
            {"id": "i8", "kind": "integer", "bits": 8, "signed": true},
            {"id": "f64", "kind": "float", "bits": 64},
            {"id": "char", "kind": "char"}, {"id": "str", "kind": "string"}, {"id": "never", "kind": "bottom"},
            {"id": "callback", "kind": "function", "parameters": [], "result": "unit"},
            {"id": "dyn shape", "kind": "interface", "definition": "def:geo::Shape", "bounds": ["def:geo::Shape"]},
            {"id": "dyn send", "kind": "interface", "bounds": ["def:std::Future"]},
            {"id": "future", "kind": "opaque", "bounds": ["def:std::Future"]},
            {"id": "raw", "kind": "pointer", "mode": "raw", "target": "type:u32"},
        ]),
    );
}

fn expected_shapes() -> Vec<TyShape> {
    let prim = |name: &str| TyShape::Prim(name.into());
    let point = TyShape::Adt {
        id: "geo::Point".into(),
        args: vec![prim("u32")],
    };
    vec![
        TyShape::Tuple(vec![prim("u32"), prim("bool")]),
        TyShape::Tuple(vec![]),
        TyShape::Array(Box::new(prim("u32"))),
        TyShape::Slice(Box::new(prim("u32"))),
        prim("usize"),
        prim("i8"),
        prim("f64"),
        prim("char"),
        prim("str"),
        prim("!"),
        TyShape::Param(0),
        TyShape::FnPtr,
        TyShape::Dyn("geo::Shape".into()),
        TyShape::Dyn(String::new()),
        TyShape::Opaque,
        TyShape::Other,
        TyShape::Other,
        TyShape::Adt {
            id: "std::Box".into(),
            args: vec![point],
        },
        TyShape::Adt {
            id: "std::Vec".into(),
            args: vec![prim("u32")],
        },
    ]
}

#[test]
fn types_have_structural_shapes() {
    let mut document = example();
    every_kind(&mut document);
    let parameters: Vec<Value> = KINDS
        .iter()
        .enumerate()
        .map(|(position, ty)| json!({"position": position, "symbol": "top.a", "type": ty}))
        .collect();
    let signature = &mut function(&mut document, "fn:top")["signature"];
    signature["parameters"] = json!(parameters);
    signature["owner_type"] = json!("vec");
    let sig = facts_of(document)[1].sig.clone().unwrap();

    assert_eq!(sig.params, expected_shapes());
    let u32 = TyShape::Prim("u32".into());
    assert_eq!(
        (sig.self_ty.as_deref(), sig.self_args),
        (Some("std::Vec"), vec![u32])
    );
}

#[test]
fn incomplete_types_are_rejected() {
    let error = |ty: Value| {
        let mut document = example();
        add_types(&mut document, json!([ty]));
        let id = document["unit"]["types"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["id"]
            .clone();
        let signature = &mut function(&mut document, "fn:top")["signature"];
        signature["owner_type"] = json!("type:u32");
        signature["result"] = id;
        run_facts(vec![document]).unwrap_err()
    };
    assert_eq!(
        error(json!({"id": "t", "kind": "parameter"})),
        "Type parameter t has no index"
    );
    assert_eq!(
        error(json!({"id": "w", "kind": "integer", "signed": true})),
        "Type w has no width"
    );
    assert_eq!(
        error(json!({"id": "r", "kind": "reference", "mode": "shared"})),
        "Type r has no target"
    );
}

#[test]
fn mutated_types_are_the_workspace_types_written_through() {
    let mutated = |ty: &str| {
        let mut document = example();
        nominal(&mut document);
        let symbols = function(&mut document, "fn:count")["symbols"]
            .as_array_mut()
            .unwrap();
        let total = symbols
            .iter_mut()
            .find(|symbol| symbol["id"] == "count.total")
            .unwrap();
        total["type"] = json!(ty);
        facts_of(document)[2].mutated_types.clone()
    };
    assert_eq!(mutated("type:u32"), Vec::<String>::new());
    assert_eq!(mutated("point"), ["geo::Point"]);
    assert_eq!(mutated("&&mut point"), ["geo::Point"]);
    assert_eq!(mutated("&box point"), ["geo::Point"]);
    assert_eq!(mutated("vec"), Vec::<String>::new());
}

#[test]
fn a_mutated_symbol_must_have_a_type() {
    let mut document = example();
    let symbols = function(&mut document, "fn:count")["symbols"]
        .as_array_mut()
        .unwrap();
    let total = symbols
        .iter_mut()
        .find(|symbol| symbol["id"] == "count.total")
        .unwrap();
    total.as_object_mut().unwrap().remove("type");
    assert_eq!(
        run_facts(vec![document]).unwrap_err(),
        "Symbol count.total is mutated but has no type"
    );
}
