//! Complete selected inline module and exact original grouped/leaf coordinates.
use super::*;
use crate::{ByteSpan, SourceSnapshot, nominal_boundary::census::expansions::FileSpan};
use syn::spanned::Spanned;

pub(super) struct Prelude {
    bytes: SourceSnapshot,
    path: std::path::PathBuf,
    module: syn::ItemMod,
    root: syn::File,
    crate_name: String,
}

impl Prelude {
    pub(super) fn original(census: &Census) -> Result<Self, ProofError> {
        let files: Vec<_> = census
            .files
            .iter()
            .filter(|file| file.path == census.context.root_source)
            .collect();
        let [file] = files.as_slice() else {
            return Err(ProofError::Missing(
                "Missing or duplicate original producer root",
            ));
        };
        if !file.bytes.matches_identity(&file.digest) {
            return Err(ProofError::Contradictory("Producer root checksum differs"));
        }
        let root = syn::parse_file(&file.bytes)
            .map_err(|_| ProofError::Unsupported("Producer root is not original Rust source"))?;
        root_attributes(&root.attrs)?;
        let modules: Vec<_> = root
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Mod(module) if module.ident == "prelude" => Some(module),
                _ => None,
            })
            .collect();
        let [module] = modules.as_slice() else {
            return Err(ProofError::Missing(
                "Missing or ambiguous original prelude module",
            ));
        };
        public(&module.vis, &module.attrs)?;
        if module.content.is_none() || module.unsafety.is_some() {
            return Err(ProofError::Unsupported(
                "Prelude requires an original safe inline module",
            ));
        }

        Ok(Self {
            bytes: file.bytes.clone(),
            path: file.path.clone(),
            module: (*module).clone(),
            root,
            crate_name: census.context.crate_name.clone(),
        })
    }

    pub(super) fn declarations<'a>(
        &self,
        producer: &'a UnitCapture,
    ) -> Result<[&'a Site; 2], ProofError> {
        let items = &self
            .module
            .content
            .as_ref()
            .ok_or(ProofError::Missing("Missing prelude body"))?
            .1;
        let [syn::Item::Use(item)] = items.as_slice() else {
            return Err(ProofError::Unsupported(
                "Prelude must contain exactly one original grouped reexport",
            ));
        };
        public(&item.vis, &item.attrs)?;
        let leaves = grouped(item)?;
        let sites = [
            self.site(producer, item, leaves[0])?,
            self.site(producer, item, leaves[1])?,
        ];
        self.complete(producer, &sites)?;
        self.original_trait("Extend")?;
        self.original_trait("Truncate")?;

        Ok(sites)
    }

    fn site<'a>(
        &self,
        producer: &'a UnitCapture,
        item: &syn::ItemUse,
        leaf: &syn::UseRename,
    ) -> Result<&'a Site, ProofError> {
        let declaration = crate::source_evidence::original_span(&self.bytes, item.span());
        // The pinned compiler records the use identifier, while the complete
        // original grouped declaration proves its exact `as _` rename.
        let leaf_span = crate::source_evidence::original_span(&self.bytes, leaf.ident.span());
        let records: Vec<_> = producer
            .declarations
            .iter()
            .filter(|record| {
                record.site.declaration.path == self.path
                    && record.site.declaration.span == declaration
                    && record.site.leaf.span == leaf_span
            })
            .collect();
        let [record] = records.as_slice() else {
            return Err(ProofError::Missing(
                "Missing or ambiguous original anonymous reexport leaf",
            ));
        };
        let site = &record.site;
        self.coordinates(&site.declaration, declaration)?;
        self.coordinates(&site.leaf, leaf_span)?;
        self.identities(site, &leaf.ident.to_string())?;

        Ok(site)
    }

    fn coordinates(&self, observed: &FileSpan, expected: ByteSpan) -> Result<(), ProofError> {
        if observed.path != self.path
            || observed.span != expected
            || !self.bytes.matches_identity(&observed.digest)
        {
            return Err(ProofError::Contradictory(
                "Original grouped/leaf source coordinates differ",
            ));
        }
        Ok(())
    }

    fn identities(&self, site: &Site, name: &str) -> Result<(), ProofError> {
        for definition in [&site.definition, &site.owner, &site.target] {
            identity(definition)?;
            if !definition.local
                || definition.crate_name != self.crate_name
                || definition.stable_crate_id != site.owner.stable_crate_id
            {
                return Err(ProofError::Contradictory(
                    "Original namespace identities have different owners",
                ));
            }
        }
        if site.scope != ["prelude"]
            || site.path != ["crate", name]
            || site.binding != Binding::Anonymous
            || !site.public
            || site.target_kind != "Trait"
            || site.owner.path != "prelude"
            || !site
                .definition
                .path
                .starts_with(&format!("{}::", site.owner.path))
            || site.target.path != name
        {
            return Err(ProofError::Contradictory(
                "Original trait reexport resolution differs",
            ));
        }
        Ok(())
    }

    fn complete(&self, producer: &UnitCapture, sites: &[&Site; 2]) -> Result<(), ProofError> {
        let owner = &sites[0].owner;
        if sites[1].owner != *owner
            || sites[0].definition == sites[1].definition
            || sites[0].target == sites[1].target
            || sites[0].definition.definition_identity == sites[1].definition.definition_identity
            || sites[0].target.definition_identity == sites[1].target.definition_identity
        {
            return Err(ProofError::Contradictory(
                "Anonymous reexports have duplicate or different owners",
            ));
        }

        let in_module = |site: &Site| {
            site.owner == *owner || site.scope.first().is_some_and(|name| name == "prelude")
        };
        if producer
            .declarations
            .iter()
            .filter(|record| in_module(&record.site))
            .count()
            != 2
            || producer
                .imports
                .iter()
                .any(|record| in_module(&record.site))
        {
            return Err(ProofError::Contradictory(
                "Selected module compiler declaration footprint is incomplete",
            ));
        }
        Ok(())
    }

    fn original_trait(&self, name: &str) -> Result<(), ProofError> {
        let traits: Vec<_> = self
            .root
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Trait(item) if item.ident == name => Some(item),
                _ => None,
            })
            .collect();
        let [item] = traits.as_slice() else {
            return Err(ProofError::Missing(
                "Missing or ambiguous original exported trait declaration",
            ));
        };
        public(&item.vis, &item.attrs)
    }
}

fn root_attributes(attributes: &[syn::Attribute]) -> Result<(), ProofError> {
    for attribute in attributes {
        let supported = match &attribute.meta {
            syn::Meta::Path(path) => path.is_ident("no_std"),
            syn::Meta::NameValue(value) if value.path.is_ident("doc") => {
                matches!(&value.value, syn::Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Str(_)))
            }
            _ => false,
        };
        if !supported {
            return Err(ProofError::Unsupported(
                "Producer root has unsupported namespace attributes",
            ));
        }
    }
    Ok(())
}

fn public(visibility: &syn::Visibility, attributes: &[syn::Attribute]) -> Result<(), ProofError> {
    if !matches!(visibility, syn::Visibility::Public(_))
        || attributes
            .iter()
            .any(|attribute| !attribute.path().is_ident("doc"))
    {
        return Err(ProofError::Unsupported(
            "Namespace declaration requires public visibility and only original documentation attributes",
        ));
    }
    Ok(())
}

fn grouped(item: &syn::ItemUse) -> Result<[&syn::UseRename; 2], ProofError> {
    let syn::UseTree::Path(path) = &item.tree else {
        return Err(ProofError::Unsupported(
            "Prelude requires crate-qualified original grouped imports",
        ));
    };
    let syn::UseTree::Group(group) = path.tree.as_ref() else {
        return Err(ProofError::Unsupported(
            "Prelude requires one exact grouped import",
        ));
    };
    if item.leading_colon.is_some() || path.ident != "crate" || group.items.len() != 2 {
        return Err(ProofError::Unsupported(
            "Prelude group has unsupported prefix or inventory",
        ));
    }

    let leaves: Vec<_> = group.items.iter().collect();
    let (syn::UseTree::Rename(first), syn::UseTree::Rename(second)) = (leaves[0], leaves[1]) else {
        return Err(ProofError::Unsupported(
            "Prelude requires exactly two anonymous trait reexports",
        ));
    };
    if first.ident != "Extend"
        || second.ident != "Truncate"
        || first.rename != "_"
        || second.rename != "_"
    {
        return Err(ProofError::Unsupported(
            "Prelude anonymous trait identities or order differ",
        ));
    }
    Ok([first, second])
}
