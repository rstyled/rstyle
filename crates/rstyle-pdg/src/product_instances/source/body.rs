//! Exactly one conversion, the complete original assertion, and one whole product.
use super::*;

pub(super) fn verify<'a>(facts: &'a HirFnFacts, syntax: &syn::ItemFn) -> Result<CheckedSource<'a>> {
    let [
        syn::Stmt::Local(local),
        assertion_source,
        syn::Stmt::Expr(syn::Expr::Struct(product), None),
    ] = syntax.block.stmts.as_slice()
    else {
        return Err("Product body must convert, assert, then return its whole result".into());
    };

    node(
        facts,
        &facts.body,
        &syntax.block,
        Shape {
            kind: "expr_block",
            children: 3,
            typed: true,
        },
    )?;
    plain(&facts.body)?;
    let let_node = &facts.body.children[0];
    let saved = local_source(facts, local, let_node)?;
    let conversion = &let_node.children[2];
    let assertion = assertion(facts, assertion_source, &facts.body.children[1], saved)?;
    let result = &facts.body.children[2];
    product_source(facts, product, result, saved)?;
    let checked = CheckedSource {
        facts,
        pattern: &facts.signature.params[0].pat,
        input: &facts.signature.params[1].pat,
        local: saved,
        conversion,
        assertion,
        result,
    };
    bindings(&checked)?;
    Ok(checked)
}

fn local_source<'a>(
    facts: &HirFnFacts,
    local: &syn::Local,
    let_node: &'a HirNode,
) -> Result<&'a HirNode> {
    node(
        facts,
        let_node,
        local,
        Shape {
            kind: "stmt_let",
            children: 3,
            typed: false,
        },
    )?;

    plain(let_node)?;
    let syn::Pat::Type(typed) = &local.pat else {
        return Err("Product local type annotation absent".into());
    };
    let syn::Pat::Ident(binding) = &*typed.pat else {
        return Err("Product local binding absent".into());
    };
    signature::binding_source(binding, "span")?;
    if !local.attrs.is_empty() || !typed.attrs.is_empty() {
        return Err("Product local attributes unsupported".into());
    }
    let saved = &let_node.children[0];
    signature::binding_node(facts, saved, binding, None)?;
    let annotation = &let_node.children[1];
    node(
        facts,
        annotation,
        &typed.ty,
        Shape {
            kind: "ty_annotation",
            children: 0,
            typed: true,
        },
    )?;
    plain(annotation)?;
    let init = local
        .init
        .as_ref()
        .ok_or("Product local initialization absent")?;
    if init.diverge.is_some() {
        return Err("Product let-else unsupported".into());
    }
    conversion_source(facts, &init.expr, &let_node.children[2])?;
    Ok(saved)
}

fn conversion_source(facts: &HirFnFacts, syntax: &syn::Expr, call: &HirNode) -> Result<()> {
    let syn::Expr::MethodCall(method) = syntax else {
        return Err("Product conversion is not one method call".into());
    };

    node(
        facts,
        call,
        method,
        Shape {
            kind: "expr_method_call",
            children: 1,
            typed: true,
        },
    )?;
    if !method.attrs.is_empty()
        || !method.args.is_empty()
        || method.turbofish.is_some()
        || method.method != "into"
        || call.binding.is_some()
        || call.binding_mut.is_some()
        || call.param_index.is_some()
        || call.lit.is_some()
        || call.expansion.is_some()
        || call
            .res
            .as_ref()
            .is_none_or(|res| res.kind != "resolved_callee" || res.target.is_some())
        || call
            .dispatch
            .as_ref()
            .is_none_or(|dispatch| dispatch.kind != "resolved_method")
    {
        return Err("Product conversion changes its original invocation".into());
    }
    path(
        facts,
        &method.receiver,
        &call.children[0],
        &facts.signature.params[1].pat,
    )
}

fn assertion<'a>(
    facts: &HirFnFacts,
    syntax: &syn::Stmt,
    wrapper: &'a HirNode,
    saved: &HirNode,
) -> Result<&'a HirNode> {
    let mac = match syntax {
        syn::Stmt::Macro(mac) if mac.semi_token.is_some() && mac.attrs.is_empty() => &mac.mac,
        _ => return Err("Product assertion must preserve its original macro statement".into()),
    };

    node(
        facts,
        wrapper,
        mac,
        Shape {
            kind: "stmt_semi",
            children: 1,
            typed: false,
        },
    )?;
    plain(wrapper)?;
    let assertion = &wrapper.children[0];
    node(
        facts,
        assertion,
        mac,
        Shape {
            kind: "expr_assert",
            children: 1,
            typed: true,
        },
    )?;
    let parsed = assertion_authority(facts, assertion, mac)?;
    comparison(facts, &parsed.condition, &assertion.children[0], saved)?;
    Ok(assertion)
}

fn assertion_authority(
    facts: &HirFnFacts,
    assertion: &HirNode,
    mac: &syn::Macro,
) -> Result<crate::assertions::Assertion> {
    crate::assertions::verify(
        assertion,
        &assertion.snippet,
        if facts.build.debug_assertions.status == crate::FactStatus::Known {
            facts.build.debug_assertions.value
        } else {
            None
        },
        crate::extraction_profile::ExtractionProfile::DataV3,
    )?;

    let parsed = crate::assertions::Assertion::from_macro(mac)?;
    if parsed.name != "assert"
        || parsed.right.is_some()
        || parsed.diagnostic.as_ref().is_none_or(|diagnostic| {
            diagnostic.message != "invalid match span" || !diagnostic.captures.is_empty()
        })
        || assertion.binding.is_some()
        || assertion.binding_mut.is_some()
        || assertion.param_index.is_some()
        || assertion.dispatch.is_some()
    {
        return Err("Product assertion diagnostic or original operation differs".into());
    }
    Ok(parsed)
}

fn comparison(
    facts: &HirFnFacts,
    syntax: &syn::Expr,
    condition: &HirNode,
    saved: &HirNode,
) -> Result<()> {
    let syn::Expr::Binary(comparison) = syntax else {
        return Err("Product assertion comparison absent".into());
    };

    node(
        facts,
        condition,
        comparison,
        Shape {
            kind: "expr_binary",
            children: 2,
            typed: true,
        },
    )?;
    let mut no_dispatch = condition.clone();
    no_dispatch.dispatch = None;
    plain(&no_dispatch)?;
    if !comparison.attrs.is_empty()
        || !matches!(comparison.op, syn::BinOp::Le(_))
        || condition.dispatch.as_ref().is_none_or(|dispatch| {
            dispatch.kind != "builtin"
                || dispatch.op != "<="
                || dispatch.operand_tys != ["usize", "usize"]
                || dispatch.result_ty != "bool"
        })
    {
        return Err("Product assertion requires the original builtin usize ordering".into());
    }
    let endpoints = Endpoints { facts, saved };
    endpoints.verify(&comparison.left, &condition.children[0], Endpoint::Start)?;
    endpoints.verify(&comparison.right, &condition.children[1], Endpoint::End)
}

enum Endpoint {
    Start,
    End,
}

struct Endpoints<'a> {
    facts: &'a HirFnFacts,
    saved: &'a HirNode,
}

impl Endpoints<'_> {
    fn verify(&self, syntax: &syn::Expr, field: &HirNode, role: Endpoint) -> Result<()> {
        let (name, position) = match role {
            Endpoint::Start => ("start", 0),
            Endpoint::End => ("end", 1),
        };

        let syn::Expr::Field(syntax) = syntax else {
            return Err("Product endpoint must be a saved Span field".into());
        };
        node(
            self.facts,
            field,
            syntax,
            Shape {
                kind: "expr_field",
                children: 1,
                typed: true,
            },
        )?;
        let mut no_res = field.clone();
        no_res.res = None;
        plain(&no_res)?;
        let res = field
            .res
            .as_ref()
            .ok_or("Product field resolution absent")?;
        let observed: Value =
            serde_json::from_str(&res.description).map_err(|error| error.to_string())?;
        if !syntax.attrs.is_empty()
            || syntax.member
                != syn::Member::Named(syn::Ident::new(name, proc_macro2::Span::call_site()))
            || res.kind != "resolved_field"
            || res.target.is_some()
            || observed
                != serde_json::json!({"position":position,"owner_path":"util::search::Span"})
            || crate::frontend::normalized(field) != Some("usize")
        {
            return Err("Product endpoint field resolution differs".into());
        }
        path(self.facts, &syntax.base, &field.children[0], self.saved)
    }
}

fn product_source(
    facts: &HirFnFacts,
    syntax: &syn::ExprStruct,
    result: &HirNode,
    saved: &HirNode,
) -> Result<()> {
    node(
        facts,
        result,
        syntax,
        Shape {
            kind: "expr_struct",
            children: 2,
            typed: true,
        },
    )?;

    plain(result)?;
    if !syntax.attrs.is_empty()
        || syntax.qself.is_some()
        || syntax.rest.is_some()
        || syntax.fields.len() != 2
    {
        return Err("Product result omits or adds source fields".into());
    }
    for (index, (field, name)) in syntax.fields.iter().zip(["pattern", "span"]).enumerate() {
        if !field.attrs.is_empty()
            || field.colon_token.is_some()
            || !matches!(&field.member, syn::Member::Named(ident) if ident == name)
        {
            return Err("Product result changes ordered shorthand fields".into());
        }
        let binding = if index == 0 {
            &facts.signature.params[0].pat
        } else {
            saved
        };
        path(facts, &field.expr, &result.children[index], binding)?;
    }
    Ok(())
}

fn bindings(checked: &CheckedSource<'_>) -> Result<()> {
    let facts = checked.facts();

    if facts.bindings.len() != 3 {
        return Err("Product lexical binding roster differs".into());
    }
    for (index, (binding, node)) in facts
        .bindings
        .iter()
        .zip([checked.pattern(), checked.input(), checked.local()])
        .enumerate()
    {
        let scope = if index < 2 {
            format!("scope:source:fn:0:{}", facts.source.bytes.len())
        } else {
            format!(
                "scope:source:block:{}:{}",
                facts.body.span.start, facts.body.span.end
            )
        };
        if binding.hir != node.hir
            || binding.decl_span != node.span
            || binding.is_param != (index < 2)
            || binding.is_mut
            || binding.scope != scope
            || binding.name != node.snippet
            || node.binding.as_ref() != Some(&binding.symbol)
            || node.ty.as_ref().is_none_or(|ty| {
                ty.normalized != binding.ty.normalized || ty.observed != binding.ty.observed
            })
            || !binding.ty.spelling.is_empty()
        {
            return Err("Product lexical binding inventory changes source identity or type".into());
        }
    }
    Ok(())
}
