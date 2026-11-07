//! A run's report: what core's analyses reported, what its frontend ran itself, and the SARIF
//! they make.
use rstyle_core::cpg::transport::{self, Encoding};
use rstyle_core::filter::{Analysis, Filter};
use rstyle_core::sarif;
use rstyle_core::unified::Analyses;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};

const REPORT_FILE: &str = "report.sarif";

/// An analysis a frontend ran itself. Its results also stay in a file of their own, as the
/// frontend wrote them.
#[derive(Serialize)]
pub(crate) struct FrontendResult {
    pub analysis: String,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub runs: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl FrontendResult {
    fn results(&self) -> usize {
        self.runs
            .iter()
            .filter_map(|run| run["results"].as_array())
            .map(Vec::len)
            .sum()
    }

    fn text(&self) -> String {
        if let Some(error) = &self.error {
            return format!("{}: failed: {error}", self.analysis);
        }
        let results = self.results();
        let plural = if results == 1 { "" } else { "s" };
        match &self.file {
            Some(file) => format!(
                "{}: {results} result{plural}, in {}",
                self.analysis,
                file.display()
            ),
            None => format!("{}: {results} result{plural}", self.analysis),
        }
    }
}

/// A run's report. Nothing here says where or when the run happened, so two runs over the same
/// source are byte-comparable.
#[derive(Serialize)]
pub(crate) struct Unified {
    #[serde(flatten)]
    pub analyses: Analyses,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub frontend: Vec<FrontendResult>,
}

impl From<Analyses> for Unified {
    fn from(analyses: Analyses) -> Self {
        Self {
            analyses,
            frontend: Vec::new(),
        }
    }
}

/// Fails before anything runs when the flags can't both be honored.
pub(crate) fn validate(filter: &Filter, fail_on_candidates: bool) -> Result<(), String> {
    if fail_on_candidates && !filter.core(Analysis::Patterns) {
        return Err(
            "--fail-on-candidates needs the patterns analysis, which the filter leaves out".into(),
        );
    }
    Ok(())
}

impl Unified {
    pub(crate) fn text(&self) -> String {
        let analyses = Some(self.analyses.text()).filter(|text| !text.is_empty());
        let frontend = self.frontend.iter().map(FrontendResult::text);
        analyses
            .into_iter()
            .chain(frontend)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The worst result of everything that ran: 0 clean, 1 findings, 2 incomplete.
    pub(crate) fn exit_code(&self, fail_on_candidates: bool) -> u8 {
        let frontend = self.frontend.iter().any(|result| result.error.is_some());
        let frontend = if frontend { 2 } else { 0 };
        self.analyses.exit_code(fail_on_candidates).max(frontend)
    }

    /// One file per core analysis in `directory`, and `report.sarif` holding every analysis's
    /// runs. `root` is what relative paths resolve against.
    // rstyle-allow(side_effect_density): IO boundary: writes the SARIF files
    pub(crate) fn write_sarif(&self, directory: &Path, root: &Path) -> Result<(), String> {
        let mut runs = Vec::new();

        for (name, log) in self.analyses.sarif(root)? {
            transport::write_json(&directory.join(name), Encoding::Json, &log)?;
            let log = serde_json::to_value(&log).map_err(|error| error.to_string())?;
            runs.extend(log["runs"].as_array().into_iter().flatten().cloned());
        }
        runs.extend(self.frontend.iter().flat_map(|result| result.runs.clone()));
        if runs.is_empty() {
            return Ok(());
        }
        let report = directory.join(REPORT_FILE);
        transport::write_json(&report, Encoding::Json, &sarif::combined(runs)).map(|_| ())
    }
}
