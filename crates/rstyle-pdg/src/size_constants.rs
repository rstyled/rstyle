//! Function-local native byte width; initializer authority is compile-time only.
mod annotations;
use crate::{HirNode, scalar_constants::ScalarConstant, unit_store::EvidenceFacts};
use serde::{Deserialize, Serialize};

pub const RESOLUTION: &str = "resolved_native_size_constant";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeSize {
    pub constant: ScalarConstant,
    pub initializer: crate::data_library::Callee,
    pub argument: String,
}

pub(crate) fn verify_function(facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !facts
        .body
        .children
        .iter()
        .any(|child| child.kind == "stmt_size_const")
    {
        return Ok(());
    }
    let mut definitions = std::collections::BTreeSet::new();
    for node in facts
        .body
        .children
        .iter()
        .filter(|child| child.kind == "stmt_size_const")
    {
        verify(node, facts)?;
        let record = crate::scalar_constants::decode_record::<NativeSize>(
            node.res.as_ref().ok_or("Missing size declaration")?,
            RESOLUTION,
        )?;
        if !definitions.insert(record.constant.definition) {
            return Err("Duplicate local size declaration".into());
        }
    }

    for node in crate::field_locations::operations(&facts.body, &["ty_annotation", "expr_path"]) {
        annotations::verify_annotation(node, facts)?;
        verify_use(node, facts)?;
    }
    Ok(())
}

pub fn declaration(source: &str) -> Result<syn::ItemConst, String> {
    let item: syn::ItemConst = syn::parse_str(source).map_err(|error| error.to_string())?;
    if !item.attrs.is_empty()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
        || !matches!(item.vis, syn::Visibility::Inherited)
        || !matches!(&*item.ty, syn::Type::Path(ty) if ty.path.is_ident("usize"))
    {
        return Err("Native size requires an unadorned local const usize".into());
    }

    let syn::Expr::Call(call) = &*item.expr else {
        return Err("Native size requires size_of".into());
    };
    let syn::Expr::Path(path) = &*call.func else {
        return Err("Native size requires a resolved path".into());
    };
    let names: Vec<_> = path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    if !call.args.is_empty()
        || path.qself.is_some()
        || path.path.leading_colon.is_some()
        || names != ["core", "mem", "size_of"]
        || path
            .path
            .segments
            .iter()
            .take(2)
            .any(|segment| !segment.arguments.is_empty())
    {
        return Err("Native size requires the complete core::mem::size_of invocation".into());
    }

    let syn::PathArguments::AngleBracketed(arguments) = &path.path.segments[2].arguments else {
        return Err("Native size requires the concrete usize argument".into());
    };
    if arguments.args.len() != 1
        || !matches!(&arguments.args[0], syn::GenericArgument::Type(syn::Type::Path(ty)) if ty.qself.is_none() && ty.path.is_ident("usize"))
    {
        return Err("Native size requires exactly usize".into());
    }
    Ok(item)
}

pub(crate) fn verify(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if !crate::extraction_profile::ExtractionProfile::from_facts(facts)?
        .supports("rust.native-borrowed-byte-readers")
        || node.kind != "stmt_size_const"
        || node.expansion.is_some()
        || !node.children.is_empty()
        || !facts
            .body
            .children
            .iter()
            .any(|child| child.hir == node.hir)
    {
        return Err("Native size requires an original top-level local constant".into());
    }

    let record = crate::scalar_constants::decode_record::<NativeSize>(
        node.res.as_ref().ok_or("Missing size constant evidence")?,
        RESOLUTION,
    )?;
    let bytes = facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or("Missing size declaration source")?;
    let item = declaration(bytes)?;
    let name = item.ident.to_string();
    if record.constant.declaration != bytes
        || record.constant.ty != "usize"
        || record.constant.value != "8"
        || record.argument != "usize"
        || facts.build.pointer_width.value != Some(64)
        || record.constant.definition != format!("{}::{name}", facts.name)
    {
        return Err("Native size value, owner or target disagrees".into());
    }
    authority(&record.initializer, facts)
}

fn authority(
    callee: &crate::data_library::Callee,
    facts: &dyn EvidenceFacts,
) -> Result<(), String> {
    let registry =
        crate::data_library::registry_for(facts).ok_or("Unpinned native size authority")?;
    if callee.crate_name != "core"
        || callee.crate_hash != registry.core_hash
        || callee.path != "std::mem::size_of"
        || registry.definitions.get(&callee.path) != Some(&callee.definition_hash)
        || callee.implementation.is_some()
    {
        return Err("Native size initializer lacks pinned core authority".into());
    }
    Ok(())
}

pub(crate) fn verify_use(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<(), String> {
    if node.kind != "expr_path"
        || node
            .res
            .as_ref()
            .is_none_or(|res| res.kind != crate::scalar_constants::RESOLUTION)
    {
        return Ok(());
    }
    for declaration in facts
        .body
        .children
        .iter()
        .filter(|child| child.kind == "stmt_size_const")
    {
        let size = crate::scalar_constants::decode_record::<NativeSize>(
            declaration
                .res
                .as_ref()
                .ok_or("Missing local size declaration")?,
            RESOLUTION,
        )?;
        let resolved = ScalarConstant::decode(node.res.as_ref().ok_or("Missing scalar constant")?)?;
        if resolved.definition == size.constant.definition {
            verify(declaration, facts)?;
            let source = facts
                .source
                .bytes
                .get(node.span.start as usize..node.span.end as usize)
                .ok_or("Missing size use source")?;
            let path: syn::ExprPath = syn::parse_str(source).map_err(|error| error.to_string())?;
            let name = resolved
                .definition
                .rsplit("::")
                .next()
                .ok_or("Missing constant name")?;
            if path.qself.is_some() || !path.path.is_ident(name) {
                return Err("Size constant use loses its exact lexical name".into());
            }
            if resolved != size.constant {
                return Err("Local size use contradicts its complete declaration".into());
            }
        }
    }
    Ok(())
}
