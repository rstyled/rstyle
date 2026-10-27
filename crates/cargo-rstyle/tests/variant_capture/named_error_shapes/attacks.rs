//! Resealed bundles still require exact conditional component and value evidence.
use super::*;
const KEY: &str = "rstyle.data:named-error-value-origins";

pub(super) fn reject(entry: &Value) {
    for change in 0..5 {
        let mut bundle = shared_evidence::wire(entry);
        let root = bundle["roots"][shared_evidence::index(entry)]
            .as_u64()
            .unwrap() as usize;
        let nodes = bundle["graphs"][root]["pdg"]["nodes"]
            .as_array_mut()
            .unwrap();
        alter(nodes, change);
        super::super::float_kernels::reject_resealed(
            bundle,
            "named error component/guard mutation",
        );
    }
}

fn alter(nodes: &mut [Value], change: usize) {
    let constructor = nodes
        .iter()
        .any(|node| node["annotations"][KEY]["role"] == "whole_error");
    if constructor {
        constructor_change(nodes, change);
    } else {
        root_change(nodes, change);
    }
}

fn role<'a>(nodes: &'a mut [Value], expected: &str) -> &'a mut Value {
    nodes
        .iter_mut()
        .find(|node| node["annotations"][KEY]["role"] == expected)
        .unwrap()
}

fn constructor_change(nodes: &mut [Value], change: usize) {
    let node = role(nodes, "whole_error");
    let origin = &mut node["annotations"][KEY]["origins"][0];
    match change {
        0 => origin["root"]["parameter"] = 1.into(),
        1 => origin["route"] = serde_json::json!([]),
        2 => origin["route"][0]["value"]["position"] = 1.into(),
        3 => {
            node["annotations"].as_object_mut().unwrap().remove(KEY);
        }
        _ => {
            node["annotations"]["rstyle.data:reference-origins"] = serde_json::json!([]);
        }
    }
}

fn root_change(nodes: &mut [Value], change: usize) {
    if change < 2 {
        let node = role(nodes, "helper_error");
        match change {
            0 => node["annotations"][KEY]["guard"]["variant"] = "Some".into(),
            _ => node["annotations"][KEY]["origins"][0]["root"]["parameter"] = 0.into(),
        }
        return;
    }
    let node = role(nodes, "result");
    match change {
        2 => {
            node["annotations"][KEY]["cases"][0]["origins"] =
                node["annotations"][KEY]["cases"][1]["origins"].clone()
        }
        3 => {
            node["annotations"][KEY]["cases"]
                .as_array_mut()
                .unwrap()
                .pop();
        }
        _ => {
            node["annotations"].as_object_mut().unwrap().remove(KEY);
        }
    }
}
