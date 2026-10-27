//! Exact pinned library bindings, independent of failed original-body resolution.
use super::*;

impl SourceMacros {
    pub(in super::super) fn library_invocation(
        &self,
        namespace: &[String],
        mac: &syn::Macro,
        registry: Option<&crate::data_library::Registry>,
    ) -> Result<bool, String> {
        let names = crate::source_evidence::plain_relative_path(&mac.path)
            .ok_or("Library cleanup macro requires a plain relative source path")?;
        let module = self
            .modules
            .get(namespace)
            .ok_or("Missing library macro owner")?;

        if module.glob {
            return Err("Unknown glob prevents library macro ownership".into());
        }
        if names.len() == 2 && names[0] == "alloc" {
            return Ok(library_macros::qualified_alloc(mac, registry).is_some());
        }
        let [name] = names.as_slice() else {
            return Ok(false);
        };
        if !matches!(name.as_str(), "vec" | "format" | "thread_local")
            || module.definitions.contains_key(name)
            || module.barriers.contains(name)
            || module.children.contains_key(name)
        {
            return Ok(false);
        }
        let Some(imports) = module.imports.get(name) else {
            return Ok(false);
        };
        let [import] = imports.as_slice() else {
            return Err("Ambiguous active library macro import".into());
        };
        if let Some(error) = &import.error {
            return Err(error.clone());
        }
        let root = if name == "thread_local" {
            "std"
        } else {
            "alloc"
        };
        Ok(!import.absolute
            && import.target == [root.to_string(), name.clone()]
            && library_macros::import(&[root.into()], name, registry))
    }
}
