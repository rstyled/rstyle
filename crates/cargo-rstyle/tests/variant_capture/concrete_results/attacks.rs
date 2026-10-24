use super::*;

pub(super) fn reject_constructor_attacks(entry: &Value) {
    for attack in [
        "crate",
        "crate_hash",
        "definition_hash",
        "path",
        "payload",
        "type",
        "order",
        "tag",
        "effect",
    ] {
        reject_resealed(entry, |bundle, index| {
            if matches!(attack, "order" | "tag" | "effect") {
                corrupt_graph(bundle, index, attack);
            } else {
                corrupt_source(bundle, index, attack);
            }
        });
    }
    for field in ["target_triple", "rustc_commit", "pointer_width"] {
        reject_resealed(entry, |bundle, index| {
            bundle["functions"][index]["build"][field]["value"] = if field == "pointer_width" {
                32.into()
            } else {
                "untrusted".into()
            };
        });
    }
    reject_policy(entry);
}

fn corrupt_source(bundle: &mut Value, index: usize, attack: &str) {
    let node = variant_mut(&mut bundle["functions"][index]["body"]).unwrap();
    if attack == "payload" {
        let child = node["children"][0].clone();
        node["children"].as_array_mut().unwrap().push(child);
    } else if attack == "type" {
        let token = node["ty"]["normalized"].as_str().unwrap();
        let mut token: Value = serde_json::from_str(token).unwrap();
        token["value"]["err"]["value"]["nominal"]["definition_path"] =
            "clone_fixture::WrongError".into();
        node["ty"]["normalized"] = serde_json::to_string(&token).unwrap().into();
    } else {
        let mut authority: Value =
            serde_json::from_str(node["res"]["description"].as_str().unwrap()).unwrap();
        authority[attack] = "wrong-core-authority".into();
        node["res"]["description"] = serde_json::to_string(&authority).unwrap().into();
    }
}

fn corrupt_graph(bundle: &mut Value, index: usize, attack: &str) {
    let graph = bundle["roots"][index].as_u64().unwrap() as usize;
    let pdg = &mut bundle["graphs"][graph]["pdg"];
    if attack == "order" {
        let edge = pdg["edges"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|edge| edge["subkind"] == "value_operand")
            .unwrap();
        edge["operand_position"] = 1.into();
        return;
    }
    let node = pdg["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| {
            matches!(
                node["attributes"]["construct"].as_str(),
                Some("data.result-ok" | "data.result-err")
            )
        })
        .unwrap();
    if attack == "tag" {
        let was_ok = node["attributes"]["construct"] == "data.result-ok";
        node["attributes"]["construct"] = if was_ok {
            "data.result-err"
        } else {
            "data.result-ok"
        }
        .into();
        node["annotations"]["rstyle.data:concrete-result"]["variant"] =
            if was_ok { "err" } else { "ok" }.into();
    } else {
        node["attributes"]["symbol_ids"] = serde_json::json!(["invented-effect"]);
    }
}

pub(super) fn reject_policy(entry: &Value) {
    reject_resealed(entry, |bundle, index| {
        let graph = bundle["roots"][index].as_u64().unwrap() as usize;
        bundle["graphs"][graph]["pdg"]["profile"]["version"] = "3.77.0".into();
        bundle["graphs"][graph]["cfg"]["profile"]["version"] = "3.77.0".into();
    });
}
