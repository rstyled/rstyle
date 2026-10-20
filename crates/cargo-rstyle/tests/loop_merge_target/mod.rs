//! Source-selected expectations precede loop/merge support.
use super::*;

mod outside;
mod tuple_cases;

fn natural_gcd() -> &'static str {
    let source = include_str!("../../../rstyle-ccgraph/src/frac.rs");
    let start = source.find("fn gcd(").unwrap();
    let end = source[start..].find("\nfn split_decimal(").unwrap();
    source[start..start + end].trim()
}

#[test]
fn loop_merge_target_preserves_connected_classifications() {
    let directory = clone_workspace();
    let source = format!(
        "{}\n{}",
        include_str!("../../../../eval/coverage-development/loop-merges.rs"),
        natural_gcd()
    );
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{report}");
    let functions = report["functions"].as_array().unwrap();
    assert_eq!(functions.len(), 7, "{report}");
    check_target_classifications(functions);
    check_aggregate_documents(&report);
    let natural = functions
        .iter()
        .find(|function| function["facts"]["name"] == "gcd")
        .unwrap();
    assert_eq!(natural["outcome"], "admitted");
    let step = functions
        .iter()
        .find(|function| function["facts"]["name"] == "tuple_step")
        .unwrap();
    check_tuple_snapshots(step);
}

fn check_target_classifications(functions: &[Value]) {
    for function in functions {
        let name = function["facts"]["name"].as_str().unwrap();
        assert_eq!(
            function["outcome"],
            if matches!(
                name,
                "sequential" | "tuple_step" | "gcd" | "after_loop" | "before_loop"
            ) {
                "admitted"
            } else {
                "excluded"
            },
            "{function}"
        );
        if name == "cyclic_merge" {
            assert!(
                function["reasons"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|reason| { reason.as_str().unwrap().contains("cyclic policy excludes") }),
                "{function}"
            );
        }
    }
}

#[test]
fn loop_merge_target_has_independent_runtime_expectations() {
    let directory = tempfile::tempdir().unwrap();
    let source = format!(
        "{}\n{}\n{}",
        include_str!("../../../../eval/coverage-development/loop-merges.rs"),
        natural_gcd(),
        include_str!("../../../../eval/coverage-development/loop-merge-oracle.rs")
            .lines()
            .filter(|line| !line.starts_with("//!"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let path = directory.path().join("oracle.rs");
    let binary = directory.path().join("oracle");
    std::fs::write(&path, source).unwrap();
    let output = Command::new("rustc")
        .args(["--edition=2021", "--test", "-C", "overflow-checks=on"])
        .arg(&path)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = Command::new(binary).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

fn check_tuple_snapshots(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = shared_evidence::pair(&unit, function);
    let rhs = &facts.body.children[0].children[0].children[1];
    check_tuple_writes(pair, rhs);
    check_original_reads(pair, rhs);
    reject_swapped_writes(function);
}

fn check_tuple_writes(pair: &rstyle_pdg::producer::Pair, rhs: &rstyle_pdg::HirNode) {
    let writes: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.category == rstyle_pdg::NodeCategory::Assignment)
        .collect();
    assert_eq!(writes.len(), 2);
    for (write, value) in writes.iter().zip(&rhs.children) {
        let edge = pair
            .pdg
            .edges
            .iter()
            .find(|edge| edge.target == write.id && edge.subkind == "value_operand")
            .unwrap();
        let producer = pair
            .pdg
            .nodes
            .iter()
            .find(|node| node.id == edge.source)
            .unwrap();
        let span = producer.source_span.as_ref().unwrap();
        assert_eq!(
            (span.start, span.end),
            (u64::from(value.span.start), u64::from(value.span.end))
        );
    }
}

fn check_original_reads(pair: &rstyle_pdg::producer::Pair, rhs: &rstyle_pdg::HirNode) {
    let original_reads: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| {
            node.category == rstyle_pdg::NodeCategory::Read
                && node.source_span.as_ref().is_some_and(|span| {
                    span.start >= u64::from(rhs.span.start) && span.end <= u64::from(rhs.span.end)
                })
        })
        .collect();
    assert_eq!(original_reads.len(), 3);
    for read in original_reads {
        let edges: Vec<_> = pair
            .pdg
            .edges
            .iter()
            .filter(|edge| edge.target == read.id && edge.subkind == "reaching_definition")
            .collect();
        assert_eq!(edges.len(), 1);
        assert!(pair.pdg.nodes.iter().any(|node| node.id == edges[0].source
            && node.category == rstyle_pdg::NodeCategory::FormalInput));
    }
}

fn reject_swapped_writes(function: &Value) {
    let (unit, index) = data_unit(function);
    let analysis = unit.analysis();
    let facts = analysis.function(index).unwrap();
    let pair = shared_evidence::pair(&unit, function);
    let writes: Vec<_> = pair
        .pdg
        .nodes
        .iter()
        .filter(|node| node.category == rstyle_pdg::NodeCategory::Assignment)
        .collect();
    let mut changed = rstyle_pdg::producer::produce(&facts).unwrap();
    let operands: Vec<_> = changed
        .pdg
        .edges
        .iter_mut()
        .filter(|edge| {
            edge.subkind == "value_operand" && writes.iter().any(|write| write.id == edge.target)
        })
        .collect();
    assert_eq!(operands.len(), 2);
    let sources: Vec<_> = operands
        .iter()
        .map(|edge| edge.source.clone())
        .rev()
        .collect();
    for (edge, source) in operands.into_iter().zip(sources) {
        edge.source = source;
    }
    assert!(rstyle_pdg::producer::verify(&changed, &facts).is_err());
    assert!(rstyle_ccgraph::admit_produced(&changed, &facts).is_err());
}
