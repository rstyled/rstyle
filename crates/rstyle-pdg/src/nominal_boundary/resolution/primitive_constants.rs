//! Primitive spelling must remain unshadowed in its exact checked lexical scope.
use super::*;
use crate::{HirNode, unit_store::EvidenceFacts};
use syn::{ext::IdentExt, visit::Visit};

impl Resolution {
    pub(crate) fn verify_primitive_constant_source(
        &self,
        node: &HirNode,
        facts: &dyn EvidenceFacts,
    ) -> Result<(), String> {
        self.environment.index.primitive_root(&self.namespace)?;
        let function: syn::ItemFn = syn::parse_str(&facts.source.bytes)
            .map_err(|_| "Primitive constant lacks its original owning function")?;
        local_source(&function, node.span, &facts.source.bytes)
    }
}

impl Index {
    fn primitive_root(&self, namespace: &Namespace) -> Result<(), String> {
        if self.unshadowed_u32.get(namespace) != Some(&true)
            || self
                .imports
                .contains_key(&(namespace.clone(), "u32".into()))
        {
            return Err(
                "Primitive constant root is shadowed or lacks a checked owning module".into(),
            );
        }
        Ok(())
    }
}

pub(super) fn shadowing_item(item: &syn::Item) -> bool {
    let name = match item {
        syn::Item::Struct(item) => Some(&item.ident),
        syn::Item::Enum(item) => Some(&item.ident),
        syn::Item::Union(item) => Some(&item.ident),
        syn::Item::Type(item) => Some(&item.ident),
        syn::Item::Trait(item) => Some(&item.ident),
        syn::Item::TraitAlias(item) => Some(&item.ident),
        syn::Item::Mod(item) => Some(&item.ident),
        syn::Item::ExternCrate(item) => {
            Some(item.rename.as_ref().map_or(&item.ident, |(_, name)| name))
        }
        syn::Item::Use(item) => return imported(&item.tree, None),
        // An invocation may introduce type names; a macro definition alone cannot.
        syn::Item::Macro(item) => return item.ident.is_none(),
        _ => None,
    };
    name.is_some_and(|name| name.unraw() == "u32")
}

fn imported(tree: &syn::UseTree, parent: Option<&syn::Ident>) -> bool {
    match tree {
        syn::UseTree::Name(name) => {
            let ident = if name.ident == "self" {
                parent.unwrap_or(&name.ident)
            } else {
                &name.ident
            };
            ident.unraw() == "u32"
        }
        syn::UseTree::Rename(rename) => rename.rename.unraw() == "u32",
        syn::UseTree::Path(path) => imported(&path.tree, Some(&path.ident)),
        syn::UseTree::Group(group) => group.items.iter().any(|item| imported(item, parent)),
        // Module-level globs are checked by the closed import environment;
        // local globs are deliberately unavailable to this bounded proof.
        syn::UseTree::Glob(_) => true,
    }
}

fn local_source(function: &syn::ItemFn, target: ByteSpan, source: &str) -> Result<(), String> {
    if function.sig.generics.params.iter().any(|parameter| {
        matches!(parameter, syn::GenericParam::Type(parameter) if parameter.ident.unraw() == "u32")
    }) {
        return Err("Primitive constant root is shadowed by a type parameter".into());
    }

    let mut scope = LocalScope {
        target,
        source,
        shadowed: false,
        found: false,
    };
    scope.visit_block(&function.block);
    if scope.shadowed || !scope.found {
        return Err("Primitive constant root is shadowed or lacks an exact local scope".into());
    }

    Ok(())
}

struct LocalScope<'a> {
    target: ByteSpan,
    source: &'a str,
    shadowed: bool,
    found: bool,
}

impl<'ast> Visit<'ast> for LocalScope<'_> {
    fn visit_block(&mut self, block: &'ast syn::Block) {
        let span = crate::source_evidence::original_span(self.source, block.span());
        if span.start > self.target.start || span.end < self.target.end {
            return;
        }

        self.found = true;
        self.shadowed |= block
            .stmts
            .iter()
            .any(|statement| matches!(statement, syn::Stmt::Item(item) if shadowing_item(item)));
        syn::visit::visit_block(self, block);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indexed(modules: &[(&[&str], &str)]) -> Index {
        let census = environment::tests::census();
        let modules: Vec<_> = modules
            .iter()
            .map(|(namespace, source)| census::Module {
                path: census.files[0].path.clone(),
                namespace: namespace.iter().map(|name| (*name).into()).collect(),
                items: syn::parse_file(source).unwrap().items,
                verified_macro_imports: Default::default(),
                verified_external_imports: Default::default(),
            })
            .collect();
        build_index(&modules, &census, None).unwrap()
    }

    #[test]
    fn checked_namespace_roots_require_present_unshadowed_source() {
        let root = Vec::new();
        let child = vec!["child".into()];
        let index = indexed(&[(&[], "fn value() {}"), (&["child"], "type u32 = Other;")]);
        index.primitive_root(&root).unwrap();
        assert!(index.primitive_root(&child).is_err());
        assert!(index.primitive_root(&vec!["missing".into()]).is_err());
        let imported = indexed(&[(&[], "use other::Word as u32;")]);
        assert!(imported.primitive_root(&root).is_err());
    }

    #[test]
    fn duplicate_namespace_shadow_cannot_be_overwritten_by_clean_source() {
        for modules in [
            [(&[][..], "type u32 = Other;"), (&[][..], "fn value() {}")],
            [(&[][..], "fn value() {}"), (&[][..], "type u32 = Other;")],
        ] {
            assert!(indexed(&modules).primitive_root(&Vec::new()).is_err());
        }
        indexed(&[(&[], "fn first() {}"), (&[], "fn second() {}")])
            .primitive_root(&Vec::new())
            .unwrap();
    }

    #[test]
    fn namespace_macro_invocations_keep_unknown_type_effects_closed() {
        assert!(
            indexed(&[(&[], "make_types!();")])
                .primitive_root(&Vec::new())
                .is_err()
        );
        indexed(&[(&[], "macro_rules! make_types { () => { struct u32; } }")])
            .primitive_root(&Vec::new())
            .unwrap();
    }

    fn check(source: &str) -> Result<(), String> {
        let start = source.rfind("u32::MAX").unwrap() as u32;
        local_source(
            &syn::parse_str(source).unwrap(),
            ByteSpan {
                start,
                end: start + 8,
            },
            source,
        )
    }

    #[test]
    fn local_type_and_import_shadows_are_never_inferred_primitive() {
        check("fn value()->u32 { u32::MAX }").unwrap();
        check("fn value()->u32 { { struct u32; } u32::MAX }").unwrap();
        check("fn value()->u32 { use other::u32::{self as Word}; u32::MAX }").unwrap();
        for source in [
            "fn value()->u32 { struct u32; u32::MAX }",
            "fn value()->u32 { use other::Word as u32; u32::MAX }",
            "fn value<u32>()->u32 { u32::MAX }",
            "fn value()->u32 { use other::*; u32::MAX }",
            "fn value()->u32 { struct r#u32; u32::MAX }",
            "fn value()->u32 { use other::u32::{self}; u32::MAX }",
            "fn value()->u32 { use other::{u32::{self}}; u32::MAX }",
        ] {
            assert!(check(source).is_err(), "{source}");
        }
    }

    #[test]
    fn owning_module_shadow_checks_leave_unrelated_children_closed() {
        for source in [
            "struct u32;",
            "mod u32 {}",
            "type u32 = Other;",
            "use other::Word as u32;",
            "make_types!();",
        ] {
            assert!(shadowing_item(&syn::parse_str(source).unwrap()));
        }
        assert!(!shadowing_item(
            &syn::parse_str("mod unrelated { struct u32; }").unwrap()
        ));
        assert!(!shadowing_item(
            &syn::parse_str("macro_rules! make_types { () => { struct u32; } }").unwrap()
        ));
    }
}
