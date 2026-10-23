//! Independent expectations for the original fresh owned LineReader result.
use super::*;

pub(super) fn check(
    pair: &Pair,
    view: &dyn rstyle_pdg::unit_store::EvidenceFacts,
) -> Result<usize, String> {
    let expected = json!([{"parameter":0,"path":[{"kind":"SharedReferent"}]}]);
    for (construct, roots) in [
        ("data.result-ok", expected.clone()),
        ("data.result-err", json!([])),
    ] {
        let nodes: Vec<_> = pair
            .pdg
            .nodes
            .iter()
            .filter(|node| node.attributes.construct.as_deref() == Some(construct))
            .collect();
        if nodes.len() != 1
            || nodes[0].annotations.get(rstyle_pdg::data_origins::KEY) != Some(&roots)
        {
            return Err(format!(
                "Original LineReader {construct} differs from input/error origin oracle"
            ));
        }
    }
    let none = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.attributes.construct.as_deref() == Some("data.option-none"))
        .count();
    let product = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.original_kind.known_value() == Some("expr_struct"))
        .count();
    if none != 1 || product != 1 {
        return Err("Original LineReader must construct one complete product with None".into());
    }
    verify_attacks(
        pair,
        view,
        &[
            "ok-root",
            "err-root",
            "saved-payload",
            "none",
            "field-slot",
            "policy",
        ],
        forge,
    )
}

fn forge(pair: &Pair, attack: &str) -> Result<Pair, String> {
    let mut forged = Pair {
        pdg: pair.pdg.clone(),
        cfg: pair.cfg.clone(),
    };
    if attack == "policy" {
        forged.pdg.profile.version = "3.88.0".into();
        forged.cfg.profile.version = "3.88.0".into();
        return Ok(forged);
    }
    let node = target(&mut forged, attack)?;
    if attack == "saved-payload" {
        let id = node.id.clone();
        let edge = forged
            .pdg
            .edges
            .iter_mut()
            .find(|edge| edge.target == id && edge.subkind == "value_operand")
            .ok_or("Missing Result saved operand")?;
        edge.source = id;
    } else {
        mutate(node, attack)?;
    }
    Ok(forged)
}

fn target<'a>(pair: &'a mut Pair, attack: &str) -> Result<&'a mut rstyle_pdg::PdgNode, String> {
    pair.pdg
        .nodes
        .iter_mut()
        .find(|node| match attack {
            "err-root" => node.attributes.construct.as_deref() == Some("data.result-err"),
            "none" => node.attributes.construct.as_deref() == Some("data.option-none"),
            "field-slot" => node.original_kind.known_value() == Some("expr_struct"),
            _ => node.attributes.construct.as_deref() == Some("data.result-ok"),
        })
        .ok_or_else(|| format!("Missing {attack} site"))
}

fn mutate(node: &mut rstyle_pdg::PdgNode, attack: &str) -> Result<(), String> {
    match attack {
        "ok-root" => {
            node.annotations
                .insert(rstyle_pdg::data_origins::KEY.into(), json!([]));
        }
        "err-root" => {
            node.annotations.insert(
                rstyle_pdg::data_origins::KEY.into(),
                json!([{"parameter":0,"path":[{"kind":"SharedReferent"}]}]),
            );
        }
        "none" => {
            let annotation = node
                .annotations
                .get_mut("rstyle.data:checked-option")
                .ok_or("Missing checked None annotation")?;
            annotation["has_payload"] = json!(true);
        }
        "field-slot" => {
            node.attributes
                .literal
                .as_mut()
                .ok_or("Missing struct field slots")?
                .value = Some("[1,0]".into());
        }
        _ => return Err("Unknown reader attack".into()),
    }
    Ok(())
}
