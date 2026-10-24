//! Byte-bound raw generated declarations; semantic ownership is checked separately.
use super::*;
use crate::generated_sources::capture::{Mark, Transparency};

pub(crate) fn shape(tree: &TypeTree) -> Result<(), String> {
    let Some((nominal, definition, template, fields)) = tree.generated_parts() else {
        return Err("Expected generated struct observation".into());
    };

    if !definition.local
        || definition.path != nominal.definition_path
        || definition.definition_identity.is_empty()
        || definition.crate_name.is_empty()
        || definition.stable_crate_id.is_empty()
        || template.is_empty()
        || template.len() > crate::concrete_types::MAX_SOURCE_BYTES
        || nominal.package_name.is_empty()
        || nominal.package_version.is_empty()
        || crate::source_identity::Scheme::parse(&nominal.manifest_digest).is_none()
        || !template.matches_identity(&nominal.declaration_digest)
    {
        return Err("Generated declaration lacks exact template or nominal identity".into());
    }
    let [field] = fields else {
        return Err("Generated struct observation requires one tuple field".into());
    };
    if field.position != 0
        || field.name != "0"
        || field.declaration.is_empty()
        || field.declaration.len() > crate::concrete_types::MAX_SOURCE_BYTES
        || field.declaration.contains('$')
    {
        return Err("Generated tuple field lacks concrete original type bytes".into());
    }
    field_type(&field.declaration)?;
    if template_field(template)? != field.declaration {
        return Err("Generated field observation differs from original template".into());
    }
    if field.ty.nominal_parts().is_none() {
        return Err("Generated tuple field requires an ordinary concrete nominal payload".into());
    }
    Ok(())
}

pub(super) fn template_field(template: &crate::SourceText) -> Result<crate::SourceText, String> {
    use syn::{parse::Parser, spanned::Spanned};
    let parser = |input: syn::parse::ParseStream<'_>| {
        let _: syn::Visibility = input.parse()?;
        let _: syn::Token![struct] = input.parse()?;
        let _: syn::Token![$] = input.parse()?;
        let _: syn::Ident = input.parse()?;
        let fields: syn::FieldsUnnamed = input.parse()?;
        let _: syn::Token![;] = input.parse()?;
        if !input.is_empty() || fields.unnamed.len() != 1 {
            return Err(input.error("Expected complete unary generated tuple template"));
        }
        let field = fields.unnamed.first().unwrap();
        if !field.attrs.is_empty() || !matches!(field.vis, syn::Visibility::Inherited) {
            return Err(input.error("Generated tuple field attributes or visibility unavailable"));
        }
        Ok(field.ty.span())
    };

    let span = parser
        .parse_str(template)
        .map_err(|error| error.to_string())?;
    let relative = crate::source_evidence::original_span(template, span);
    let (snapshot, range) = template.snapshot_range()?;
    crate::SourceText::from_snapshot(
        snapshot.clone(),
        crate::ByteSpan {
            start: range.start + relative.start,
            end: range.start + relative.end,
        },
    )
}

pub(super) fn field_type(source: &str) -> Result<(), String> {
    if source.is_empty()
        || source.len() > crate::concrete_types::MAX_SOURCE_BYTES
        || source.contains('$')
    {
        return Err("Generated tuple field lacks bounded original concrete type bytes".into());
    }
    let path: syn::TypePath = syn::parse_str(source)
        .map_err(|_| "Generated tuple field requires a concrete original type path")?;
    if path.qself.is_some() || crate::source_evidence::plain_relative_path(&path.path).is_none() {
        return Err("Generated tuple field requires an unsubstituted relative type path".into());
    }
    Ok(())
}

pub(crate) fn verify(
    interface: &InterfaceObservations,
    origins: &[GeneratedOrigin],
) -> Result<(), String> {
    let expected = declarations(interface)?;
    if expected.len() != origins.len() {
        return Err("Generated declaration origin inventory disagrees".into());
    }

    let mut seen = std::collections::BTreeSet::new();
    for origin in origins {
        let key = &origin.nominal.definition_path;
        let tree = expected
            .get(key.as_str())
            .ok_or("Unexpected generated declaration origin")?;
        if !seen.insert(key) {
            return Err("Duplicate generated declaration origin".into());
        }
        check(tree, origin)?;
    }
    Ok(())
}

fn check(tree: &TypeTree, origin: &GeneratedOrigin) -> Result<(), String> {
    let (nominal, definition, template, _) = tree
        .generated_parts()
        .ok_or("Missing generated declaration observation")?;
    let raw = &origin.origin.raw;
    if nominal != &origin.nominal
        || definition != &origin.definition
        || raw.path.as_os_str().is_empty()
        || !origin.file_bytes.matches_identity(&raw.digest)
        || crate::source_evidence::byte_range(&origin.file_bytes, raw.span)? != template.as_str()
        || origin.defining_expansion.is_empty()
        || origin.origin.marks
            != [Mark {
                expansion: origin.defining_expansion.clone(),
                transparency: Transparency::SemiOpaque,
            }]
    {
        return Err("Generated declaration template coordinates or hygiene disagree".into());
    }
    Ok(())
}

pub(crate) fn declarations(
    interface: &InterfaceObservations,
) -> Result<BTreeMap<&str, &TypeTree>, String> {
    super::collect_declarations(interface, collect)
}

fn collect<'a>(
    tree: &'a TypeTree,
    expected: &mut BTreeMap<&'a str, &'a TypeTree>,
) -> Result<(), String> {
    if let Some((nominal, ..)) = tree.generated_parts()
        && expected
            .insert(&nominal.definition_path, tree)
            .is_some_and(|previous| previous != tree)
    {
        return Err("Conflicting generated declaration observations".into());
    }
    for child in tree
        .components()
        .chain(tree.nominal_fields().map(|field| &field.ty))
    {
        collect(child, expected)?;
    }
    match tree {
        TypeTree::Array { element, .. }
        | TypeTree::Shared(element)
        | TypeTree::Slice(element)
        | TypeTree::Option(element) => collect(element, expected),
        _ => Ok(()),
    }
}

pub(super) fn share_sources(
    origins: &mut [GeneratedOrigin],
    census: &crate::nominal_boundary::census::Census,
) -> Result<(), String> {
    for origin in origins {
        let raw = &origin.origin.raw;
        let file = census
            .files
            .iter()
            .find(|file| {
                file.path == raw.path
                    && file.digest == raw.digest
                    && file.bytes == origin.file_bytes
            })
            .ok_or("Generated declaration lacks its owned original source snapshot")?;
        origin.file_bytes = file.bytes.clone();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_tuple_field_retains_its_byte_owner_and_exact_spelling() {
        let bytes: crate::SourceSnapshot = "// π\r\npub struct $err( SmallIndexError );\r\n".into();
        let start = bytes.find("pub struct").unwrap() as u32;
        let end = bytes.find(';').unwrap() as u32 + 1;
        let template =
            crate::SourceText::from_snapshot(bytes.clone(), ByteSpan { start, end }).unwrap();
        let field = template_field(&template).unwrap();
        assert_eq!(field.as_str(), "SmallIndexError");
        assert!(crate::SourceSnapshot::ptr_eq(
            field.snapshot_range().unwrap().0,
            &bytes
        ));
        assert_eq!(
            field.snapshot_range().unwrap().1.start as usize,
            bytes.find("SmallIndexError").unwrap()
        );
    }

    #[test]
    fn generated_template_shape_rejects_interpreted_or_partial_declarations() {
        for source in [
            "pub struct Error(SmallIndexError);",
            "pub struct $err(SmallIndexError, Other);",
            "pub struct $err { field: SmallIndexError }",
            "pub struct $err<T>(T);",
            "pub struct $err(pub SmallIndexError);",
            "pub struct $err(#[custom] SmallIndexError);",
            "pub struct $err(SmallIndexError); extra!();",
            "pub struct $err(SmallIndexError)",
        ] {
            assert!(template_field(&source.into()).is_err(), "{source}");
        }
    }
}
