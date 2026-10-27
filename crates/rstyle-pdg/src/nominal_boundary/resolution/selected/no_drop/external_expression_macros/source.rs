use super::*;
use crate::{ByteSpan, nominal_boundary::census::UnitFile};
use syn::spanned::Spanned;

pub(super) const MODULE: &str = include_str!("smart_display.rs.txt");
pub(super) const CORRECTED_MODULE: &str = include_str!("smart_display.rs.021.txt");
pub(super) const CORRECTED_ROOT: &str = include_str!("lib.rs.021.txt");
const ROOT: &str = include_str!("lib.rs.txt");
const DEFINITION: &str = "__not_public_at_root__padded_width_of";

pub(super) struct Template {
    definition: ByteSpan,
    reexport: ByteSpan,
    leaf: ByteSpan,
}

pub(super) fn definition(census: &Census, review: Review<'_>) -> Result<FileSpan, String> {
    let path = census
        .context
        .root_source
        .parent()
        .ok_or("Missing macro source root")?
        .join("smart_display.rs");
    let module = file(census, &path)?;
    Ok(FileSpan {
        path: module.path.clone(),
        digest: module.digest.clone(),
        span: template_reviewed(&module.bytes, review)?.definition,
    })
}

pub(super) fn verify(input: &Input<'_>) -> Result<(), String> {
    input.census.context.validate()?;
    if input.census.identity != crate::nominal_boundary::census::IDENTITY
        || input.namespaces.identity != crate::dependency_namespaces::capture::IDENTITY
    {
        return Err("External macro requires original source and namespace records".into());
    }

    let root = file(&input.census, &input.census.context.root_source)?;

    let path = root
        .path
        .parent()
        .ok_or("Missing macro source root")?
        .join("smart_display.rs");
    let module = file(&input.census, &path)?;
    let paired = root.bytes.as_ref() == ROOT && module.bytes.as_ref() == MODULE
        || input.review.corrected
            && root.bytes.as_ref() == CORRECTED_ROOT
            && module.bytes.as_ref() == CORRECTED_MODULE;
    if !paired {
        return Err("External macro root/module source pair differs from reviewed release".into());
    }
    let template = template_reviewed(&module.bytes, input.review)?;

    if input.definition.path != module.path
        || input.definition.digest != module.digest
        || input.definition.span != template.definition
    {
        return Err("External macro definition coordinates disagree with original item".into());
    }

    reexport(input, module, &template)
}

fn file<'a>(census: &'a Census, path: &std::path::Path) -> Result<&'a UnitFile, String> {
    let found: Vec<_> = census
        .files
        .iter()
        .filter(|file| file.path == path)
        .collect();
    let [file] = found.as_slice() else {
        return Err("Missing or duplicate external macro source file".into());
    };
    if !file.bytes.matches_identity(&file.digest) {
        return Err("External macro source identity changed".into());
    }
    Ok(file)
}

/// Exact complete module bytes retain every matcher/transcriber, including the
/// original alternate -> with_width arm. No rule is inferred from its name.
#[cfg(test)]
pub(super) fn template(bytes: &str, cfgs: &[String]) -> Result<Template, String> {
    template_reviewed(
        bytes,
        Review {
            cfgs,
            corrected: false,
        },
    )
}

fn template_reviewed(bytes: &str, review: Review<'_>) -> Result<Template, String> {
    if bytes != MODULE && !(review.corrected && bytes == CORRECTED_MODULE) {
        return Err("External expression macro module differs from reviewed source".into());
    }

    let file = syn::parse_file(bytes).map_err(|error| error.to_string())?;
    let configuration = crate::conditional_source::Configuration::new(review.cfgs);
    let mut definition = None;
    let mut reexport = None;

    for item in &file.items {
        if !configuration.active(crate::conditional_source::item_attributes(item))? {
            continue;
        }
        match item {
            syn::Item::Macro(item)
                if item.ident.as_ref().is_some_and(|name| name == DEFINITION) =>
            {
                rules(item.mac.tokens.clone())?;
                definition = Some(crate::source_evidence::original_span(bytes, item.span()));
            }
            syn::Item::Use(item)
                if matches!(&item.tree, syn::UseTree::Rename(rename)
                if rename.ident == DEFINITION && rename.rename == "padded_width_of") =>
            {
                let syn::UseTree::Rename(rename) = &item.tree else {
                    unreachable!()
                };
                reexport = Some(reexport_spans(bytes, item, rename));
            }
            _ => {}
        }
    }

    let (reexport, leaf) = reexport.ok_or("Actual external macro reexport is inactive")?;
    Ok(Template {
        definition: definition.ok_or("Actual external macro is inactive")?,
        reexport,
        leaf,
    })
}

fn reexport_spans(
    bytes: &str,
    item: &syn::ItemUse,
    rename: &syn::UseRename,
) -> (ByteSpan, ByteSpan) {
    // rustc's original use declaration excludes its cfg attributes.
    // Their activity was checked against the complete item before this join.
    let mut declaration = item.clone();
    declaration.attrs.clear();

    (
        crate::source_evidence::original_span(bytes, declaration.span()),
        crate::source_evidence::original_span(bytes, rename.ident.span()),
    )
}

fn rules(tokens: proc_macro2::TokenStream) -> Result<(), String> {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    let mut count = 0;

    for rule in tokens.chunks(5) {
        let [
            proc_macro2::TokenTree::Group(_),
            equal,
            arrow,
            proc_macro2::TokenTree::Group(_),
            separator,
        ] = rule
        else {
            return Err("External macro rule inventory changed".into());
        };
        if equal.to_string() != "=" || arrow.to_string() != ">" || separator.to_string() != ";" {
            return Err("External macro rule inventory changed".into());
        }
        count += 1;
    }

    if count != 14 {
        return Err("External macro requires all fourteen original rules".into());
    }

    Ok(())
}

fn reexport(input: &Input<'_>, module: &UnitFile, template: &Template) -> Result<(), String> {
    let [route] = input.export.route.as_slice() else {
        return Err("External macro needs one exact original reexport".into());
    };
    if !matches!(
        route.kind,
        crate::dependency_namespaces::capture::RouteKind::Single
    ) || route.declaration.local
        || !input.export.public
        || input.export.target.local
        || input.export.binding
            != crate::dependency_namespaces::capture::Binding::Named("padded_width_of".into())
        || input.export.target_kind != "Macro(MacroKinds(1))"
    {
        return Err("External macro export route changes its original bang macro".into());
    }

    let sites: Vec<_> = input
        .namespaces
        .declarations
        .iter()
        .filter(|entry| identity(&entry.site.definition, &route.declaration))
        .collect();
    let [entry] = sites.as_slice() else {
        return Err("External macro lacks unique original namespace reexport".into());
    };
    target(input, module, template, &entry.site)
}

fn target(
    input: &Input<'_>,
    module: &UnitFile,
    template: &Template,
    site: &crate::dependency_namespaces::capture::Site,
) -> Result<(), String> {
    if site.declaration.path != module.path
        || site.declaration.digest != module.digest
        || site.declaration.span != template.reexport
        || site.leaf.path != module.path
        || site.leaf.digest != module.digest
        || site.leaf.span != template.leaf
        || !site.public
        || site.scope != ["smart_display"]
        || site.binding
            != crate::dependency_namespaces::capture::Binding::Named("padded_width_of".into())
        || site.path != [DEFINITION]
        || !macro_identity(&site.target, &input.export.target)
        || !site.target.local
        || site.target.crate_name != input.census.context.crate_name
        || !site.definition.local
        || site.target_kind != input.export.target_kind
        || !site.owner.local
        || relative_path(&site.owner) != "smart_display"
        || site.owner.crate_name != site.target.crate_name
        || site.owner.stable_crate_id != site.target.stable_crate_id
        || site.definition.crate_name != site.owner.crate_name
        || site.definition.stable_crate_id != site.owner.stable_crate_id
    {
        return Err("External macro namespace target differs from original source reexport".into());
    }
    Ok(())
}

// rustc retains the defining module in the local macro DefPath, but presents
// this #[macro_export] target at the foreign crate root. Both retain one DefId.
fn macro_identity(local: &Definition, external: &Definition) -> bool {
    same_definition(local, external)
        && matches!(
            relative_path(local),
            DEFINITION | "smart_display::__not_public_at_root__padded_width_of"
        )
        && relative_path(external) == DEFINITION
}

fn same_definition(left: &Definition, right: &Definition) -> bool {
    !left.definition_identity.is_empty()
        && !left.stable_crate_id.is_empty()
        && left.crate_name == right.crate_name
        && left.stable_crate_id == right.stable_crate_id
        && left.definition_identity == right.definition_identity
}

fn relative_path(definition: &Definition) -> &str {
    definition
        .path
        .strip_prefix(&format!("{}::", definition.crate_name))
        .unwrap_or(&definition.path)
}

pub(super) fn identity(left: &Definition, right: &Definition) -> bool {
    same_definition(left, right) && relative_path(left) == relative_path(right)
}
