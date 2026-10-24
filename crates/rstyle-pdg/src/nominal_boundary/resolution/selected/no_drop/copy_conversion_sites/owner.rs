use super::*;
use crate::concrete_types::source_proof::PathResolution;
use crate::nominal_boundary::resolution::{FunctionSource, Scoped, implementation_source_span};
use crate::source_evidence::trait_members::{self, AttributePolicy};
use syn::spanned::Spanned;

pub(super) struct Site {
    pub implementation: FileSpan,
    pub method: FileSpan,
    pub associated_error: Option<FileSpan>,
}

const ATTRIBUTES: AttributePolicy = AttributePolicy {
    inactive: "Copy conversion original member is inactive",
    member: |member| match member {
        syn::ImplItem::Fn(item) => &item.attrs,
        syn::ImplItem::Type(item) => &item.attrs,
        _ => &[],
    },
};

pub(super) fn find(
    facts: &HirFnFacts,
    resolution: &Resolution,
    endpoints: &Endpoints,
) -> Result<Site, String> {
    resolution.environment.check_input_owner(facts)?;
    let file = facts
        .build
        .frontend_options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Missing original conversion source file")?;
    let context = Owner {
        facts,
        resolution,
        endpoints,
    };
    let mut sites = Vec::new();

    for module in resolution
        .environment
        .modules
        .iter()
        .filter(|module| module.path.to_string_lossy() == file)
    {
        sites.extend(context.module(module)?);
    }

    if sites.len() != 1 {
        return Err("Missing or ambiguous exact original conversion owner".into());
    }
    sites
        .pop()
        .ok_or_else(|| "Missing original conversion owner".into())
}

struct Owner<'a> {
    facts: &'a HirFnFacts,
    resolution: &'a Resolution,
    endpoints: &'a Endpoints,
}

impl Owner<'_> {
    fn module(&self, module: &census::Module) -> Result<Vec<Site>, String> {
        let bytes = self.resolution.environment.source(&module.path)?.bytes();
        let mut sites = Vec::new();

        for item in &module.items {
            let syn::Item::Impl(item) = item else {
                continue;
            };
            for member in &item.items {
                let syn::ImplItem::Fn(method) = member else {
                    continue;
                };
                if (FunctionSource {
                    vis: &method.vis,
                    sig: &method.sig,
                    body: &method.block,
                })
                .exact(bytes, self.facts)
                {
                    sites.push(self.site(item, method, module)?);
                }
            }
        }
        Ok(sites)
    }

    fn site(
        &self,
        item: &syn::ItemImpl,
        method: &syn::ImplItemFn,
        module: &census::Module,
    ) -> Result<Site, String> {
        trait_members::attributes(
            self.facts,
            item,
            ATTRIBUTES,
            super::super::super::function_attributes,
        )?;
        let scope = Scoped {
            resolution: self.resolution,
            namespace: &module.namespace,
        };
        header(item, method, &scope, self.endpoints)?;
        super::super::verify_ancestor_attributes(self.facts, self.resolution, &module.namespace)?;
        let associated_error = error_member(item, &scope, self.endpoints)?;
        let file = self
            .resolution
            .environment
            .census()
            .files
            .iter()
            .find(|file| file.path == module.path)
            .ok_or("Missing original conversion source snapshot")?;
        let bytes = file.bytes.as_ref();
        let anchor = |span| FileSpan {
            path: file.path.clone(),
            digest: file.digest.clone(),
            span,
        };

        Ok(Site {
            implementation: anchor(implementation_source_span(bytes, item)),
            method: anchor(self.facts.fn_span),
            associated_error: associated_error
                .map(|item| anchor(crate::source_evidence::original_span(bytes, item.span()))),
        })
    }
}

fn header(
    item: &syn::ItemImpl,
    method: &syn::ImplItemFn,
    scope: &Scoped<'_>,
    endpoints: &Endpoints,
) -> Result<(), String> {
    if item.unsafety.is_some()
        || item.defaultness.is_some()
        || !item.generics.params.is_empty()
        || item.generics.where_clause.is_some()
    {
        return Err("Copy conversion requires its concrete safe original impl".into());
    }
    let input = trait_input(item, endpoints.direction)?;
    let expected_self = match endpoints.direction {
        Direction::Wrap => endpoints.tagged(),
        Direction::Unwrap => endpoints.product(),
    };
    if resolve(scope, input)? != *endpoints.input()
        || resolve(scope, &item.self_ty)? != *expected_self
    {
        return Err("Copy conversion source impl endpoint identity disagrees".into());
    }

    signature(method, scope, endpoints)
}

fn trait_input(item: &syn::ItemImpl, direction: Direction) -> Result<&syn::Type, String> {
    let Some((None, path, _)) = &item.trait_ else {
        return Err("Copy conversion requires its original positive trait impl".into());
    };

    let expected = match direction {
        Direction::Wrap => "From",
        Direction::Unwrap => "TryFrom",
    };
    if path.leading_colon.is_some()
        || path.segments.len() != 1
        || path.segments[0].ident != expected
    {
        return Err("Copy conversion requires its original direct trait spelling".into());
    }

    let syn::PathArguments::AngleBracketed(arguments) = &path.segments[0].arguments else {
        return Err("Copy conversion lacks its original input substitution".into());
    };
    let Some(syn::GenericArgument::Type(input)) = arguments.args.first() else {
        return Err("Copy conversion requires a concrete original input type".into());
    };
    if arguments.colon2_token.is_some() || arguments.args.len() != 1 {
        return Err("Copy conversion requires one original trait input".into());
    }
    Ok(input)
}

fn signature(
    method: &syn::ImplItemFn,
    scope: &Scoped<'_>,
    endpoints: &Endpoints,
) -> Result<(), String> {
    let name = match endpoints.direction {
        Direction::Wrap => "from",
        Direction::Unwrap => "try_from",
    };
    let Some(syn::FnArg::Typed(input)) = method.sig.inputs.first() else {
        return Err("Copy conversion lacks its original input annotation".into());
    };
    let syn::ReturnType::Type(_, output) = &method.sig.output else {
        return Err("Copy conversion lacks its original return annotation".into());
    };

    if method.defaultness.is_some()
        || method.sig.ident != name
        || resolve(scope, &input.ty)? != *endpoints.input()
    {
        return Err("Copy conversion member name or original input differs".into());
    }
    match endpoints.direction {
        Direction::Wrap if crate::source_evidence::plain_type_path(output, "Self") => Ok(()),
        Direction::Unwrap => result_type(output),
        _ => Err("Copy conversion wrapping member must return Self".into()),
    }
}

fn error_member<'a>(
    item: &'a syn::ItemImpl,
    scope: &Scoped<'_>,
    endpoints: &Endpoints,
) -> Result<Option<&'a syn::ImplItemType>, String> {
    match (endpoints.direction, item.items.as_slice()) {
        (Direction::Wrap, [syn::ImplItem::Fn(_)]) => Ok(None),
        (Direction::Unwrap, [syn::ImplItem::Type(error), syn::ImplItem::Fn(_)]) => {
            if error.defaultness.is_some()
                || error.ident != "Error"
                || !error.generics.params.is_empty()
                || error.generics.where_clause.is_some()
                || endpoints.error() != Some(&resolve(scope, &error.ty)?)
            {
                return Err("Copy conversion associated Error identity or source differs".into());
            }
            Ok(Some(error))
        }
        _ => Err("Copy conversion requires its complete original member roster".into()),
    }
}

fn resolve(scope: &Scoped<'_>, ty: &syn::Type) -> Result<TypeTree, String> {
    let syn::Type::Path(path) = ty else {
        return Err("Copy conversion requires an original concrete nominal path".into());
    };
    scope.resolve(path)
}

fn result_type(ty: &syn::Type) -> Result<(), String> {
    let syn::Type::Path(path) = ty else {
        return Err("Copy conversion requires original Result source".into());
    };
    let Some(segment) = path.path.segments.first() else {
        return Err("Missing Result path".into());
    };
    let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return Err("Missing original Result payloads".into());
    };

    if arguments.colon2_token.is_some() {
        return Err("Copy conversion requires its original Result annotation".into());
    }
    let arguments: Vec<_> = arguments.args.iter().collect();
    let valid = match arguments.as_slice() {
        [
            syn::GenericArgument::Type(ok),
            syn::GenericArgument::Type(syn::Type::Path(error)),
        ] => {
            crate::source_evidence::plain_type_path(ok, "Self")
                && error.qself.is_none()
                && crate::source_evidence::plain_relative_path(&error.path)
                    .is_some_and(|names| names == ["Self", "Error"])
        }
        _ => false,
    };

    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || path.path.segments.len() != 1
        || segment.ident != "Result"
        || !valid
    {
        return Err("Copy conversion Result must retain exact Self and associated Error".into());
    }
    Ok(())
}
