//! A complete two-statement constructor grammar, paired against every HIR child.
mod fields;
mod nodes;
mod signature;

use super::{CheckedSource, Result, Role, Site, site};
use crate::{HirFnFacts, HirNode, HirRef};
use std::collections::BTreeMap;
use syn::spanned::Spanned;

struct Flow<'a> {
    facts: &'a HirFnFacts,
    types: BTreeMap<Site, Role>,
    formal: &'a HirRef,
    saved: Option<&'a HirRef>,
    nominal_paths: Vec<(Role, syn::TypePath)>,
}

pub(super) fn verify(facts: &HirFnFacts) -> Result<CheckedSource<'_>> {
    if facts.source.bytes.len() > crate::concrete_types::MAX_SOURCE_BYTES {
        return Err("Constructor source exceeds the bounded proof budget".into());
    }
    let syntax: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;
    let [parameter] = facts.signature.params.as_slice() else {
        return Err("Input constructor requires its complete one-formal roster".into());
    };
    let mut flow = Flow {
        facts,
        types: BTreeMap::new(),
        formal: &parameter.pat.hir,
        saved: None,
        nominal_paths: Vec::new(),
    };
    signature::verify(&syntax, &mut flow)?;

    let length = flow.body(&syntax.block, &facts.body)?;
    flow.bindings()?;
    Ok(CheckedSource {
        types: flow.types,
        length,
        nominal_paths: flow.nominal_paths,
    })
}

impl<'a> Flow<'a> {
    fn node(
        &mut self,
        node: &'a HirNode,
        syntax: &impl Spanned,
        expected: nodes::Shape,
    ) -> Result<()> {
        let span = crate::source_evidence::original_span(&self.facts.source.bytes, syntax.span());
        nodes::verify(node, expected)?;
        if node.hir.owner != self.formal.owner {
            return Err("Constructor HIR value belongs to a different original owner".into());
        }

        if node.span != span
            || self
                .facts
                .source
                .bytes
                .get(span.start as usize..span.end as usize)
                != Some(node.snippet.as_str())
        {
            return Err("Constructor source/HIR span or exact original bytes disagree".into());
        }
        if let Some(role) = expected.role
            && self.types.insert(site(&node.hir), role).is_some()
        {
            return Err("Constructor HIR duplicates a complete value-node identity".into());
        }
        Ok(())
    }

    fn body(&mut self, syntax: &syn::Block, body: &'a HirNode) -> Result<&'a HirNode> {
        self.node(
            body,
            syntax,
            nodes::Shape::value("expr_block", 2, Role::Input),
        )?;
        let [
            syn::Stmt::Local(local),
            syn::Stmt::Expr(syn::Expr::Struct(result), None),
        ] = syntax.stmts.as_slice()
        else {
            return Err(
                "Constructor must bind one conversion then return its complete Input value".into(),
            );
        };
        self.local(local, &body.children[0])?;
        self.product(result, &body.children[1])
    }

    fn local(&mut self, syntax: &syn::Local, node: &'a HirNode) -> Result<()> {
        self.node(node, syntax, nodes::Shape::statement())?;
        if !syntax.attrs.is_empty() {
            return Err("Constructor local attributes require additional source proof".into());
        }
        let syn::Pat::Ident(pattern) = &syntax.pat else {
            return Err("Constructor saved view must have one original identifier binding".into());
        };
        let initial = syntax
            .init
            .as_ref()
            .ok_or("Constructor saved view lacks initialization")?;
        if initial.diverge.is_some()
            || pattern.ident != "haystack"
            || pattern.by_ref.is_some()
            || pattern.mutability.is_some()
            || pattern.subpat.is_some()
        {
            return Err("Constructor saved view changes binding/initialization semantics".into());
        }

        self.saved_binding(pattern, &node.children[0])?;
        self.conversion(&initial.expr, &node.children[1])
    }

    fn saved_binding(&mut self, pattern: &syn::PatIdent, binding: &'a HirNode) -> Result<()> {
        self.node(binding, pattern, nodes::Shape::binding(Role::Bytes, None))?;
        self.saved = Some(&binding.hir);
        Ok(())
    }

    fn conversion(&mut self, syntax: &syn::Expr, node: &'a HirNode) -> Result<()> {
        let syn::Expr::MethodCall(call) = syntax else {
            return Err("Constructor saved value must be its single AsRef call".into());
        };
        self.node(node, call, nodes::Shape::method(Role::Bytes))?;
        if call.method != "as_ref"
            || !call.args.is_empty()
            || call.turbofish.is_some()
            || !call.attrs.is_empty()
        {
            return Err(
                "Constructor conversion source changed its original AsRef invocation".into(),
            );
        }
        self.path(&call.receiver, &node.children[0], Role::Formal)
    }

    fn path(&mut self, syntax: &syn::Expr, node: &'a HirNode, role: Role) -> Result<()> {
        let binding = match role {
            Role::Formal => self.formal,
            Role::Bytes => self
                .saved
                .ok_or("Constructor saved view is uninitialized")?,
            _ => return Err("Constructor local read has no supported binding role".into()),
        };

        let syn::Expr::Path(path) = syntax else {
            return Err("Constructor value is not a saved local read".into());
        };
        self.node(node, path, nodes::Shape::path(role))?;
        if !path.attrs.is_empty()
            || path.qself.is_some()
            || !path.path.is_ident("haystack")
            || !node.res.as_ref().is_some_and(|resolution| {
                resolution.kind == "local" && resolution.target.as_ref() == Some(binding)
            })
        {
            return Err("Constructor local read loses its exact saved binding origin".into());
        }
        Ok(())
    }

    fn bindings(&self) -> Result<()> {
        let saved = self
            .saved
            .ok_or("Constructor complete source lacks its saved view")?;
        let [formal, local] = self.facts.bindings.as_slice() else {
            return Err("Constructor binding inventory omits or invents a declaration".into());
        };

        let body = &self.facts.body;
        nodes::binding(
            formal,
            &self.facts.signature.params[0].pat,
            true,
            format!("scope:source:fn:0:{}", self.facts.source.bytes.len()),
        )?;
        let local_node = &body.children[0].children[0];
        nodes::binding(
            local,
            local_node,
            false,
            format!("scope:source:block:{}:{}", body.span.start, body.span.end),
        )?;
        if &local.hir != saved || local.hir == formal.hir {
            return Err("Constructor saved/formal bindings are conflated".into());
        }
        Ok(())
    }
}
