//! Explicit producer artifacts, independent of matching orchestration.

use rstyle_pdg::{HirFnFacts, classify, producer};
use serde::Serialize;
use std::{
    io,
    path::{Path, PathBuf},
};

pub struct Destination {
    pub facts_file: PathBuf,
    pub document_directory: Option<PathBuf>,
}

#[derive(Serialize)]
struct Assessment<'a> {
    function: &'a str,
    compilation: &'static str,
    eligibility: String,
    normal_flow_validation: &'static str,
    matching_admission: &'static str,
    diagnostics: &'a [rstyle_pdg::HirDiagnostic],
    extraction_errors: Vec<String>,
    manifest_file: Option<String>,
}

impl Destination {
    pub fn write(&self, facts: &[HirFnFacts]) -> io::Result<()> {
        if let Some(directory) = &self.document_directory {
            write(directory, facts)?;
        }
        write_json(&self.facts_file, facts)
    }
}

fn write(directory: &Path, facts: &[HirFnFacts]) -> io::Result<()> {
    std::fs::create_dir_all(directory)?;
    for (index, function) in facts.iter().enumerate() {
        write_function(directory, index, function)?;
    }

    Ok(())
}

fn write_function(directory: &Path, index: usize, facts: &HirFnFacts) -> io::Result<()> {
    let eligibility = classify(facts);
    let result = producer::produce(facts)
        .and_then(|pair| rstyle_ccgraph::admit_produced(&pair, facts).map(|_| pair));
    let (manifest_file, extraction_errors) = match result {
        Ok(pair) => (Some(write_pair(directory, index, &pair)?), Vec::new()),
        Err(errors) => (None, errors),
    };
    let validation = if manifest_file.is_some() {
        "validated"
    } else {
        "not_evaluated"
    };
    let assessment = Assessment {
        function: &facts.name,
        compilation: "successful",
        eligibility: format!("{:?}", eligibility.status),
        normal_flow_validation: validation,
        matching_admission: validation,
        diagnostics: &eligibility.diagnostics,
        extraction_errors,
        manifest_file,
    };

    write_json(
        &directory.join(format!("function-{index}.assessment.json")),
        &assessment,
    )
}

fn write_pair(directory: &Path, index: usize, pair: &producer::Pair) -> io::Result<String> {
    let serialized = pair.serialize().map_err(io::Error::other)?;
    std::fs::write(
        directory.join(format!("function-{index}.pdg.json")),
        serialized.pdg_bytes,
    )?;
    std::fs::write(
        directory.join(format!("function-{index}.cfg.json")),
        serialized.cfg_bytes,
    )?;
    let manifest = format!("function-{index}.manifest.json");
    write_json(&directory.join(&manifest), &serialized.manifest)?;

    Ok(manifest)
}

fn write_json(path: &Path, value: &(impl Serialize + ?Sized)) -> io::Result<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;

    std::fs::write(path, bytes)
}
