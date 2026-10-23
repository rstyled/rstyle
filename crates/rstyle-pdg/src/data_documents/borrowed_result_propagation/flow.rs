//! These synthetic operations have exact direct control transfers, without effects between them.
use super::*;

pub(in crate::data_documents) fn verify(
    pair: PairView<'_>,
    operations: &Operations<'_>,
) -> Result<(), String> {
    let Operations {
        selector,
        error,
        success,
        returned,
        ..
    } = *operations;

    let map = crate::read_operation_map(&pair.pdg.extensions)?;

    let operation = |node: &PdgNode| -> Result<String, String> {
        let found: Vec<_> = map
            .iter()
            .filter(|entry| entry.pdg_node_id == node.id)
            .collect();
        match found.as_slice() {
            [entry] => Ok(entry.cfg_node_id.clone()),
            _ => Err("Borrowed Result requires unique real CFG operations".into()),
        }
    };

    let selector = operation(selector)?;

    let error = operation(error)?;

    let success = operation(success)?;

    let returned = operation(returned)?;

    for (source, expected) in [
        (
            &selector,
            vec![
                (error.as_str(), CfgEdgeKind::True),
                (success.as_str(), CfgEdgeKind::False),
            ],
        ),
        (&error, vec![(returned.as_str(), CfgEdgeKind::Normal)]),
    ] {
        let actual: Vec<_> = pair
            .cfg
            .edges
            .iter()
            .filter(|edge| edge.source == *source)
            .map(|edge| (edge.target.as_str(), edge.kind))
            .collect();
        exact_edges(&actual, &expected)?;
    }
    verify_refresh(pair, operations.saved, [&selector, &error, &success])
}

/// Reusing a consumed branch requires a newly completed original input operand.
fn verify_refresh(
    pair: PairView<'_>,
    saved: &PdgNode,
    operations: [&str; 3],
) -> Result<(), String> {
    let frontier = super::super::callbacks::order::completion(pair, saved)?;
    let mut outgoing = BTreeMap::<&str, Vec<&str>>::new();
    for edge in &pair.cfg.edges {
        outgoing.entry(&edge.source).or_default().push(&edge.target);
    }

    for operation in operations {
        if revisits_without_refresh(&outgoing, operation, &frontier) {
            return Err(
                "Borrowed Result payload/tag can repeat without a newly completed input operand"
                    .into(),
            );
        }
    }
    Ok(())
}

fn revisits_without_refresh(
    outgoing: &BTreeMap<&str, Vec<&str>>,
    operation: &str,
    frontier: &BTreeSet<String>,
) -> bool {
    let mut pending = outgoing.get(operation).cloned().unwrap_or_default();
    let mut seen = BTreeSet::new();

    while let Some(node) = pending.pop() {
        if frontier.contains(node) || !seen.insert(node) {
            continue;
        }
        if node == operation {
            return true;
        }

        pending.extend(outgoing.get(node).into_iter().flatten().copied());
    }
    false
}

fn exact_edges(
    actual: &[(&str, CfgEdgeKind)],
    expected: &[(&str, CfgEdgeKind)],
) -> Result<(), String> {
    if actual.len() != expected.len()
        || expected
            .iter()
            .any(|edge| actual.iter().filter(|actual| *actual == edge).count() != 1)
    {
        return Err(
            "Borrowed Result branch bypasses a selected payload or its mandatory return".into(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enclosing_loop_is_allowed_only_after_completed_input_refresh() {
        let outgoing = BTreeMap::from([
            ("saved", vec!["selector"]),
            ("selector", vec!["error", "success"]),
            ("error", vec!["return"]),
            ("success", vec!["loop_header"]),
            ("loop_header", vec!["saved", "exit"]),
        ]);
        let frontier = BTreeSet::from(["saved".into()]);
        for operation in ["selector", "error", "success"] {
            assert!(!revisits_without_refresh(&outgoing, operation, &frontier));
        }
        for target in ["selector", "success"] {
            let mut bypass = outgoing.clone();
            bypass.get_mut("success").unwrap().push(target);
            assert!(
                revisits_without_refresh(&bypass, target, &frontier),
                "{target}"
            );
        }
    }

    #[test]
    fn one_fresh_branch_does_not_hide_an_alternate_stale_cycle() {
        let outgoing = BTreeMap::from([
            ("success", vec!["saved", "stale"]),
            ("saved", vec!["selector"]),
            ("selector", vec!["success"]),
            ("stale", vec!["success"]),
        ]);
        assert!(revisits_without_refresh(
            &outgoing,
            "success",
            &BTreeSet::from(["saved".into()])
        ));
    }

    #[test]
    fn branches_reject_extra_missing_duplicate_or_reversed_transfers() {
        let expected = [
            ("error", CfgEdgeKind::True),
            ("success", CfgEdgeKind::False),
        ];
        exact_edges(&expected, &expected).unwrap();

        for actual in [
            vec![("error", CfgEdgeKind::True)],
            vec![("error", CfgEdgeKind::True), ("success", CfgEdgeKind::True)],
            vec![
                ("error", CfgEdgeKind::False),
                ("success", CfgEdgeKind::True),
            ],
            vec![("error", CfgEdgeKind::True), ("error", CfgEdgeKind::True)],
            vec![
                ("error", CfgEdgeKind::True),
                ("success", CfgEdgeKind::False),
                ("exit", CfgEdgeKind::Normal),
            ],
        ] {
            assert!(exact_edges(&actual, &expected).is_err());
        }
    }

    #[test]
    fn extracted_error_cannot_bypass_or_duplicate_its_return() {
        let expected = [("return", CfgEdgeKind::Normal)];
        exact_edges(&expected, &expected).unwrap();

        for actual in [
            vec![],
            vec![("success", CfgEdgeKind::Normal)],
            vec![("return", CfgEdgeKind::Return)],
            vec![
                ("return", CfgEdgeKind::Normal),
                ("exit", CfgEdgeKind::Normal),
            ],
            vec![
                ("return", CfgEdgeKind::Normal),
                ("return", CfgEdgeKind::Normal),
            ],
        ] {
            assert!(exact_edges(&actual, &expected).is_err());
        }
    }
}
