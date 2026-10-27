//! The original caller path must retain its exact function, scope and concrete owner.
use super::*;
use crate::source_evidence::type_scopes::{self, TypeRootUse};

pub(super) fn verify(
    node: &HirNode,
    record: &ProductConstant,
    authority: &SourceAuthority<'_>,
) -> Result<(), String> {
    let facts = authority.facts.facts();
    if occurrences(&facts.body, node)? != 1 {
        return Err("Product constant use lacks one exact retained compiler node".into());
    }
    compiled(facts, record)?;
    let expression = crate::source_evidence::original_expression(node, authority.facts)?;
    let syn::Expr::Path(path) = expression else {
        return Err("Product constant use requires an original path".into());
    };
    let member = record
        .definition
        .path
        .rsplit("::")
        .next()
        .ok_or("Missing associated constant member")?;
    let owner = crate::source_evidence::associated_owner(
        &path,
        member,
        "Product constant use differs from original associated constant path",
    )?;
    let actual = Scoped {
        resolution: authority.resolution,
        namespace: &authority.resolution.namespace,
    }
    .resolve(&owner)?;
    if actual != record.ty {
        return Err("Product constant use selects a different lexical nominal owner".into());
    }

    local_scope(node, &owner, authority)?;
    selected::check_function_namespace(
        authority.facts,
        authority.resolution,
        &authority.resolution.namespace,
    )
}

fn occurrences(tree: &HirNode, expected: &HirNode) -> Result<usize, String> {
    let own = if tree.hir == expected.hir {
        if tree != expected {
            return Err("Product constant use differs from retained original compiler node".into());
        }
        1
    } else {
        0
    };
    tree.children.iter().try_fold(
        own,
        |count, child| Ok(count + occurrences(child, expected)?),
    )
}

fn compiled(facts: &HirFnFacts, record: &ProductConstant) -> Result<(), String> {
    let compiled = facts
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Product constant lacks original compiled crate association")?;
    if compiled["identity"] != "rstyle.rust.compiled-crate/1"
        || compiled["crate_name"].as_str() != Some(record.definition.crate_name.as_str())
        || !compiled["stable_crate_id"]
            .as_str()
            .is_some_and(|identity| !identity.is_empty())
        || record.definition.crate_hash != "local-unencoded"
    {
        return Err("Product constant differs from original compiled crate association".into());
    }
    Ok(())
}

fn local_scope(
    node: &HirNode,
    owner: &syn::TypePath,
    authority: &SourceAuthority<'_>,
) -> Result<(), String> {
    let facts = authority.facts.facts();
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Product constant use lacks original source file")?;
    let name = owner
        .path
        .segments
        .first()
        .ok_or("Missing product constant path root")?
        .ident
        .to_string();

    let target = ByteSpan {
        start: facts
            .fn_span
            .start
            .checked_add(node.span.start)
            .ok_or("Product constant span overflow")?,
        end: facts
            .fn_span
            .start
            .checked_add(node.span.end)
            .ok_or("Product constant span overflow")?,
    };
    function_scope(&UseTarget {
        facts,
        environment: &authority.resolution.environment,
        namespace: &authority.resolution.namespace,
        file,
        name: &name,
        span: target,
    })
}

struct UseTarget<'a> {
    facts: &'a HirFnFacts,
    environment: &'a Environment,
    namespace: &'a Namespace,
    file: &'a str,
    name: &'a str,
    span: ByteSpan,
}

fn function_scope(target: &UseTarget<'_>) -> Result<(), String> {
    let mut matched = 0;
    for (position, module) in target.environment.modules.iter().enumerate() {
        if module.path != std::path::Path::new(target.file) || &module.namespace != target.namespace
        {
            continue;
        }
        let bytes = target.environment.source(&module.path)?.bytes();
        for (item, method) in target
            .environment
            .owner_candidates(position, target.facts.fn_span)
        {
            let (signature, block) = function(item, method)?;
            if crate::source_evidence::byte_range(bytes, target.facts.fn_span)?
                != target.facts.source.bytes.as_str()
            {
                return Err(
                    "Product constant use differs from original complete function source".into(),
                );
            }
            scope(
                signature,
                block,
                LocalScope::new(bytes, target.name, target.span),
            )?;
            matched += 1;
        }
    }
    if matched != 1 {
        return Err("Product constant use lacks unique original function scope".into());
    }
    Ok(())
}

fn function<'a>(
    item: &'a syn::Item,
    method: Option<&'a syn::ImplItemFn>,
) -> Result<(&'a syn::Signature, &'a syn::Block), String> {
    match (item, method) {
        (syn::Item::Fn(item), None) => Ok((&item.sig, &item.block)),
        (syn::Item::Impl(_), Some(method)) => Ok((&method.sig, &method.block)),
        _ => Err("Product constant use is outside an original callable body".into()),
    }
}

fn scope(
    signature: &syn::Signature,
    block: &syn::Block,
    local: LocalScope<'_>,
) -> Result<(), String> {
    type_scopes::verify(
        signature,
        block,
        local.site,
        |statement, name| match statement {
            syn::Stmt::Item(item) => bindings::shadows(item, name),
            syn::Stmt::Macro(_) => true,
            _ => false,
        },
    )
}

struct LocalScope<'a> {
    site: TypeRootUse<'a>,
}

impl<'a> LocalScope<'a> {
    fn new(bytes: &'a str, name: &'a str, target: ByteSpan) -> Self {
        Self {
            site: TypeRootUse {
                bytes,
                name,
                target,
                parameter_error: "Product constant owner path is shadowed by a type parameter",
                local_error: "Product constant owner path has unsupported local source shadowing",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_local_type_aliases_and_globs_do_not_supply_nominal_authority() {
        for prefix in [
            "type SmallIndex = Other;",
            "use other::SmallIndex;",
            "use other::*;",
            "unknown!();",
        ] {
            let bytes = format!("fn caller() {{ {prefix} SmallIndex::MAX }}");
            let item: syn::ItemFn = syn::parse_str(&bytes).unwrap();
            let start = bytes.rfind("SmallIndex::MAX").unwrap() as u32;
            let target = ByteSpan {
                start,
                end: start + 15,
            };
            assert!(
                scope(
                    &item.sig,
                    &item.block,
                    LocalScope::new(&bytes, "SmallIndex", target)
                )
                .is_err()
            );
        }
    }
    #[test]
    fn associated_paths_keep_owner_and_member_syntax() {
        let path: syn::ExprPath = syn::parse_str("SmallIndex::MAX").unwrap();
        assert!(
            crate::source_evidence::associated_owner(&path, "MAX", "invalid path")
                .unwrap()
                .path
                .is_ident("SmallIndex")
        );
        for original in [
            "MAX",
            "SmallIndex::OTHER",
            "SmallIndex::<u32>::MAX",
            "::SmallIndex::MAX",
        ] {
            let path = syn::parse_str(original).unwrap();
            assert!(
                crate::source_evidence::associated_owner(&path, "MAX", "invalid path").is_err(),
                "{original}"
            );
        }
    }

    #[test]
    fn unrelated_nested_alias_does_not_shadow_the_actual_use_scope() {
        let bytes = "fn caller() { { type SmallIndex = Other; } SmallIndex::MAX }";
        let item: syn::ItemFn = syn::parse_str(bytes).unwrap();
        let start = bytes.rfind("SmallIndex::MAX").unwrap() as u32;
        let target = ByteSpan {
            start,
            end: start + 15,
        };
        assert!(
            scope(
                &item.sig,
                &item.block,
                LocalScope::new(bytes, "SmallIndex", target)
            )
            .is_ok()
        );
    }
}
