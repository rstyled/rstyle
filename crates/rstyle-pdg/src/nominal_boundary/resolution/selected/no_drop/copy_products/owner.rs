//! Exact source/build ownership; only checked projection slots may normalize later.
use super::*;
use crate::{BuildFacts, ByteSpan, HirRef, SourceFacts};
use serde_json::Value;

pub(in crate::nominal_boundary::resolution::selected::no_drop) struct Owner {
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
    pub(in crate::nominal_boundary::resolution::selected::no_drop) fn capture(
        facts: &HirFnFacts,
        environment: &Environment,
        origins: &Origins,
    ) -> Result<Self, String> {
        let file = source_file(facts)?;
        let namespace = namespace(facts, environment, file)?;
        Self::capture_at(facts, environment, origins, namespace)
    }

    /// The caller must first prove the exact original callable namespace.
    pub(in crate::nominal_boundary::resolution::selected::no_drop) fn capture_at(
        facts: &HirFnFacts,
        environment: &Environment,
        origins: &Origins,
        namespace: Namespace,
    ) -> Result<Self, String> {
        environment.check_input_owner(facts)?;
        let file = source_file(facts)?;
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

    pub(in crate::nominal_boundary::resolution::selected::no_drop) fn verify(
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

    pub(in crate::nominal_boundary::resolution::selected::no_drop) fn namespace(
        &self,
    ) -> &[String] {
        &self.namespace
    }
}

fn namespace(
    facts: &HirFnFacts,
    environment: &Environment,
    file: &str,
) -> Result<Namespace, String> {
    if crate::extraction_profile::facts_support(facts, crate::range_values::CONSUMING_CAPABILITY)
        && crate::range_values::consuming_input(facts)
    {
        crate::range_values::consuming::verify_fields(facts)?;
        return range_namespace(facts, environment, file);
    }
    let mut candidates = Vec::new();

    for (module_index, module) in environment
        .modules
        .iter()
        .enumerate()
        .filter(|(_, module)| module.path == std::path::Path::new(file))
    {
        let bytes =
            crate::nominal_boundary::resolution::file_bytes(environment.census(), &module.path)?;
        candidates.extend(crate::nominal_boundary::resolution::module_functions(
            module_index,
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

/// Bind this receipt to source, without granting callable trait ownership.
/// The reverse conversion resolver independently checks the exact trait/impl.
fn range_namespace(
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
        let bytes = environment.source(&module.path)?.bytes();
        for item in &module.items {
            let syn::Item::Impl(item) = item else {
                continue;
            };
            if item.trait_.is_none()
                || item.unsafety.is_some()
                || item.defaultness.is_some()
                || !item.generics.params.is_empty()
                || item.generics.where_clause.is_some()
                || item.items.len() != 1
            {
                continue;
            }
            for member in &item.items {
                let syn::ImplItem::Fn(method) = member else {
                    continue;
                };
                if (crate::nominal_boundary::resolution::FunctionSource {
                    vis: &method.vis,
                    sig: &method.sig,
                    body: &method.block,
                })
                .exact(bytes, facts)
                {
                    super::super::super::function_attributes(facts, &item.attrs)?;
                    super::super::super::function_attributes(facts, &method.attrs)?;
                    candidates.push(module.namespace.clone());
                }
            }
        }
    }
    let [namespace] = candidates.as_slice() else {
        return Err("Range Copy-source receipt lacks one exact original source owner".into());
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
