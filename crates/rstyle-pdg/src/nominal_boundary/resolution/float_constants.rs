//! Exact same-module float declarations in the existing checked source closure.
use super::*;
use crate::{HirNode, scalar_constants::ScalarConstant, unit_store::EvidenceFacts};

impl Resolution {
    pub(crate) fn verify_float_constant(
        &self,
        node: &HirNode,
        record: &ScalarConstant,
        facts: &dyn EvidenceFacts,
    ) -> Result<(), String> {
        let syntax = crate::source_evidence::original_expression(node, facts)?;
        let syn::Expr::Path(path) = syntax else {
            return Err("Float constant requires an original bare value path".into());
        };
        let name = path
            .path
            .get_ident()
            .ok_or("Float constant must be same-module")?;
        if path.qself.is_some()
            || !path.attrs.is_empty()
            || facts.bindings.iter().any(|binding| *name == binding.name)
            || self
                .environment
                .index
                .imports
                .contains_key(&(self.namespace.clone(), name.to_string()))
        {
            return Err("Float constant is shadowed or imported".into());
        }

        let expected = self
            .namespace
            .iter()
            .cloned()
            .chain([name.to_string()])
            .collect::<Vec<_>>()
            .join("::");
        if record.definition != expected {
            return Err("Float constant definition belongs to another lexical module".into());
        }
        self.float_declaration(&name.to_string(), record)
    }

    fn float_declaration(&self, name: &str, record: &ScalarConstant) -> Result<(), String> {
        let mut declarations = self
            .environment
            .modules
            .iter()
            .filter(|module| module.namespace == self.namespace)
            .flat_map(|module| {
                module.items.iter().filter_map(move |item| match item {
                    syn::Item::Const(item) if item.ident == name => Some((module, item)),
                    _ => None,
                })
            });
        let (module, declaration) = declarations.next().ok_or("Missing owned float constant")?;
        if declarations.next().is_some() || !declaration.attrs.is_empty() {
            return Err("Float constant declaration is ambiguous or attributed".into());
        }

        let source = self.environment.source(&module.path)?;
        let span = declaration.span();
        let range = ByteSpan {
            start: crate::source_evidence::byte_offset(source.bytes(), span.start()) as u32,
            end: crate::source_evidence::byte_offset(source.bytes(), span.end()) as u32,
        };
        if source.range(range)? != record.declaration {
            return Err("Float constant disagrees with its complete checked declaration".into());
        }
        Ok(())
    }
}
