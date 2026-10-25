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
        names: &[&str],
    ) -> Result<Vec<&'a Site>, ProofError> {
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
        let widening = names.len() == 3;
        metadata_public(&item.vis, &item.attrs, widening)?;
        let leaves = grouped(item, names)?;
        let sites = leaves
            .iter()
            .map(|leaf| self.site(producer, item, leaf))
            .collect::<Result<Vec<_>, _>>()?;
        self.complete(producer, &sites)?;
        for name in names {
            self.original_trait(name, widening)?;
        }

        Ok(sites)
    }

    fn site<'a>(
        &self,
        producer: &'a UnitCapture,
        item: &syn::ItemUse,
        leaf: &syn::UseRename,
    ) -> Result<&'a Site, ProofError> {
        let mut original = item.clone();
        original.attrs.clear();
        let declaration = crate::source_evidence::original_span(&self.bytes, original.span());
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

    fn complete(&self, producer: &UnitCapture, sites: &[&Site]) -> Result<(), ProofError> {
        let owner = &sites[0].owner;
        if sites.iter().enumerate().any(|(index, site)| {
            site.owner != *owner
                || sites[..index].iter().any(|other| {
                    site.definition == other.definition
                        || site.target == other.target
                        || site.definition.definition_identity
                            == other.definition.definition_identity
                        || site.target.definition_identity == other.target.definition_identity
                })
        }) {
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
            != sites.len()
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

    fn original_trait(&self, name: &str, widening: bool) -> Result<(), ProofError> {
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
        metadata_public(&item.vis, &item.attrs, widening)
    }
}

fn root_attributes(attributes: &[syn::Attribute]) -> Result<(), ProofError> {
    if !crate::source_evidence::literal_metadata(attributes, "no_std") {
        return Err(ProofError::Unsupported(
            "Producer root has unsupported namespace attributes",
        ));
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

fn metadata_public(
    visibility: &syn::Visibility,
    attributes: &[syn::Attribute],
    widening: bool,
) -> Result<(), ProofError> {
    let ordinary: Vec<_> = attributes
        .iter()
        .filter(|attribute| {
            !(widening
                && (crate::source_evidence::single_identifier_attribute(
                    &attribute.meta,
                    "allow",
                    "deprecated",
                ) || crate::nominal_boundary::resolution::deprecated_metadata(&attribute.meta)))
        })
        .cloned()
        .collect();
    public(visibility, &ordinary)
}

fn grouped<'a>(
    item: &'a syn::ItemUse,
    names: &[&str],
) -> Result<Vec<&'a syn::UseRename>, ProofError> {
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
    if item.leading_colon.is_some() || path.ident != "crate" || group.items.len() != names.len() {
        return Err(ProofError::Unsupported(
            "Prelude group has unsupported prefix or inventory",
        ));
    }
    group
        .items
        .iter()
        .zip(names)
        .map(|(leaf, name)| match leaf {
            syn::UseTree::Rename(leaf) if leaf.ident == name && leaf.rename == "_" => Ok(leaf),
            _ => Err(ProofError::Unsupported(
                "Prelude anonymous trait identities or order differ",
            )),
        })
        .collect()
}
