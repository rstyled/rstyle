//! Bind widths to the exact retained expression or original array annotation.
use super::*;
use crate::{id_reader_constants::source, source_evidence::type_scopes};

pub(super) fn expression(
    authority: &SourceAuthority<'_>,
    node: &HirNode,
    record: &Observation,
) -> Result<(), String> {
    retained(authority, node)?;
    let expression = crate::source_evidence::original_expression(node, authority.input)?;
    lexical(authority, node, &expression, record)
}

pub(super) fn annotation(
    authority: &SourceAuthority<'_>,
    node: &HirNode,
    record: &Annotation,
) -> Result<(), String> {
    retained(authority, node)?;
    let bytes = crate::source_evidence::byte_range(&authority.input.source.bytes, node.span)?;
    let array: syn::TypeArray = syn::parse_str(bytes).map_err(|error| error.to_string())?;
    let length = crate::source_evidence::original_span(bytes, array.len.span());
    let length = ByteSpan {
        start: node
            .span
            .start
            .checked_add(length.start)
            .ok_or("ID annotation span overflow")?,
        end: node
            .span
            .start
            .checked_add(length.end)
            .ok_or("ID annotation span overflow")?,
    };

    let expected = crate::data_types::TypeToken::Array {
        element: Box::new(crate::data_types::TypeToken::Scalar("u8".into())),
        length: 4,
    }
    .text()?;
    if record.length != length
        || !matches!(array.elem.as_ref(), syn::Type::Path(path) if path.qself.is_none() && path.path.is_ident("u8"))
        || node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some(expected.as_str())
    {
        return Err(
            "ID reader annotation lost its actual u8 element, length use or concrete type".into(),
        );
    }
    lexical(authority, node, &array.len, &record.constant)
}

fn retained(authority: &SourceAuthority<'_>, node: &HirNode) -> Result<(), String> {
    let candidates =
        crate::field_locations::operations(&authority.input.body, &[node.kind.as_str()]);
    let selected = one(
        candidates
            .into_iter()
            .filter(|candidate| candidate.hir == node.hir),
        "retained ID reader width node",
    )?;
    if selected != node {
        return Err("ID byte width differs from retained compiler node".into());
    }
    Ok(())
}

fn lexical(
    authority: &SourceAuthority<'_>,
    node: &HirNode,
    expression: &syn::Expr,
    record: &Observation,
) -> Result<(), String> {
    let owner = source::owner_path(expression)?;
    let scoped = Scoped {
        resolution: authority.resolution,
        namespace: &authority.resolution.namespace,
    };
    original::nominal(scoped.resolve(&owner)?, &record.owner)?;
    original::declaration(authority.resolution, &record.owner, &record.owner_source)?;

    scope(authority, node, &owner)?;
    selected::check_function_namespace(
        authority.input,
        authority.resolution,
        &authority.resolution.namespace,
    )
}

fn scope(
    authority: &SourceAuthority<'_>,
    node: &HirNode,
    owner: &syn::TypePath,
) -> Result<(), String> {
    let facts = authority.input.facts();
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("ID byte width lacks original source file")?;
    let name = owner
        .path
        .segments
        .first()
        .ok_or("ID width lost its path root")?
        .ident
        .to_string();
    let span = ByteSpan {
        start: facts
            .fn_span
            .start
            .checked_add(node.span.start)
            .ok_or("ID use span overflow")?,
        end: facts
            .fn_span
            .start
            .checked_add(node.span.end)
            .ok_or("ID use span overflow")?,
    };

    caller_scopes(authority, file, &LocalScope { name, span })
}

struct LocalScope {
    name: String,
    span: ByteSpan,
}

fn caller_scopes(
    authority: &SourceAuthority<'_>,
    file: &str,
    scope: &LocalScope,
) -> Result<(), String> {
    let facts = authority.input.facts();
    let mut matched = 0;
    for (position, module) in authority.resolution.environment.modules.iter().enumerate() {
        if module.path != std::path::Path::new(file)
            || module.namespace != authority.resolution.namespace
        {
            continue;
        }
        let bytes = authority
            .resolution
            .environment
            .source(&module.path)?
            .bytes();
        if crate::source_evidence::byte_range(bytes, facts.fn_span)? != facts.source.bytes.as_str()
        {
            return Err("ID width caller differs from complete original function source".into());
        }
        for (item, method) in authority
            .resolution
            .environment
            .owner_candidates(position, facts.fn_span)
        {
            scope.verify(bytes, item, method)?;
            matched += 1;
        }
    }

    if matched != 1 {
        return Err("ID width lacks one complete original caller scope".into());
    }
    Ok(())
}

impl LocalScope {
    fn verify(
        &self,
        bytes: &str,
        item: &syn::Item,
        method: Option<&syn::ImplItemFn>,
    ) -> Result<(), String> {
        let (signature, body) = match (item, method) {
            (syn::Item::Fn(item), None) => (&item.sig, item.block.as_ref()),
            (syn::Item::Impl(_), Some(method)) => (&method.sig, &method.block),
            _ => return Err("ID width is outside an exact original callable".into()),
        };
        type_scopes::verify(
            signature,
            body,
            type_scopes::TypeRootUse {
                bytes,
                name: &self.name,
                target: self.span,
                parameter_error: "ID width owner is shadowed by a type parameter",
                local_error: "ID width owner has unsupported enclosing local bindings",
            },
            |statement, name| match statement {
                syn::Stmt::Item(syn::Item::Use(_)) | syn::Stmt::Macro(_) => true,
                syn::Stmt::Item(item) => {
                    builtin_options::type_name(item).as_deref() == Some(name)
                        || matches!(item, syn::Item::Macro(_))
                }
                _ => false,
            },
        )
    }
}
