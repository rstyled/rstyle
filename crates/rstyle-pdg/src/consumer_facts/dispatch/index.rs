use super::{Assessment, capabilities, review, shapes};
use crate::{
    HirFnFacts,
    nominal_boundary::{self, census},
};
use serde::Deserialize;
use std::{cell::OnceCell, collections::BTreeMap, sync::Arc};

pub struct Index {
    units: Vec<Unit>,
}

struct Unit {
    owner_index: usize,
    census: Arc<census::Census>,
    context: serde_json::Value,
    source: Result<SourceIndex, shapes::SourceFailure>,
    locations: OnceCell<Result<super::locations::Locations, String>>,
}

struct SourceIndex {
    modules: Vec<shapes::Module>,
    declarations: BTreeMap<(u32, u32), (usize, usize)>,
}

impl SourceIndex {
    fn new(modules: Vec<shapes::Module>, source: &str) -> Result<Self, String> {
        let mut declarations = BTreeMap::new();
        let mut namespaces = std::collections::BTreeSet::new();
        for (module_index, module) in modules.iter().enumerate() {
            if !namespaces.insert(&module.namespace) {
                return Err("Dispatch source has ambiguous inline module qualification".into());
            }
            for (function_index, function) in module.functions.iter().enumerate() {
                let span = shapes::function_range(source, function);
                if declarations
                    .insert((span.start, span.end), (module_index, function_index))
                    .is_some()
                {
                    return Err("Dispatch source function has ambiguous module ownership".into());
                }
            }
        }
        Ok(Self {
            modules,
            declarations,
        })
    }
}

impl Index {
    pub fn from_facts(facts: &[&HirFnFacts]) -> Result<Self, String> {
        let mut units: Vec<Unit> = vec![];
        for facts in facts {
            let Some((census, context)) = owner(facts)? else {
                continue;
            };
            if let Some(unit) = units.iter().find(|unit| Arc::ptr_eq(&unit.census, &census)) {
                if unit.context != *context {
                    return Err("Dispatch source owner has conflicting selected contexts".into());
                }
            } else {
                let unit = Unit::new(census, context, units.len())?;
                units.push(unit);
            }
        }
        Ok(Self { units })
    }

    pub fn evaluate(&self, facts: &HirFnFacts) -> Result<Assessment, String> {
        self.query(facts, |declaration| {
            shapes::evaluate(declaration.module, declaration.function, declaration.source)
        })
    }

    pub fn evaluate_capability_wrapper(
        &self,
        facts: &HirFnFacts,
    ) -> Result<capabilities::Assessment, String> {
        self.query(facts, capabilities::evaluate)
    }

    /// Source-located identity without requiring a positive analysis finding.
    pub fn review_location(&self, facts: &HirFnFacts) -> Result<(String, &'static str), String> {
        let Some(unit) = self.selected(facts)? else {
            return Err("Review requires original selected-unit source ownership".into());
        };
        let Some(file) = unit.file(facts)? else {
            return Err("Review requires an original function source path".into());
        };
        if file.path != unit.census.context.root_source {
            let locations = unit
                .locations
                .get_or_init(|| super::locations::Locations::new(&unit.census, &facts.build.cfgs));
            return locations
                .as_ref()
                .map_err(Clone::clone)?
                .location(facts, &file.bytes);
        }
        available(unit.query_file(facts, file, |declaration| Assessment {
            availability: super::super::Availability::Known(Some(Ok(review::location(
                &declaration,
            )))),
            reason: None,
            budget_exhausted: false,
        })?)
    }

    pub fn review_dispatch(
        &self,
        facts: &HirFnFacts,
        candidate: &super::Candidate,
    ) -> Result<review::Dispatch, String> {
        self.review(facts, |declaration| {
            review::dispatch(declaration, candidate)
        })
    }

    pub fn review_capability(
        &self,
        facts: &HirFnFacts,
        wrapper: &capabilities::Wrapper,
    ) -> Result<review::Capability, String> {
        self.review(facts, |declaration| {
            review::capability(declaration, wrapper)
        })
    }

    fn review<T>(
        &self,
        facts: &HirFnFacts,
        build: impl FnOnce(CheckedDeclaration<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        let result = self.query(facts, |declaration| Assessment {
            availability: super::super::Availability::Known(Some(build(declaration))),
            reason: None,
            budget_exhausted: false,
        })?;
        available(result)
    }

    fn query<T>(
        &self,
        facts: &HirFnFacts,
        evaluate: impl FnOnce(CheckedDeclaration<'_>) -> Assessment<T>,
    ) -> Result<Assessment<T>, String> {
        let Some(unit) = self.selected(facts)? else {
            return Ok(Assessment::unknown(
                "Original selected-unit source ownership was not captured",
            ));
        };
        let Some(file) = unit.file(facts)? else {
            return Ok(Assessment::unknown(
                "Original function source path was not captured",
            ));
        };

        unit.query_file(facts, file, evaluate)
    }

    fn selected(&self, facts: &HirFnFacts) -> Result<Option<&Unit>, String> {
        let Some((census, context)) = owner(facts)? else {
            return Ok(None);
        };
        let unit = self
            .units
            .iter()
            .find(|unit| Arc::ptr_eq(&unit.census, &census))
            .ok_or("Dispatch facts are outside the indexed immutable source owners")?;
        if unit.context != *context {
            return Err("Dispatch source owner context changed".into());
        }
        Ok(Some(unit))
    }
}

impl Unit {
    fn new(
        census: Arc<census::Census>,
        context: &serde_json::Value,
        owner_index: usize,
    ) -> Result<Self, String> {
        let selected = nominal_boundary::UnitContext::deserialize(context)
            .map_err(|error| error.to_string())?;
        selected.validate()?;
        super::super::check_census(&census, &selected)?;
        checked_files(&census)?;

        let file = census
            .files
            .iter()
            .find(|file| file.path == selected.root_source)
            .ok_or("Dispatch census lacks the compiler-selected root source")?;
        let source = match shapes::parse(&file.bytes) {
            Ok(modules) => Ok(SourceIndex::new(modules, &file.bytes)?),
            Err(failure) => Err(failure),
        };
        Ok(Self {
            owner_index,
            census,
            context: context.clone(),
            source,
            locations: OnceCell::new(),
        })
    }

    fn file(&self, facts: &HirFnFacts) -> Result<Option<&census::UnitFile>, String> {
        let Some(path) = facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
        else {
            return Ok(None);
        };
        let file = self
            .census
            .files
            .iter()
            .find(|file| file.path.to_str() == Some(path))
            .ok_or("Dispatch function file is absent from its immutable census")?;
        Ok(Some(file))
    }

    fn query_file<T>(
        &self,
        facts: &HirFnFacts,
        file: &census::UnitFile,
        evaluate: impl FnOnce(CheckedDeclaration<'_>) -> Assessment<T>,
    ) -> Result<Assessment<T>, String> {
        check_function_bytes(facts, &file.bytes)?;
        if file.path != self.census.context.root_source {
            return Ok(Assessment::unsupported(
                "Only the compiler-selected root file and its inline modules are indexed",
            ));
        }
        if facts
            .source_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.construct == "macro_generated_function")
        {
            return Ok(Assessment::unsupported(
                "Generated function has no original dispatch declaration",
            ));
        }

        let modules = match &self.source {
            Ok(modules) => modules,
            Err(failure) => {
                let mut assessment = Assessment::unsupported(&failure.reason);
                assessment.budget_exhausted = failure.budget_exhausted;
                return Ok(assessment);
            }
        };
        let Some(declaration) =
            CheckedDeclaration::new(modules, facts, &file.bytes, self.owner_index)?
        else {
            return Ok(Assessment::unsupported(
                "Owned function is not an original indexed free declaration",
            ));
        };
        Ok(evaluate(declaration))
    }
}

fn owner(facts: &HirFnFacts) -> Result<Option<(Arc<census::Census>, &serde_json::Value)>, String> {
    let options = &facts.build.frontend_options;
    let (Some(context), true) = (
        options.get(nominal_boundary::CONTEXT_KEY),
        options.contains_key(census::KEY),
    ) else {
        return Ok(None);
    };
    Ok(Some((
        options.data::<Arc<census::Census>>()?.into_owned(),
        context,
    )))
}

fn checked_files(census: &census::Census) -> Result<(), String> {
    let mut paths = std::collections::BTreeSet::new();
    for file in &census.files {
        if !paths.insert(&file.path) || !file.bytes.matches_identity(&file.digest) {
            return Err(
                "Dispatch census has duplicate paths or inconsistent original file bytes".into(),
            );
        }
    }
    Ok(())
}

pub(super) fn check_function_bytes(facts: &HirFnFacts, source: &str) -> Result<(), String> {
    if !facts.source.bytes.matches_identity(&facts.source.digest)
        || facts.fn_span.end.checked_sub(facts.fn_span.start)
            != u32::try_from(facts.source.bytes.len()).ok()
        || crate::source_evidence::byte_range(source, facts.fn_span)? != facts.source.bytes.as_str()
    {
        return Err("Dispatch original function bytes disagree with their owned range".into());
    }
    Ok(())
}

pub(super) struct CheckedDeclaration<'a> {
    pub module: &'a shapes::Module,
    pub function: &'a syn::ItemFn,
    pub source: &'a str,
    pub owner_index: usize,
    pub module_index: usize,
}

impl<'a> CheckedDeclaration<'a> {
    fn new(
        index: &'a SourceIndex,
        facts: &HirFnFacts,
        source: &'a str,
        owner_index: usize,
    ) -> Result<Option<Self>, String> {
        let Some(&(module_index, function_index)) = index
            .declarations
            .get(&(facts.fn_span.start, facts.fn_span.end))
        else {
            return Ok(None);
        };
        let module = &index.modules[module_index];
        let function = &module.functions[function_index];
        check_header(facts, function, source)?;

        Ok(Some(Self {
            module,
            function,
            source,
            owner_index,
            module_index,
        }))
    }
}

pub(super) fn check_header(
    facts: &HirFnFacts,
    function: &syn::ItemFn,
    source: &str,
) -> Result<(), String> {
    let header = shapes::function_header(source, function);
    if header.start != facts.fn_span.start
        || header.end.checked_sub(header.start) != Some(facts.signature.sig_span.end)
        || facts.signature.sig_span.start != 0
        || function.sig.inputs.len() != facts.signature.params.len()
        || facts.signature.has_self != function.sig.receiver().is_some()
        || facts.signature.receiver.is_some() != function.sig.receiver().is_some()
    {
        return Err("Dispatch source header differs from captured signature observations".into());
    }
    if facts
        .build
        .frontend_options
        .get("source_identifier")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|name| name != function.sig.ident.to_string().trim_start_matches("r#"))
    {
        return Err("Dispatch source identifier differs from its owned declaration".into());
    }
    Ok(())
}

fn available<T>(assessment: Assessment<Result<T, String>>) -> Result<T, String> {
    match assessment.availability {
        super::super::Availability::Known(Some(value)) => value,
        _ => Err("Review requires an available original indexed declaration".into()),
    }
}
