mod config;
mod events;
pub(super) use events::Cache;
mod policy;
use crate::clones::{
    Function,
    analysis::{self, Outcome, source_exclusions, source_suppressions},
};
pub(in crate::clones) use config::{Settings, configuration};
use rstyle_pdg::consumer_facts::{self, Availability, effects, mutations, statement_inventory};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Report {
    event_inventory: events::Inventory,
    measurement: policy::Measurement,
    pub complete: bool,
    pub visible_findings: usize,
    pub findings: usize,
    source_exclusion: source_exclusions::Report,
    source_suppression: source_suppressions::Report,
    suppressed: bool,
    invalid_suppression: Option<analysis::Assessment>,
}

pub(super) struct Inputs<'a, 'u> {
    pub function: &'a Function,
    pub assessment: &'a effects::Assessment,
    pub session: &'a effects::Session<'u>,
    pub cache: &'a mut Cache,
    pub statements: &'a statement_inventory::Inventory,
}

pub(super) fn evaluate(
    inputs: Inputs<'_, '_>,
    settings: &Settings,
    root: &std::path::Path,
) -> Result<Report, String> {
    let Inputs {
        function,
        assessment,
        session,
        statements,
        cache,
    } = inputs;
    let observed = session.observations(function.facts.index);
    let inventory = events::inspect(session, assessment, &settings.side_effects.paths, cache)?;
    let original = if observed.is_some() {
        Some(consumer_facts::inspect(&function.facts)?)
    } else {
        None
    };
    let source = original.as_ref().map(|facts| &facts.source);

    let measurement = policy::evaluate(
        &settings.side_effects,
        &inventory.counts,
        source_size(statements, source),
        inventory.coverage == mutations::Coverage::Complete,
    )?;
    let unavailable_source = Availability::Unknown {
        reason: "Density source policy was not inspected".into(),
    };
    let exclusion = source_exclusions::evaluate_source(
        source.unwrap_or(&unavailable_source),
        function,
        root,
        &settings.exclusions,
    );
    let suppression = original
        .as_ref()
        .map(|original| source_suppressions::evaluate(&original.preceding_comment))
        .unwrap_or_else(|| {
            source_suppressions::Report::failure("Density source policy was not inspected".into())
        });

    Ok(finish(inventory, measurement, exclusion, suppression))
}

fn source_size(
    statements: &statement_inventory::Inventory,
    source: Option<&Availability<&str>>,
) -> policy::SourceSize {
    let count = match statements.statements {
        Availability::Known(count) => Some(count),
        _ => None,
    };
    let lines = source.and_then(|source| match source {
        Availability::Known(source) => Some(rstyle_core::lines::code_lines(source)),
        _ => None,
    });
    policy::SourceSize {
        statements: count,
        code_lines: lines,
    }
}

fn finish(
    inventory: events::Inventory,
    measurement: policy::Measurement,
    exclusion: source_exclusions::Report,
    suppression: source_suppressions::Report,
) -> Report {
    let excluded = exclusion.excluded();
    let complete = excluded
        || (measurement.outcome != Outcome::NotEvaluated
            && exclusion.complete()
            && suppression.complete());
    let suppressed = suppression.allows_density();
    let invalid_suppression = if excluded {
        None
    } else {
        suppression.invalid_assessment()
    };
    let finding = measurement.outcome == Outcome::Finding;
    let findings = usize::from(finding) + usize::from(invalid_suppression.is_some());

    Report {
        event_inventory: inventory,
        measurement,
        complete,
        visible_findings: usize::from(finding && !excluded && !suppressed)
            + usize::from(invalid_suppression.is_some()),
        findings,
        source_exclusion: exclusion,
        source_suppression: suppression,
        suppressed,
        invalid_suppression,
    }
}

impl Report {
    pub(super) fn render(
        &self,
        name: &str,
        file: Option<&str>,
        span: rstyle_pdg::ByteSpan,
    ) -> String {
        if self.visible_findings == 0 {
            return String::new();
        }

        let location = format!(
            "{name} at {} [bytes {}..{}]",
            file.unwrap_or("<source unavailable>"),
            span.start,
            span.end
        );
        let mut output = String::new();
        if self.measurement.outcome == Outcome::Finding
            && !self.source_exclusion.excluded()
            && !self.suppressed
        {
            output.push_str(&format!(
                "\nside_effect_density {location}: {} > {} milli (static policy events).",
                self.measurement.actual.unwrap_or(0),
                self.measurement.limit
            ));
        }
        if let Some(invalid) = &self.invalid_suppression {
            output.push_str(&format!(
                "\ninvalid_suppression {location}: {}.",
                invalid.reasons.join(" ")
            ));
        }
        output
    }
}
