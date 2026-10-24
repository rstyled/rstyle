use super::memory_checks::{KEY, accesses};
use rstyle_pdg::{producer::Pair, unit_store::EvidenceFacts};

pub(super) fn check(pair: &Pair, facts: &dyn EvidenceFacts) {
    for attack in [
        "raw",
        "war",
        "waw",
        "loop",
        "root",
        "snapshot",
        "snapshot-kind",
        "snapshot-missing",
        "definition",
        "write",
        "load-root",
        "exit-kind",
        "exception-edge",
    ] {
        let mut changed = Pair {
            pdg: pair.pdg.clone(),
            cfg: pair.cfg.clone(),
        };
        mutate(&mut changed, attack);
        let sealed = changed.serialize().unwrap();
        assert!(
            rstyle_pdg::verification_policy::strict(|| {
                rstyle_pdg::producer::verify_serialized(&sealed, facts)
            })
            .is_err(),
            "resealed {attack} must fail source-based strict verification"
        );
    }
}

fn mutate(pair: &mut Pair, attack: &str) {
    match attack {
        "raw" | "war" | "waw" | "loop" | "root" => dependencies(pair, attack),
        "snapshot" | "snapshot-kind" | "snapshot-missing" => snapshot(pair, attack),
        "definition" | "write" | "load-root" => effects(pair, attack),
        "exit-kind" => {
            pair.cfg
                .nodes
                .iter_mut()
                .find(|node| node.exit_kind == Some(rstyle_pdg::cfg::CfgExitKind::Exceptional))
                .unwrap()
                .exit_kind = Some(rstyle_pdg::cfg::CfgExitKind::Normal);
        }
        "exception-edge" => {
            pair.cfg
                .edges
                .iter_mut()
                .find(|edge| edge.kind == rstyle_pdg::cfg::CfgEdgeKind::Exception)
                .unwrap()
                .kind = rstyle_pdg::cfg::CfgEdgeKind::Normal;
        }
        _ => unreachable!(),
    }
}

fn dependencies(pair: &mut Pair, attack: &str) {
    match attack {
        "raw" | "war" | "waw" => {
            let subkind = match attack {
                "raw" => "reaching_definition",
                "war" => "memory_war",
                _ => "memory_waw",
            };
            let position = pair
                .pdg
                .edges
                .iter()
                .position(|edge| edge.location_id.is_some() && edge.subkind == subkind)
                .unwrap();
            pair.pdg.edges.remove(position);
        }
        "loop" => {
            let edge = pair
                .pdg
                .edges
                .iter_mut()
                .find(|edge| edge.location_id.is_some() && edge.loop_carried == Some(true))
                .unwrap();
            edge.loop_carried = Some(false);
        }
        "root" => {
            let other = pair.pdg.extensions[KEY]["roots"][0]["location_id"]
                .as_str()
                .unwrap()
                .to_owned();
            let edge = pair
                .pdg
                .edges
                .iter_mut()
                .find(|edge| {
                    edge.location_id.is_some() && edge.location_id.as_ref() != Some(&other)
                })
                .unwrap();
            edge.location_id = Some(other);
        }
        _ => unreachable!(),
    }
}

fn snapshot(pair: &mut Pair, attack: &str) {
    let mut record = pair.pdg.extensions[KEY].clone();
    match attack {
        "snapshot" => {
            record["exits"][0]["reaching"][0]["definitions"]
                .as_array_mut()
                .unwrap()
                .pop();
        }
        "snapshot-kind" => record["exits"][0]["exit_kind"] = "process_termination".into(),
        "snapshot-missing" => {
            record["exits"].as_array_mut().unwrap().pop();
        }
        _ => unreachable!(),
    }
    // Keep the paired extension coherent and recompute both document identities.
    pair.pdg.extensions.insert(KEY.into(), record.clone());
    pair.cfg.extensions.insert(KEY.into(), record);
}

fn effects(pair: &mut Pair, attack: &str) {
    let nodes = accesses(pair);
    let load = nodes[0].id.clone();
    let store = nodes[5].id.clone();
    match attack {
        "definition" => {
            pair.pdg
                .definitions
                .iter_mut()
                .find(|definition| definition.node_id == store)
                .unwrap()
                .location_id = None;
        }
        "write" => {
            pair.pdg
                .nodes
                .iter_mut()
                .find(|node| node.id == store)
                .unwrap()
                .attributes
                .writes
                .clear();
        }
        "load-root" => {
            pair.pdg
                .nodes
                .iter_mut()
                .find(|node| node.id == load)
                .unwrap()
                .attributes
                .position = Some(1);
        }
        _ => unreachable!(),
    }
}
