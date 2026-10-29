//! Root style presentation; the full evidence archive remains available for replay.
use super::*;
use crate::clones::advisory::style::Projection;

#[derive(Serialize)]
struct Style<'a, 'e> {
    #[serde(flatten)]
    evaluation: Projection<'a>,
    compiler_advisory: Option<&'a super::super::advisory::Report>,
    unit_contexts: &'a [super::super::identity::UnitContext],
    observed_units: &'a [capture::Unit],
    build_contexts: &'a [super::super::UnitBuildContexts],
    scope: &'a Scope<'e>,
    run_directory: &'a Path,
    compiler_identity: &'a str,
    cargo_identity: &'a str,
    #[serde(flatten)]
    capture_status: &'a CaptureStatus<'e>,
    #[serde(flatten)]
    strict_execution: &'a StrictExecution,
}

impl Report<'_> {
    pub(in crate::clones) fn publish_style(&self, format: Format) -> Result<String, String> {
        self.publish_with(|| self.render_style(format))
    }

    fn render_style(&self, format: Format) -> Result<String, String> {
        let evaluation = Projection::new(self.compiler_advisory.as_ref())?;
        match format {
            Format::Json => serde_json::to_string(&Style {
                evaluation,
                compiler_advisory: self.compiler_advisory.as_ref(),
                unit_contexts: self.unit_contexts,
                observed_units: self.observed_units,
                build_contexts: self.build_contexts,
                scope: &self.scope,
                run_directory: self.run_directory,
                compiler_identity: self.compiler_identity,
                cargo_identity: self.cargo_identity,
                capture_status: &self.capture_status,
                strict_execution: &self.strict_execution,
            })
            .map_err(|error| error.to_string()),
            Format::Text => Ok(format!(
                "{}\nCompiler evidence and coverage: {}",
                evaluation.text(),
                self.run_directory.join("report.json").display()
            )),
        }
    }
}
