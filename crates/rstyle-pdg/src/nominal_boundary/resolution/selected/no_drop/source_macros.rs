//! Original same-crate macro imports; no expansion, callable or cleanup authority.
mod collect;
mod paths;
#[cfg(test)]
mod tests;

use super::*;
use std::collections::{BTreeMap, BTreeSet};

type Key = Vec<String>;

#[derive(Clone)]
struct Import {
    target: Key,
    visible: Key,
    position: usize,
    absolute: bool,
    error: Option<String>,
}

#[derive(Default)]
struct Module {
    definitions: BTreeMap<String, Vec<syn::ItemMacro>>,
    imports: BTreeMap<String, Vec<Import>>,
    children: BTreeMap<String, (Key, Key)>,
    barriers: BTreeSet<String>,
    invalid_definitions: BTreeMap<usize, String>,
    glob: bool,
}

#[derive(Clone)]
enum Resolved {
    Macro(syn::ItemMacro),
    Module(Key),
}

struct Request<'a> {
    requester: &'a [String],
    visiting: &'a mut BTreeSet<Key>,
    local_position: Option<usize>,
}

pub(super) struct SourceMacros {
    modules: BTreeMap<Key, Module>,
    imports: BTreeMap<Key, Result<Resolved, String>>,
}

impl SourceMacros {
    pub(super) fn new(environment: &Environment, cfgs: &[String]) -> Result<Self, String> {
        let configuration = crate::conditional_source::Configuration::new(cfgs);
        let mut result = Self::from_modules(&environment.modules, &configuration)?;

        result.check_originals(environment)?;
        result.check_globs(environment, &configuration)?;
        result.index_imports();
        Ok(result)
    }

    pub(super) fn resolve(
        &self,
        namespace: &[String],
        mac: &syn::Macro,
    ) -> Result<Option<syn::ItemMacro>, String> {
        let names = crate::source_evidence::plain_relative_path(&mac.path)
            .ok_or("Cleanup macro import requires a plain relative source path")?;
        let module = self
            .modules
            .get(namespace)
            .ok_or("Missing original macro module")?;

        if names.len() == 1 && module.definitions.contains_key(&names[0]) {
            // Textual definition order and block-local definitions belong to Closure.
            return Ok(None);
        }
        let mut resolving = BTreeSet::new();
        let mut request = Request {
            requester: namespace,
            visiting: &mut resolving,
            local_position: None,
        };
        match self.path(namespace, &names, &mut request)? {
            Some(Resolved::Macro(definition)) => Ok(Some(definition)),
            Some(Resolved::Module(_)) => Err("Macro invocation resolves to a source module".into()),
            None => Ok(None),
        }
    }

    fn index_imports(&mut self) {
        let keys: Vec<_> = self
            .modules
            .iter()
            .flat_map(|(owner, module)| {
                module
                    .imports
                    .keys()
                    .map(|name| joined(owner, name))
                    .collect::<Vec<_>>()
            })
            .collect();
        for key in keys {
            let result = self.import(&key, &mut BTreeSet::new());
            self.imports.insert(key, result);
        }
    }

    fn import(&self, key: &Key, visiting: &mut BTreeSet<Key>) -> Result<Resolved, String> {
        if let Some(result) = self.imports.get(key) {
            return result.clone();
        }
        if visiting.len() >= 128 {
            return Err("Original macro reexport depth exceeds bounded proof".into());
        }
        if !visiting.insert(key.clone()) {
            return Err("Cyclic original macro reexport".into());
        }

        let (name, owner) = key.split_last().ok_or("Empty macro import key")?;
        let module = self
            .modules
            .get(owner)
            .ok_or("Missing original macro import owner")?;
        let imports = module
            .imports
            .get(name)
            .ok_or("Missing claimed macro import")?;
        let [import] = imports.as_slice() else {
            return Err("Ambiguous active original macro import".into());
        };
        if let Some(error) = &import.error {
            return Err(error.clone());
        }
        let result = if import.absolute {
            Err("External-root macro import requires separate source ownership".into())
        } else {
            let mut request = Request {
                requester: owner,
                visiting,
                local_position: Some(import.position),
            };
            self.path(owner, &import.target, &mut request)?
                .ok_or_else(|| "Claimed macro import has no exact original target".into())
        };
        visiting.remove(key);
        result
    }

    fn binding(
        &self,
        owner: &[String],
        name: &str,
        request: &mut Request<'_>,
    ) -> Result<Option<Resolved>, String> {
        let module = self
            .modules
            .get(owner)
            .ok_or("Missing macro namespace source")?;

        if let Some(position) = request.local_position
            && let Some(definition) = collect::preceding(module, name, position)?
        {
            return Ok(Some(Resolved::Macro(definition)));
        }
        if module.barriers.contains(name) {
            return Err("Macro path has an unresolved original namespace binding".into());
        }
        if let Some((child, visible)) = module.children.get(name) {
            if module.imports.contains_key(name) {
                return Err("Macro path has duplicate module and import bindings".into());
            }
            if !request.requester.starts_with(visible) {
                return Err("Original macro module path is inaccessible".into());
            }
            return Ok(Some(Resolved::Module(child.clone())));
        }
        if let Some(imports) = module.imports.get(name) {
            let [import] = imports.as_slice() else {
                return Err("Ambiguous active macro reexport binding".into());
            };
            if !request.requester.starts_with(&import.visible) {
                return Err("Original macro reexport is inaccessible from this module".into());
            }
            return self
                .import(&joined(owner, name), request.visiting)
                .map(Some);
        }
        if module.definitions.contains_key(name) {
            return Err("Textual macro definition lacks a checked path reexport".into());
        }
        if module.glob {
            return Err("Unknown glob prevents exact original macro import resolution".into());
        }
        Ok(None)
    }
}

fn joined(owner: &[String], name: &str) -> Key {
    owner.iter().cloned().chain([name.to_owned()]).collect()
}
