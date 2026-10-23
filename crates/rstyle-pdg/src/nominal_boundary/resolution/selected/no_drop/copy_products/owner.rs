//! Exact source/build ownership; only checked projection slots may normalize later.
use super::*;
use crate::{BuildFacts, ByteSpan, HirRef, SourceFacts};
use serde_json::Value;

pub(super) struct Owner {
    name: String,
    span: ByteSpan,
    source: SourceFacts,
    file: String,
    definition: Value,
    invocation: Value,
    hir: HirRef,
    build: BuildFacts<()>,
    origins: Origins,
    namespace: Namespace,
    profile: crate::extraction_profile::ExtractionProfile,
}

impl Owner {
    pub(super) fn capture(
        facts: &HirFnFacts,
        environment: &Environment,
        origins: &Origins,
    ) -> Result<Self, String> {
        environment.check_input_owner(facts)?;
        let file = source_file(facts)?;
        let namespace = namespace(facts, environment, file)?;
        Ok(Self {
            name: facts.name.clone(),
            span: facts.fn_span,
            source: facts.source.clone(),
            file: file.into(),
            definition: definition(facts)?.clone(),
            invocation: invocation(facts)?.clone(),
            hir: facts.body.hir.clone(),
            build: build(facts),
            origins: origins.clone(),
            namespace,
            profile: crate::extraction_profile::ExtractionProfile::from_facts(facts)?,
        })
    }

    pub(super) fn verify(
        &self,
        facts: &HirFnFacts,
        environment: &Environment,
    ) -> Result<(), String> {
        environment.check_input_owner(facts)?;
        let origins = facts.build.frontend_options.data::<Origins>()?;

        if self.name != facts.name
            || self.span != facts.fn_span
            || self.source != facts.source
            || self.file != source_file(facts)?
            || &self.definition != definition(facts)?
            || &self.invocation != invocation(facts)?
            || self.hir != facts.body.hir
            || self.build != build(facts)
            || self.origins != *origins
            || self.profile != crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        {
            return Err(
                "Copy product source belongs to another original owner/build/declaration context"
                    .into(),
            );
        }
        Ok(())
    }

    pub(super) fn namespace(&self) -> &[String] {
        &self.namespace
    }
}

fn namespace(
    facts: &HirFnFacts,
    environment: &Environment,
    file: &str,
) -> Result<Namespace, String> {
    let mut candidates = Vec::new();

    for module in environment
        .modules
        .iter()
        .filter(|module| module.path == std::path::Path::new(file))
    {
        let bytes =
            crate::nominal_boundary::resolution::file_bytes(environment.census(), &module.path)?;
        candidates.extend(crate::nominal_boundary::resolution::module_functions(
            module,
            bytes,
            facts,
            environment,
        )?);
    }
    let [(namespace, _)] = candidates.as_slice() else {
        return Err("Copy product source lacks a unique exact original function namespace".into());
    };
    Ok(namespace.clone())
}

fn source_file(facts: &HirFnFacts) -> Result<&str, String> {
    facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(Value::as_str)
        .ok_or_else(|| "Copy product source lacks its original source file".into())
}

fn definition(facts: &HirFnFacts) -> Result<&Value, String> {
    required_value(
        facts,
        "definition_identity",
        |value| value.as_str().is_some_and(|text| !text.is_empty()),
        "Copy product source compiler definition identity is missing or incomplete",
    )
}

fn build(facts: &HirFnFacts) -> BuildFacts<()> {
    let build = &facts.build;
    BuildFacts {
        producer_revision: build.producer_revision.clone(),
        frontend_options: (),
        rustc_version: build.rustc_version.clone(),
        rustc_commit: build.rustc_commit.clone(),
        edition: build.edition.clone(),
        target_triple: build.target_triple.clone(),
        pointer_width: build.pointer_width.clone(),
        data_layout: build.data_layout.clone(),
        cfgs: build.cfgs.clone(),
        features: build.features.clone(),
        overflow_checks: build.overflow_checks.clone(),
        debug_assertions: build.debug_assertions.clone(),
        panic_strategy: build.panic_strategy.clone(),
        opt_level: build.opt_level.clone(),
    }
}

fn invocation(facts: &HirFnFacts) -> Result<&Value, String> {
    required_value(
        facts,
        "invocation",
        |value| {
            value
                .as_array()
                .is_some_and(|items| !items.is_empty() && items.iter().all(Value::is_string))
        },
        "Copy product source compiler invocation is missing or incomplete",
    )
}

fn required_value<'a>(
    facts: &'a HirFnFacts,
    key: &str,
    valid: impl FnOnce(&Value) -> bool,
    error: &str,
) -> Result<&'a Value, String> {
    facts
        .build
        .frontend_options
        .get(key)
        .filter(|value| valid(value))
        .ok_or_else(|| error.into())
}
