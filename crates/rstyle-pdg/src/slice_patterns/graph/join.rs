use super::*;
use serde_json::Value;

pub(super) fn verify(
    pair: PairView<'_>,
    checked: &Checked<'_>,
    replay: &Replay<'_>,
) -> Result<(), String> {
    let merge = value_operation(pair, checked.matched())?;
    let success = value_operation(pair, checked.success())?;
    if !replay.none.attributes.reads.is_empty()
        || !replay.none.attributes.writes.is_empty()
        || !replay.none.attributes.defines.is_empty()
    {
        return Err("Prefix None must retain its explicit empty reference/effect case".into());
    }
    verify_saved_operands(pair, replay.none, &[])?;
    let site =
        crate::data_documents::extension_record(pair, "compat.rust.hir-r0:value-sites", &merge.id)?;
    let alternatives = site["alternatives"]
        .as_array()
        .ok_or("Missing prefix alternatives")?;
    if merge.category != NodeCategory::Merge
        || alternatives.len() != 2
        || site["controller_pdg_node_id"] != replay.length.id
        || site["rstyle.hir-coverage:selection-kind"] != "ordered-match/1"
    {
        return Err(
            "Prefix match loses its original length controller or complete alternatives".into(),
        );
    }
    verify_saved_operands(pair, merge, &[])?;
    for (outcome, value) in [("arm:0", success), ("arm:1", replay.none)] {
        alternative(pair, alternatives, (outcome, value), replay)?;
    }

    crate::data_documents::checked_roles::alternative_edges(
        pair,
        merge,
        [(success, "arm:0"), (replay.none, "arm:1")],
    )
}

fn alternative(
    pair: PairView<'_>,
    alternatives: &[Value],
    expected: (&str, &PdgNode),
    replay: &Replay<'_>,
) -> Result<(), String> {
    let matches: Vec<_> = alternatives
        .iter()
        .filter(|alternative| {
            alternative["outcome"] == expected.0
                && alternative["value_pdg_node_id"] == expected.1.id
        })
        .collect();
    let [alternative] = matches.as_slice() else {
        return Err("Prefix join changes its original branch value".into());
    };
    let cfg = flow::Flow::new(pair)?;
    let completion = vec![cfg.operation(expected.1)?];
    if alternative["completion_cfg_node_ids"] != serde_json::json!(completion) {
        return Err("Prefix alternative completion changes its real body result frontier".into());
    }
    let actual: Vec<Vec<crate::producer::Atom>> =
        serde_json::from_value(alternative["annotations"]["compat.rust.hir-r0:guards"].clone())
            .map_err(|error| error.to_string())?;
    if canonical(actual) != canonical(super::guards(replay, expected.0 == "arm:0")) {
        return Err("Prefix alternative guards disagree with ordered selector outcomes".into());
    }

    Ok(())
}

fn canonical(mut terms: Vec<Vec<crate::producer::Atom>>) -> Vec<Vec<crate::producer::Atom>> {
    for term in &mut terms {
        term.sort();
    }
    terms.sort();
    terms
}
