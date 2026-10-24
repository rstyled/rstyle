//! Original consumer coordinates in an independently checked active module set.
use super::*;
use crate::{
    ByteSpan,
    nominal_boundary::census::{self, expansions::FileSpan},
};
use syn::spanned::Spanned;

/// Requires the complete, already source-checked active modules of this census.
/// The lexical-site join grants no foreign artifact or callable authority.
pub fn verify_consumer(
    census: &Census,
    modules: &[census::Module],
    receipt: &AnonymousPrelude,
) -> Result<(), ProofError> {
    let site = receipt.import();
    census
        .context
        .validate()
        .map_err(|_| ProofError::Contradictory("Invalid consumer source context"))?;
    if census.identity != census::IDENTITY {
        return Err(ProofError::Contradictory(
            "Invalid consumer census identity",
        ));
    }
    local_owner(census, site)?;
    let modules: Vec<_> = modules
        .iter()
        .filter(|module| module.namespace == site.scope)
        .collect();
    let [module] = modules.as_slice() else {
        return Err(ProofError::Missing(
            "Missing or ambiguous active consumer module",
        ));
    };
    if module.path != site.declaration.path {
        return Err(ProofError::Contradictory(
            "Consumer module source ownership differs",
        ));
    }
    let files: Vec<_> = census
        .files
        .iter()
        .filter(|file| file.path == module.path)
        .collect();
    let [file] = files.as_slice() else {
        return Err(ProofError::Missing(
            "Missing or ambiguous consumer source buffer",
        ));
    };
    if !file.bytes.matches_identity(&file.digest) {
        return Err(ProofError::Contradictory(
            "Consumer source checksum differs",
        ));
    }

    original_use(module, &file.bytes, site)
}

fn local_owner(census: &Census, site: &Site) -> Result<(), ProofError> {
    let namespace = site.scope.join("::");
    if !site.owner.local
        || !site.definition.local
        || site.owner.path != namespace
        || site.owner.crate_name != census.context.crate_name
        || site.definition.crate_name != site.owner.crate_name
        || site.owner.stable_crate_id.is_empty()
        || site.owner.definition_identity.is_empty()
        || site.definition.stable_crate_id != site.owner.stable_crate_id
        || site.definition.definition_identity.is_empty()
    {
        return Err(ProofError::Contradictory(
            "Consumer import definition or module owner differs",
        ));
    }

    let suffix = if namespace.is_empty() {
        Some(site.definition.path.as_str())
    } else {
        site.definition.path.strip_prefix(&format!("{namespace}::"))
    };
    if suffix.is_none_or(|suffix| !use_definition(suffix)) {
        return Err(ProofError::Contradictory(
            "Consumer use definition is outside its original module",
        ));
    }
    Ok(())
}

fn use_definition(path: &str) -> bool {
    !path.is_empty()
        && path.split("::").all(|part| {
            part.strip_prefix("{use#")
                .and_then(|part| part.strip_suffix('}'))
                .is_some_and(|ordinal| {
                    !ordinal.is_empty() && ordinal.bytes().all(|byte| byte.is_ascii_digit())
                })
        })
}

fn original_use(
    module: &census::Module,
    bytes: &crate::SourceSnapshot,
    site: &Site,
) -> Result<(), ProofError> {
    let uses: Vec<_> = module
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Use(item)
                if crate::source_evidence::original_span(bytes, item.span())
                    == site.declaration.span =>
            {
                Some(item)
            }
            _ => None,
        })
        .collect();
    let [item] = uses.as_slice() else {
        return Err(ProofError::Missing(
            "Missing or ambiguous original consumer use declaration",
        ));
    };
    if !item.attrs.is_empty() {
        return Err(ProofError::Unsupported(
            "Consumer namespace attributes require independent active-source evidence",
        ));
    }
    coordinate(
        bytes,
        &site.declaration,
        crate::source_evidence::original_span(bytes, item.span()),
    )?;
    if site.public != matches!(item.vis, syn::Visibility::Public(_))
        || site.binding != Binding::Glob
    {
        return Err(ProofError::Contradictory(
            "Consumer original import visibility or binding differs",
        ));
    }
    if item.leading_colon.is_some() {
        return Err(ProofError::Unsupported(
            "Absolute consumer namespace import is unavailable",
        ));
    }

    original_leaf(bytes, &item.tree, site)
}

fn original_leaf(
    bytes: &crate::SourceSnapshot,
    tree: &syn::UseTree,
    site: &Site,
) -> Result<(), ProofError> {
    let mut leaves = GlobLeaves {
        bytes,
        output: Vec::new(),
        visited: 0,
    };
    leaves.walk(tree, &[], 0)?;

    let matching: Vec<_> = leaves
        .output
        .iter()
        .filter(|leaf| leaf.span == site.leaf.span)
        .collect();
    let [leaf] = matching.as_slice() else {
        return Err(ProofError::Missing(
            "Missing or ambiguous original consumer glob leaf",
        ));
    };
    coordinate(bytes, &site.leaf, leaf.span)?;
    if site.leaf.path != site.declaration.path || site.path != leaf.path {
        return Err(ProofError::Contradictory(
            "Consumer original glob path differs",
        ));
    }
    Ok(())
}

fn coordinate(
    bytes: &crate::SourceSnapshot,
    observed: &FileSpan,
    span: ByteSpan,
) -> Result<(), ProofError> {
    if observed.span != span || !bytes.matches_identity(&observed.digest) {
        return Err(ProofError::Contradictory(
            "Consumer original source coordinates differ",
        ));
    }
    Ok(())
}

struct GlobLeaf {
    path: Vec<String>,
    span: ByteSpan,
}

struct GlobLeaves<'a> {
    bytes: &'a crate::SourceSnapshot,
    output: Vec<GlobLeaf>,
    visited: usize,
}

impl GlobLeaves<'_> {
    fn walk(
        &mut self,
        tree: &syn::UseTree,
        inherited: &[String],
        depth: usize,
    ) -> Result<(), ProofError> {
        self.visited += 1;
        if depth >= census::MAX_FILES || self.visited > crate::data_receivers::MAX_RECORDS {
            return Err(ProofError::Unsupported(
                "Consumer use tree exceeds finite source budget",
            ));
        }
        let prefix = self.prefix(tree, inherited)?;

        match prefix.remaining {
            syn::UseTree::Glob(_) => {
                let span = prefix.span.ok_or(ProofError::Unsupported(
                    "Bare grouped glob lacks a pinned original prefix span",
                ))?;
                self.output.push(GlobLeaf {
                    path: prefix.path,
                    span,
                });
            }
            syn::UseTree::Group(group) => {
                for child in &group.items {
                    self.walk(child, &prefix.path, depth + 1)?;
                }
            }
            syn::UseTree::Name(_) | syn::UseTree::Rename(_) => {}
            syn::UseTree::Path(_) => {
                return Err(ProofError::Contradictory("Unconsumed original use path"));
            }
        }
        Ok(())
    }

    fn prefix<'a>(
        &self,
        tree: &'a syn::UseTree,
        inherited: &[String],
    ) -> Result<Prefix<'a>, ProofError> {
        let mut path = inherited.to_vec();
        let mut cursor = tree;
        let mut prefix = None;

        while let syn::UseTree::Path(segment) = cursor {
            let span = crate::source_evidence::original_span(self.bytes, segment.ident.span());
            prefix = Some(prefix.map_or(span, |first: ByteSpan| ByteSpan {
                start: first.start,
                end: span.end,
            }));
            path.push(segment.ident.to_string());
            cursor = &segment.tree;
            if path.len() > census::MAX_FILES {
                return Err(ProofError::Unsupported(
                    "Consumer use path exceeds finite source budget",
                ));
            }
        }
        Ok(Prefix {
            path,
            span: prefix,
            remaining: cursor,
        })
    }
}

struct Prefix<'a> {
    path: Vec<String>,
    span: Option<ByteSpan>,
    remaining: &'a syn::UseTree,
}

#[cfg(test)]
mod tests;
