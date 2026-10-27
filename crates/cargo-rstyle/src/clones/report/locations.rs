//! Navigation over accepted decisions, separate from matching authority.
use super::super::{Function, Handoff};
use rstyle_ccgraph::{Decision, RunReport};
use rstyle_pdg::{ByteSpan, nominal_boundary::census::Census};
use serde::Serialize;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Serialize)]
pub(super) struct AcceptedPair<'a> {
    pub first: Location<'a>,
    pub second: Location<'a>,
    pub displayed_score: Option<&'a str>,
}

#[derive(Serialize)]
pub(super) struct Location<'a> {
    pub key: &'a str,
    pub name: &'a str,
    pub path: Option<&'a str>,
    pub span: ByteSpan,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

pub(super) fn accepted<'a>(
    handoff: &'a Handoff,
    matching: &'a RunReport,
) -> Result<Vec<AcceptedPair<'a>>, String> {
    let functions: BTreeMap<_, _> = handoff
        .assessed
        .iter()
        .map(|function| (function.key.to_string(), function))
        .collect();
    matching
        .pairs
        .iter()
        .filter(|pair| pair.decision == Decision::Accepted)
        .map(|pair| {
            Ok(AcceptedPair {
                first: locate(&pair.first, &functions)?,
                second: locate(&pair.second, &functions)?,
                displayed_score: pair.displayed_score.as_deref(),
            })
        })
        .collect()
}

fn locate<'a>(
    key: &'a str,
    functions: &BTreeMap<String, &'a Function>,
) -> Result<Location<'a>, String> {
    let function = functions
        .get(key)
        .ok_or_else(|| format!("Accepted pair references absent function {key}"))?;
    let facts = &function.facts;
    let path = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(|v| v.as_str());
    let position = position(facts, path);
    Ok(Location {
        key,
        name: &facts.name,
        path,
        span: facts.fn_span,
        line: position.map(|(line, _)| line),
        column: position.map(|(_, column)| column),
    })
}

fn position(facts: &rstyle_pdg::HirFnFacts, path: Option<&str>) -> Option<(usize, usize)> {
    let census = facts.build.frontend_options.data::<Arc<Census>>().ok()?;
    let file = census
        .files
        .iter()
        .find(|file| file.path.to_str() == path)?;
    let start = facts.fn_span.start as usize;
    if file.bytes.get(start..facts.fn_span.end as usize)? != facts.source.bytes.as_str() {
        return None;
    }
    let prefix = file.bytes.get(..start)?;
    Some((
        prefix.bytes().filter(|byte| *byte == b'\n').count() + 1,
        prefix.rsplit('\n').next()?.chars().count() + 1,
    ))
}

impl std::fmt::Display for Location<'_> {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            output,
            "{} {} at {}",
            self.key,
            self.name,
            self.path.unwrap_or("<source unavailable>")
        )?;
        match (self.line, self.column) {
            (Some(line), Some(column)) => write!(output, ":{line}:{column}"),
            _ => write!(output, " [bytes {}..{}]", self.span.start, self.span.end),
        }
    }
}
