//! Frozen source/contract expectations, declared before evaluating the selection.
use rstyle_ccgraph::{CcgraphConfig, Decision, ImportedFunction, MatchingProfile};
use rstyle_pdg::{producer::Pair, unit_store::EvidenceFacts};

pub(super) fn admit(
    pair: &Pair,
    facts: &dyn EvidenceFacts,
    key: String,
) -> Result<ImportedFunction, String> {
    let mut imported =
        rstyle_ccgraph::admit_produced(pair, facts).map_err(|errors| errors.join("; "))?;
    imported.key = key;
    if imported.matching_profile != MatchingProfile::DataCallsV1 {
        return Err("Selected constructor lost its registered DataCalls matching profile".into());
    }

    Ok(imported)
}

pub(super) fn check(imports: Vec<ImportedFunction>) -> Result<(), String> {
    if imports.len() != 4 {
        return Err("Expected original/replay imports for both selected substitutions".into());
    }
    distinct_interfaces(&imports)?;

    // Both bodies have the same retained category/dependency topology. CALL
    // labels omit callee/construct/type; identical name/return strings alone
    // guarantee a string mean >= 2/3, exceeding the unchanged default 1/2.
    // Range still requires independent consuming authority during admission.
    let report = rstyle_ccgraph::evaluate_all(&imports, &CcgraphConfig::default());
    let pairs: Vec<_> = report
        .pairs
        .iter()
        .map(|pair| {
            serde_json::json!({
                "first": pair.first, "second": pair.second,
                "decision": pair.decision, "score": pair.displayed_score,
            })
        })
        .collect();
    println!(
        "Selected matching report: {}",
        serde_json::json!({
            "profile": report.profile_id, "functions": report.functions.len(),
            "incomplete": report.incomplete, "pairs": pairs,
        })
    );
    if report.incomplete
        || report.functions.len() != 4
        || report.functions.iter().any(|function| !function.eligible)
        || report.pairs.len() != 6
        || report
            .pairs
            .iter()
            .any(|pair| pair.decision != Decision::Accepted)
    {
        return Err("Selected matching differs from predeclared six Accepted pairs".into());
    }

    Ok(())
}

fn distinct_interfaces(imports: &[ImportedFunction]) -> Result<(), String> {
    if imports[0].inputs != imports[1].inputs
        || imports[2].inputs != imports[3].inputs
        || imports[0].inputs == imports[2].inputs
        || imports
            .iter()
            .any(|function| function.outputs != imports[0].outputs)
    {
        return Err("Selected substitutions lost distinct inputs or stable replay/results".into());
    }

    Ok(())
}
