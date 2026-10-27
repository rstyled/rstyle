use super::*;
use syn::{spanned::Spanned, visit::Visit};

#[derive(Default)]
struct Calls<'ast>(Vec<&'ast syn::ExprMethodCall>);

impl<'ast> Visit<'ast> for Calls<'ast> {
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        if call.method == "as_ref" {
            self.0.push(call);
        }
        syn::visit::visit_expr_method_call(self, call);
    }
}

pub(super) fn site(canonical: &HirFnFacts) -> Result<(&HirNode, &HirNode), String> {
    // This leaf proves one original call, not support for the whole body. Raw
    // generic owners retain deferred call diagnostics until projection is proved.
    if canonical.name != "util::search::Input::<'h>::new" {
        return Err("AsRef requires its original constructor source".into());
    }
    let syntax: syn::ItemFn = syn::parse_str(&canonical.source.bytes).map_err(|e| e.to_string())?;
    let mut calls = Calls::default();
    calls.visit_item_fn(&syntax);

    let [source] = calls.0.as_slice() else {
        return Err("Input source requires exactly one AsRef invocation".into());
    };
    let span = crate::source_evidence::original_span(&canonical.source.bytes, source.span());
    let mut methods = Vec::new();
    crate::frontend::collect_kind(&canonical.body, "expr_method_call", &mut methods);

    let matching: Vec<_> = methods
        .into_iter()
        .filter(|node| node.span == span)
        .collect();
    let [call] = matching.as_slice() else {
        return Err("AsRef original HIR call is absent or duplicated".into());
    };

    let [receiver] = call.children.as_slice() else {
        return Err("AsRef original call requires one receiver".into());
    };
    invocation(canonical, source, call, receiver)?;
    Ok((call, receiver))
}

fn invocation(
    canonical: &HirFnFacts,
    source: &syn::ExprMethodCall,
    call: &HirNode,
    receiver: &HirNode,
) -> Result<(), String> {
    let [parameter] = canonical.signature.params.as_slice() else {
        return Err("Input constructor requires one original haystack formal".into());
    };
    let syn::Expr::Path(path) = &*source.receiver else {
        return Err("AsRef source receiver is not its original formal".into());
    };
    let resolution = receiver
        .res
        .as_ref()
        .ok_or("AsRef receiver lacks original local resolution")?;
    if !source.args.is_empty()
        || source.turbofish.is_some()
        || path.qself.is_some()
        || path.path.get_ident().map(ToString::to_string).as_deref() != Some("haystack")
        || call.expansion.is_some()
        || receiver.expansion.is_some()
        || receiver.kind != "expr_path"
        || resolution.kind != "local"
        || resolution.target.as_ref() != Some(&parameter.pat.hir)
        || !resolved(call)
    {
        return Err("AsRef source/receiver/dispatch correspondence disagrees".into());
    }

    let spelling = canonical
        .source
        .bytes
        .get(call.span.start as usize..call.span.end as usize)
        .ok_or("Missing original AsRef bytes")?;
    if call.snippet != spelling {
        return Err("AsRef original snippet disagrees with source bytes".into());
    }
    Ok(())
}

fn resolved(call: &HirNode) -> bool {
    call.dispatch.as_ref().is_some_and(|dispatch| {
        dispatch.kind == "resolved_method" && dispatch.op == "core::convert::AsRef::as_ref"
    }) && call
        .res
        .as_ref()
        .is_some_and(|res| res.kind == "resolved_callee" && res.target.is_none())
}
