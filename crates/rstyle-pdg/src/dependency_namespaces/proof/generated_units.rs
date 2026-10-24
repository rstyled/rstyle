//! Complete bounded source projection; compiler ownership and exports join later.
//! No generic method, constant value, callable effect or cleanup is certified here.
mod invocation;
mod syntax;
mod templates;
#[cfg(test)]
pub(super) mod tests;

use super::ProofError;
use crate::{
    SourceSnapshot,
    nominal_boundary::census::{self, Census, Module, UnitFile, expansions::FileSpan},
};
use invocation::Invocation;
pub(super) use invocation::Unit;
use syn::spanned::Spanned;
pub(super) use templates::TemplateRole;

/// An independently checked source footprint, never accepted from serialized data.
pub(super) struct Projection {
    namespace: Vec<String>,
    macro_name: String,
    source: SourceSnapshot,
    definition: FileSpan,
    full_definition: FileSpan,
    invocation: FileSpan,
    units: Vec<Unit>,
    templates: Vec<(TemplateRole, FileSpan)>,
}

impl Projection {
    pub(super) fn namespace(&self) -> &[String] {
        &self.namespace
    }
    pub(super) fn macro_name(&self) -> &str {
        &self.macro_name
    }
    pub(super) fn source(&self) -> &SourceSnapshot {
        &self.source
    }
    pub(super) fn definition(&self) -> &FileSpan {
        &self.definition
    }
    pub(super) fn full_definition(&self) -> &FileSpan {
        &self.full_definition
    }
    pub(super) fn invocation(&self) -> &FileSpan {
        &self.invocation
    }
    pub(super) fn units(&self) -> &[Unit] {
        &self.units
    }
    pub(super) fn template(&self, role: TemplateRole) -> &FileSpan {
        &self
            .templates
            .iter()
            .find(|(key, _)| *key == role)
            .unwrap()
            .1
    }
}

/// `modules` is the complete already-checked active ordinary module set. The
/// original complete selected file is parsed again, so inactive or extra syntax
/// cannot disappear merely because an active item list omitted it.
pub(super) fn verify(
    census: &Census,
    modules: &[Module],
    namespace: &[String],
) -> Result<Projection, ProofError> {
    root(census, namespace)?;
    let module = selected_module(modules, namespace)?;
    let file = original_file(census, &module.path)?;
    let parsed = syn::parse_file(&file.bytes)
        .map_err(|_| ProofError::Unsupported("Generated-unit module is not original Rust"))?;
    let (definition, invocation) = syntax::items(&parsed)?;
    let checked = syntax::verify(file, definition, invocation)?;
    let units = Invocation::parse(invocation.mac.tokens.clone(), file)?;

    Ok(Projection {
        namespace: namespace.to_vec(),
        macro_name: definition.ident.as_ref().unwrap().to_string(),
        source: file.bytes.clone(),
        definition: checked.header,
        full_definition: site(file, definition.span()),
        invocation: site(file, invocation.span()),
        units,
        templates: checked.templates,
    })
}

fn selected_module<'a>(
    modules: &'a [Module],
    namespace: &[String],
) -> Result<&'a Module, ProofError> {
    let mut matching = modules
        .iter()
        .filter(|module| module.namespace == namespace);
    let module = matching
        .next()
        .ok_or(ProofError::Missing("Missing generated-unit source module"))?;
    if matching.next().is_some() {
        return Err(ProofError::Contradictory("Ambiguous generated-unit module"));
    }
    Ok(module)
}

fn root(census: &Census, namespace: &[String]) -> Result<(), ProofError> {
    if census.identity != census::IDENTITY || namespace.len() != 1 {
        return Err(ProofError::Unsupported(
            "Generated-unit projection requires one original root child module",
        ));
    }
    census.context.validate().map_err(|_| {
        ProofError::Contradictory("Generated-unit producer package context differs")
    })?;
    let file = original_file(census, &census.context.root_source)?;
    let root = syn::parse_file(&file.bytes).map_err(|_| {
        ProofError::Unsupported("Generated-unit producer root is not original Rust")
    })?;
    syntax::root(&root, &namespace[0])
}

fn original_file<'a>(
    census: &'a Census,
    path: &std::path::Path,
) -> Result<&'a UnitFile, ProofError> {
    let mut matching = census.files.iter().filter(|file| file.path == path);
    let file = matching.next().ok_or(ProofError::Missing(
        "Missing original generated-unit buffer",
    ))?;
    if matching.next().is_some() || !file.bytes.matches_identity(&file.digest) {
        return Err(ProofError::Contradictory(
            "Original generated-unit buffer ownership differs",
        ));
    }
    if file.bytes.len() > census::MAX_BYTES {
        return Err(ProofError::Unsupported(
            "Generated-unit source exceeds budget",
        ));
    }
    Ok(file)
}

fn site(file: &UnitFile, span: proc_macro2::Span) -> FileSpan {
    FileSpan {
        path: file.path.clone(),
        digest: file.digest.clone(),
        span: crate::source_evidence::original_span(&file.bytes, span),
    }
}
