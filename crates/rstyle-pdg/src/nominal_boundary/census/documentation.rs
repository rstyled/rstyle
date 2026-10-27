//! Literal crate documentation inputs retain bytes and exact compiler ownership.
use super::{Census, expansions::FileSpan};
use crate::{data_library::Callee, unit_store::EvidenceFacts};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};
use syn::spanned::Spanned;
#[cfg(test)]
mod tests;

pub const CAPABILITY: &str = "rust.literal-documentation-inputs";
const DEFINITION: &str = "core::include_str#macro";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub invocation: FileSpan,
    pub input: FileSpan,
    pub physical_path: PathBuf,
    pub builtin: Callee,
    pub expanded_digest: String,
}

pub(super) fn authority(facts: &dyn EvidenceFacts, census: &Census) -> Result<(), String> {
    if census.documentation.is_empty() {
        return Ok(());
    }
    if !crate::extraction_profile::facts_support(facts, CAPABILITY) {
        return Err("Documentation inputs require their registered extraction capability".into());
    }

    for input in &census.documentation {
        crate::data_library::verify_core_definition(
            &input.builtin,
            facts,
            crate::data_library::CoreDefinition {
                paths: &[DEFINITION, "core::include_str", "std::include_str"],
                unavailable: "Documentation input requires pinned compiler/target",
                mismatch: "Documentation input lacks exact builtin include_str authority",
            },
        )?;
    }

    Ok(())
}

pub(crate) fn verify(census: &Census, cfgs: &[String]) -> Result<BTreeSet<PathBuf>, String> {
    let root = census
        .files
        .iter()
        .find(|file| file.path == census.context.root_source)
        .ok_or("Missing documentation owner root source")?;
    let source = root.bytes.strip_prefix('\u{feff}').unwrap_or(&root.bytes);
    let syntax = syn::parse_file(source).map_err(|error| error.to_string())?;

    let configuration = crate::conditional_source::Configuration::new(cfgs);
    let attributes = configuration.effective_attributes(&syntax.attrs)?;
    let mut inputs = BTreeSet::new();

    for input in &census.documentation {
        check_builtin(input)?;
        if input.invocation.path != root.path
            || !root.bytes.matches_identity(&input.invocation.digest)
        {
            return Err("Documentation invocation belongs to another source owner".into());
        }

        let matching = attributes
            .iter()
            .filter_map(|attribute| match attribute {
                syn::Meta::NameValue(value) if value.path.is_ident("doc") => Some(&value.value),
                _ => None,
            })
            .filter(|value| {
                crate::source_evidence::original_span(&root.bytes, value.span())
                    == input.invocation.span
            })
            .collect::<Vec<_>>();
        let [syn::Expr::Macro(invocation)] = matching.as_slice() else {
            return Err("Documentation input lacks a unique original root doc invocation".into());
        };
        let literal: syn::LitStr =
            syn::parse2(invocation.mac.tokens.clone()).map_err(|error| error.to_string())?;
        if !invocation.mac.path.is_ident("include_str") {
            return Err("Documentation input requires direct literal include_str".into());
        }

        check_input(census, input, &literal.value())?;
        if !inputs.insert(input.input.path.clone()) {
            return Err("Documentation input has duplicate ownership".into());
        }
    }
    Ok(inputs)
}

fn check_builtin(input: &Input) -> Result<(), String> {
    if !["core::include_str", "std::include_str"].contains(&input.builtin.path.as_str())
        || !crate::data_library::registries()
            .iter()
            .any(|registry| registry.exact_core_definition(&input.builtin, DEFINITION))
    {
        return Err("Documentation input lacks a pinned builtin definition".into());
    }

    Ok(())
}

fn check_input(census: &Census, input: &Input, literal: &str) -> Result<(), String> {
    let package = census
        .context
        .package
        .manifest_path
        .parent()
        .ok_or("Missing package root")?;
    let parent = census
        .context
        .root_source
        .parent()
        .ok_or("Missing documentation owner directory")?;
    if Path::new(literal).is_absolute()
        || parent.join(literal) != input.input.path
        || in_package(package, &input.input.path)? != input.physical_path
    {
        return Err(
            "Documentation path escapes, redirects or disagrees with its literal input".into(),
        );
    }

    let file = census
        .files
        .iter()
        .find(|file| file.path == input.input.path)
        .ok_or("Missing immutable documentation input")?;
    if !file.bytes.matches_identity(&file.digest)
        || input.input.digest != file.digest
        || input.expanded_digest != file.digest
        || input.input.span.start
            != if file.bytes.starts_with('\u{feff}') {
                3
            } else {
                0
            }
        || input.input.span.end as usize != file.bytes.len()
    {
        return Err("Documentation input bytes, expansion or complete input span disagree".into());
    }

    Ok(())
}

/// Lexical resolution is restricted to a declared documentation input. Compiler
/// capture additionally records its physical path; redirections cannot match.
pub(super) fn in_package(root: &Path, path: &Path) -> Result<PathBuf, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "Documentation input is outside package")?;
    let mut result = root.to_path_buf();
    for component in relative.components() {
        match component {
            Component::Normal(part) => result.push(part),
            Component::CurDir => {}
            Component::ParentDir if result != root => {
                result.pop();
            }
            _ => return Err("Documentation input escapes package".into()),
        }
    }
    Ok(result)
}
