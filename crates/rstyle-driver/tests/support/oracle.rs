//! Independent fixture facts: no producer analysis is used to create expectations.
use rstyle_pdg::{
    R0FnFacts,
    producer::{Graph, analyze},
};
use rstyle_r0fixture::fixture::FixtureCase;
use std::collections::{BTreeMap, BTreeSet};

struct Names<'a> {
    aliases: &'a BTreeMap<String, String>,
    symbols: &'a BTreeMap<String, String>,
    definitions: &'a BTreeMap<String, String>,
}

pub fn check_bindings(
    case: &FixtureCase,
    facts: &R0FnFacts,
    graph: &Graph,
    aliases: &BTreeMap<String, String>,
) {
    let expected = case.expected.symbols.as_deref().unwrap_or_default();
    assert_eq!(
        facts.bindings.len(),
        expected.len(),
        "{} symbol inventory",
        case.id
    );
    let mut symbols = BTreeMap::new();
    for binding in &facts.bindings {
        let oracle = expected
            .iter()
            .find(|s| {
                s.4.resolve(&case.source)
                    == Some((
                        binding.decl_span.start as usize,
                        binding.decl_span.end as usize,
                    ))
            })
            .expect("declaring-token correspondence");
        assert_eq!(binding.name, oracle.1);
        let expected_type = if oracle.2 == "()" { "unit" } else { &oracle.2 };
        assert_eq!(binding.ty.normalized.as_deref(), Some(expected_type));
        assert_eq!(binding.is_param, oracle.3 == "parameter");
        assert!(!binding.scope.is_empty());
        symbols.insert(binding.symbol.clone(), oracle.0.clone());
    }
    let definitions = definition_names(case, graph, aliases, &symbols);
    check_sets(
        case,
        graph,
        &Names {
            aliases,
            symbols: &symbols,
            definitions: &definitions,
        },
    );
    check_attributes(case, graph, aliases);
}

fn definition_names(
    case: &FixtureCase,
    graph: &Graph,
    aliases: &BTreeMap<String, String>,
    symbols: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let expected = case.expected.definitions.as_deref().unwrap_or_default();
    let actual: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| node.definition.is_some())
        .collect();
    assert_eq!(actual.len(), expected.len(), "{} definitions", case.id);
    actual
        .iter()
        .map(|node| {
            let oracle = expected
                .iter()
                .find(|d| d.2 == aliases[&node.id])
                .expect("definition operation");
            assert_eq!(symbols[node.symbol.as_ref().expect("binding")], oracle.1);
            (node.definition.clone().unwrap(), oracle.0.clone())
        })
        .collect()
}

fn check_sets(case: &FixtureCase, graph: &Graph, names: &Names<'_>) {
    let analysis = analyze(graph).expect("actual reaching sets");
    for (read, expected) in case
        .expected
        .read_definitions
        .as_ref()
        .into_iter()
        .flatten()
    {
        let node = graph
            .nodes
            .iter()
            .find(|n| names.aliases[&n.id] == *read)
            .unwrap();
        let actual = analysis
            .definition_in
            .get(&node.id)
            .and_then(|set| set.get(node.symbol.as_ref().unwrap()));
        let actual: BTreeSet<_> = actual
            .into_iter()
            .flatten()
            .map(|id| names.definitions[id].clone())
            .collect();
        assert_eq!(
            actual,
            expected.iter().cloned().collect(),
            "{} read {read}",
            case.id
        );
    }
    check_local_sets(case, graph, names, &analysis);
}

fn check_local_sets(
    case: &FixtureCase,
    graph: &Graph,
    names: &Names<'_>,
    analysis: &rstyle_pdg::producer::Analysis,
) {
    if let Some(sets) = &case.expected.local_definition_sets {
        for (expected, actual) in [
            (&sets.inn, &analysis.definition_in),
            (&sets.out, &analysis.definition_out),
        ] {
            for (operation, bindings) in expected {
                let node = graph
                    .nodes
                    .iter()
                    .find(|n| names.aliases[&n.id] == *operation)
                    .unwrap();
                for (symbol, expected) in bindings {
                    let id = names
                        .symbols
                        .iter()
                        .find(|(_, name)| *name == symbol)
                        .unwrap()
                        .0;
                    let found: BTreeSet<_> = actual
                        .get(&node.id)
                        .and_then(|set| set.get(id))
                        .into_iter()
                        .flatten()
                        .map(|id| names.definitions[id].clone())
                        .collect();
                    assert_eq!(
                        found,
                        expected.iter().cloned().collect(),
                        "{} {operation}/{symbol}",
                        case.id
                    );
                }
            }
        }
    }
}

fn check_attributes(case: &FixtureCase, graph: &Graph, aliases: &BTreeMap<String, String>) {
    for (expected, select) in [
        (
            &case.expected.literal_attributes,
            (|n: &rstyle_pdg::producer::Operation| n.literal.as_deref())
                as fn(&rstyle_pdg::producer::Operation) -> Option<&str>,
        ),
        (
            &case.expected.operator_attributes,
            (|n: &rstyle_pdg::producer::Operation| n.operator.as_deref())
                as fn(&rstyle_pdg::producer::Operation) -> Option<&str>,
        ),
        (
            &case.expected.source_operation_roles,
            (|n: &rstyle_pdg::producer::Operation| Some(n.role.as_str()))
                as fn(&rstyle_pdg::producer::Operation) -> Option<&str>,
        ),
    ] {
        for (alias, value) in expected.as_ref().into_iter().flatten() {
            let node = graph
                .nodes
                .iter()
                .find(|n| aliases[&n.id] == *alias)
                .unwrap();
            assert_eq!(select(node), Some(value.as_str()), "{} {alias}", case.id);
        }
    }
}
