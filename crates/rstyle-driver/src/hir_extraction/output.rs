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
    pub fn write_source_inventory(&self, value: &serde_json::Value) -> io::Result<()> {
        if let Some(directory) = &self.document_directory {
            std::fs::create_dir_all(directory)?;
            write_json(&directory.join("source-inventory.json"), value)?;
        }
        Ok(())
    }
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
    let result = produce_function(directory, index, facts)?;
    let assessment = assessment(facts, &eligibility, result);

    write_json(
        &directory.join(format!("function-{index}.assessment.json")),
        &assessment,
    )
}

fn produce_function(
    directory: &Path,
    index: usize,
    facts: &HirFnFacts,
) -> io::Result<AssessmentResult> {
    let result = producer::produce(facts)
        .and_then(|pair| rstyle_ccgraph::admit_produced(&pair, facts).map(|_| pair));
    publish_function(directory, index, result)
}

fn publish_function(
    directory: &Path,
    index: usize,
    result: Result<producer::Pair, Vec<String>>,
) -> io::Result<AssessmentResult> {
    let (manifest_file, extraction_errors) = match result {
        Ok(pair) => (Some(write_pair(directory, index, &pair)?), Vec::new()),
        Err(errors) => (None, errors),
    };

    Ok(AssessmentResult {
        manifest_file,
        extraction_errors,
    })
}

fn assessment<'a>(
    facts: &'a HirFnFacts,
    eligibility: &'a rstyle_pdg::HirEligibility,
    result: AssessmentResult,
) -> Assessment<'a> {
    let AssessmentResult {
        manifest_file,
        extraction_errors,
    } = result;
    let validation = if manifest_file.is_some() {
        "validated"
    } else {
        "not_evaluated"
    };
    Assessment {
        function: &facts.name,
        compilation: "successful",
        eligibility: format!("{:?}", eligibility.status),
        normal_flow_validation: validation,
        matching_admission: validation,
        diagnostics: &eligibility.diagnostics,
        extraction_errors,
        manifest_file,
    }
}

struct AssessmentResult {
    manifest_file: Option<String>,
    extraction_errors: Vec<String>,
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
