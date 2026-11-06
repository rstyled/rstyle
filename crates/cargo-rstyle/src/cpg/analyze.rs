//! A run's report: what core's analyses reported, what its frontend ran itself, and the SARIF
//! they make.
use rstyle_core::cpg::transport::{self, Encoding};
use rstyle_core::filter::{Analysis, Filter};
use rstyle_core::unified::Analyses;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// An analysis a frontend ran itself. Its results stay in their own file, pending until the
/// report can hold them.
#[derive(Serialize)]
pub(crate) struct FrontendResult {
    pub analysis: String,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl FrontendResult {
    fn text(&self) -> String {
        match (&self.file, &self.error) {
            (Some(file), _) => format!("{}: pending in {}", self.analysis, file.display()),
            (None, Some(error)) => format!("{}: failed: {error}", self.analysis),
            (None, None) => format!("{}: {}", self.analysis, self.status),
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

    /// One file per analysis in `directory`; `root` is what their relative paths resolve against.
    pub(crate) fn write_sarif(&self, directory: &Path, root: &Path) -> Result<(), String> {
        for (name, log) in self.analyses.sarif(root)? {
            transport::write_json(&directory.join(name), Encoding::Json, &log)?;
        }
        Ok(())
    }
}
