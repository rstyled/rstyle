//! Original external declaration locations; grants no dispatch authority.
use super::{index, shapes};
use crate::{HirFnFacts, SourceSnapshot, nominal_boundary::census};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use syn::visit::Visit;

mod methods;

pub(super) struct Locations {
    cfgs: Vec<String>,
    files: BTreeMap<PathBuf, SourceSnapshot>,
    declarations: BTreeMap<(PathBuf, u32, u32), Declaration>,
}

struct Declaration {
    qualified_name: String,
    function: syn::ItemFn,
    kind: &'static str,
}

impl Locations {
    pub(super) fn new(census: &census::Census, cfgs: &[String]) -> Result<Self, String> {
        preflight(census)?;
        let modules = census::verify_selected(census, cfgs)?;
        check_population(&modules)?;

        let files = census
            .files
            .iter()
            .map(|file| (file.path.clone(), file.bytes.clone()))
            .collect();
        let mut locations = Self {
            cfgs: cfgs.to_vec(),
            files,
            declarations: BTreeMap::new(),
        };

        let mut names = BTreeSet::new();
        for module in modules {
            locations.insert_module(module, &mut names)?;
        }
        Ok(locations)
    }

    fn insert_module(
        &mut self,
        module: census::Module,
        names: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        let source = &self.files[&module.path];
        for (name, function, kind) in methods::declarations(module.items, &self.cfgs, source)? {
            let qualified_name = if module.namespace.is_empty() {
                format!("crate::{name}")
            } else {
                format!("crate::{}::{name}", module.namespace.join("::"))
            };
            if !names.insert(qualified_name.clone()) {
                return Err("Review source has ambiguous qualified declaration identity".into());
            }

            let range = shapes::function_range(source, &function);
            let key = (module.path.clone(), range.start, range.end);
            let declaration = Declaration {
                qualified_name,
                function,
                kind,
            };
            if self.declarations.insert(key, declaration).is_some() {
                return Err("Review source has ambiguous external declaration ownership".into());
            }
        }
        Ok(())
    }

    pub(super) fn location(
        &self,
        facts: &HirFnFacts,
        source: &str,
    ) -> Result<(String, &'static str), String> {
        self.check_context(facts)?;
        let path = self.checked_path(facts, source)?;
        index::check_function_bytes(facts, source)?;

        let key = (PathBuf::from(path), facts.fn_span.start, facts.fn_span.end);
        let declaration = self
            .declarations
            .get(&key)
            .ok_or("Review requires an original indexed external declaration")?;
        index::check_header(facts, &declaration.function, source)?;
        Ok((declaration.qualified_name.clone(), declaration.kind))
    }

    fn check_context(&self, facts: &HirFnFacts) -> Result<(), String> {
        if self.cfgs != facts.build.cfgs {
            return Err("Review original module selection has another compiler cfg context".into());
        }
        if facts
            .source_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.construct == "macro_generated_function")
        {
            return Err("Generated function has no original external declaration".into());
        }
        Ok(())
    }

    fn checked_path<'a>(&self, facts: &'a HirFnFacts, source: &str) -> Result<&'a str, String> {
        let path = facts
            .build
            .frontend_options
            .get("source_file")
            .and_then(serde_json::Value::as_str)
            .ok_or("Review original function source path is unavailable")?;
        let file = self
            .files
            .get(std::path::Path::new(path))
            .ok_or("Review external file is outside its immutable census")?;
        if &**file != source {
            return Err("Review external file differs from its immutable census bytes".into());
        }
        Ok(path)
    }
}

fn preflight(census: &census::Census) -> Result<(), String> {
    census::check_budget(census)?;
    for file in &census.files {
        if file.bytes.len() > 256 * 1024 {
            return Err("Review original source byte budget exhausted before parsing".into());
        }
        let mut depth = 0_usize;
        for byte in file.bytes.bytes() {
            match byte {
                b'{' | b'(' | b'[' => depth += 1,
                b'}' | b')' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
            if depth > 256 {
                return Err(
                    "Review original source delimiter budget exhausted before parsing".into(),
                );
            }
        }
    }
    Ok(())
}

fn check_population(modules: &[census::Module]) -> Result<(), String> {
    let mut namespaces = BTreeSet::new();
    let mut population = Population::default();
    for module in modules {
        if module.namespace.len() > 32 || !namespaces.insert(&module.namespace) {
            return Err(
                "Review module qualification is ambiguous or exceeds its depth budget".into(),
            );
        }
        for item in &module.items {
            population.visit_item(item);
        }
        if population.items > 4096 {
            return Err("Review original declaration population budget exhausted".into());
        }
    }
    Ok(())
}

#[derive(Default)]
struct Population {
    items: usize,
}

impl Population {
    fn retain_item(&mut self) -> bool {
        self.items += 1;
        self.items <= 4096
    }
}

impl<'ast> Visit<'ast> for Population {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        if self.retain_item() {
            syn::visit::visit_item(self, item);
        }
    }

    fn visit_impl_item(&mut self, item: &'ast syn::ImplItem) {
        if self.retain_item() {
            syn::visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast syn::TraitItem) {
        if self.retain_item() {
            syn::visit::visit_trait_item(self, item);
        }
    }
}

#[cfg(test)]
mod tests;
