//! File/range handles select existing parsed AST nodes without reparsing source files.
use super::*;
use crate::nominal_constants::SourceUnit;
use syn::spanned::Spanned;

pub(super) fn sources<'a>(
    resolution: &'a Resolution,
    record: &ProductConstant,
) -> Result<SourceSelection<'a>, String> {
    let constant_unit = implementation(resolution, &record.implementation_source, record)?;
    let constructor_unit = implementation(
        resolution,
        &record.constructor.implementation_source,
        record,
    )?;

    let constant = one(constant_unit
        .implementation
        .items
        .iter()
        .filter_map(|item| {
            let syn::ImplItem::Const(item) = item else {
                return None;
            };
            (constant_span(constant_unit.bytes, item) == record.declaration.span).then_some(item)
        }))?;
    let constructor = one(constructor_unit
        .implementation
        .items
        .iter()
        .filter_map(|item| {
            let syn::ImplItem::Fn(item) = item else {
                return None;
            };
            (function_span(constructor_unit.bytes, item) == record.constructor.declaration.span)
                .then_some(item)
        }))?;

    if constant_unit.file != record.declaration.file
        || constructor_unit.file != record.constructor.declaration.file
    {
        return Err("Product constant declarations cross their observed original files".into());
    }
    Ok(SourceSelection {
        constant_unit,
        constant,
        constructor_unit,
        constructor,
    })
}

fn implementation<'a>(
    resolution: &'a Resolution,
    site: &SourceSite,
    record: &ProductConstant,
) -> Result<SourceUnit<'a>, String> {
    let expected = namespace(record)?;
    let mut found = None;
    for module in resolution.environment.modules.iter().filter(|module| {
        module.path == std::path::Path::new(&site.file) && module.namespace == expected
    }) {
        let bytes = resolution.environment.source(&module.path)?.bytes();
        for item in &module.items {
            if let syn::Item::Impl(item) = item
                && implementation_span(bytes, item) == site.span
            {
                if found.is_some() {
                    return Err("Product constant has ambiguous original impl ownership".into());
                }
                found = Some(SourceUnit {
                    file: module
                        .path
                        .to_str()
                        .ok_or("Non-UTF8 product constant source path")?,
                    bytes,
                    implementation: item,
                });
            }
        }
    }
    found.ok_or_else(|| "Missing exact original product constant implementation".into())
}

pub(super) fn namespace(record: &ProductConstant) -> Result<Namespace, String> {
    let (nominal, _) = record
        .ty
        .nominal_parts()
        .ok_or("Product constant lacks nominal owner")?;
    let mut names: Namespace = nominal
        .definition_path
        .split("::")
        .map(str::to_owned)
        .collect();
    if names.iter().any(String::is_empty) || names.pop().is_none() {
        return Err("Product constant has malformed original nominal namespace".into());
    }
    Ok(names)
}

pub(super) fn owner_declaration(
    resolution: &Resolution,
    record: &ProductConstant,
) -> Result<SourceSite, String> {
    let (nominal, _) = record
        .ty
        .nominal_parts()
        .ok_or("Missing product constant nominal owner")?;
    let names: Namespace = nominal
        .definition_path
        .split("::")
        .map(str::to_owned)
        .collect();
    let declaration = resolution
        .environment
        .index
        .declarations
        .get(&names)
        .ok_or("Missing independently checked original product declaration")?;
    resolution.check_nominal(&names, nominal)?;

    Ok(SourceSite {
        file: declaration
            .path
            .to_str()
            .ok_or("Non-UTF8 product declaration source path")?
            .into(),
        span: declaration.span,
    })
}

fn implementation_span(bytes: &str, item: &syn::ItemImpl) -> ByteSpan {
    ByteSpan {
        start: source_span(bytes, item.impl_token.span()).start,
        end: source_span(bytes, item.brace_token.span.close()).end,
    }
}

fn constant_span(bytes: &str, item: &syn::ImplItemConst) -> ByteSpan {
    let start = if matches!(item.vis, syn::Visibility::Inherited) {
        item.const_token.span()
    } else {
        item.vis.span()
    };
    ByteSpan {
        start: source_span(bytes, start).start,
        end: source_span(bytes, item.semi_token.span()).end,
    }
}

fn function_span(bytes: &str, item: &syn::ImplItemFn) -> ByteSpan {
    FunctionSource {
        vis: &item.vis,
        sig: &item.sig,
        body: &item.block,
    }
    .range(bytes)
}

fn one<T>(mut candidates: impl Iterator<Item = T>) -> Result<T, String> {
    let selected = candidates
        .next()
        .ok_or("Missing exact original product constant member")?;
    if candidates.next().is_some() {
        return Err("Ambiguous exact original product constant member".into());
    }
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_or_ambiguous_source_handles_cannot_select_a_supported_prefix() {
        assert!(one(std::iter::empty::<usize>()).is_err());
        assert_eq!(one([4].into_iter()).unwrap(), 4);
        assert!(one([4, 5].into_iter()).is_err());
    }
}
