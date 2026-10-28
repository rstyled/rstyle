use super::super::baseline_io;
use super::model::Baseline;
use std::path::Path;

impl baseline_io::Artifact for Baseline {
    const LABEL: &'static str = "Density baseline";

    fn validate(&self) -> Result<(), String> {
        Baseline::validate(self)
    }
}

pub(super) fn load(path: &Path) -> Result<Baseline, String> {
    baseline_io::load(path)
}

#[cfg(test)]
mod tests;
