//! Exact source contracts for finite Copy enums with scalar tuple payloads.
use crate::concrete_types::source_proof::PathResolution;
use crate::{HirNode, data_types::TypeToken, unit_store::EvidenceFacts};
use syn::spanned::Spanned;

pub(crate) const CAPABILITY: &str = "rust.scalar-enum-values";

pub(crate) struct Match<'a> {
    pub scrutinee: &'a HirNode,
    pub arms: Vec<Arm<'a>>,
}

pub(crate) struct Arm<'a> {
    pub pattern: &'a HirNode,
    pub tag: String,
    pub binding: Option<&'a HirNode>,
}

fn enum_value(node: &HirNode) -> bool {
    node.ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        .is_some_and(|ty| ty.starts_with("{\"kind\":\"enum\","))
}

pub(crate) fn constructor_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    node.kind == "expr_call"
        && enum_value(node)
        && crate::extraction_profile::facts_support(facts, CAPABILITY)
}

pub(crate) fn match_candidate(node: &HirNode, facts: &dyn EvidenceFacts) -> bool {
    child_candidate(node, facts, "expr_match", enum_value)
}

pub(crate) fn child_candidate(
    node: &HirNode,
    facts: &dyn EvidenceFacts,
    kind: &str,
    selected: fn(&HirNode) -> bool,
) -> bool {
    node.kind == kind
        && node.children.first().is_some_and(selected)
        && crate::extraction_profile::facts_support(facts, CAPABILITY)
}

pub(crate) fn constructor(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<String, String> {
    if !constructor_candidate(node, facts) || node.expansion.is_some() {
        return Err("Enum construction requires original direct scalar-variant evidence".into());
    }

    let source = source(node, facts)?;
    let context = SourceContext { node, facts };
    let syntax: syn::Expr = syn::parse_str(source).map_err(|error| error.to_string())?;
    let syn::Expr::Call(call) = crate::data_aggregates::ungroup(&syntax) else {
        return Err("Enum constructor requires original call syntax".into());
    };
    let syn::Expr::Path(path) = crate::data_aggregates::ungroup(&call.func) else {
        return Err("Enum constructor requires a resolved variant path".into());
    };
    if path.qself.is_some() || call.args.len() != 1 || node.children.len() != 1 {
        return Err("Enum constructor requires exactly one scalar evaluation".into());
    }

    let (tag, payload) = variant(&path.path, node, "resolved_callee", &context)?;
    if crate::data_aggregates::token(&node.children[0])? != payload.text()? {
        return Err("Enum constructor payload type disagrees".into());
    }
    verify_constructor_dispatch(node, &payload)?;

    context.correspondence(call.args[0].span(), &node.children[0])?;
    Ok(tag)
}

fn verify_constructor_dispatch(node: &HirNode, payload: &TypeToken) -> Result<(), String> {
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Missing enum constructor dispatch")?;
    let resolution = node
        .res
        .as_ref()
        .ok_or("Missing enum constructor identity")?;
    let identity: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;

    if dispatch.kind != "resolved_direct"
        || dispatch.op != identity.path
        || dispatch.operand_tys != [payload.text()?]
        || dispatch.result_ty != crate::data_aggregates::token(node)?
    {
        return Err("Enum constructor dispatch disagrees with its exact scalar evaluation".into());
    }
    Ok(())
}

pub(crate) fn matched<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Result<Match<'a>, String> {
    if !match_candidate(node, facts) || node.expansion.is_some() {
        return Err("Enum match requires original supported syntax".into());
    }

    let source = source(node, facts)?;
    let context = SourceContext { node, facts };
    let syntax: syn::ExprMatch = syn::parse_str(source).map_err(|error| error.to_string())?;
    let scrutinee = node.children.first().ok_or("Missing enum scrutinee")?;
    let TypeToken::Enum { variants, .. } =
        crate::data_types::parse(crate::data_aggregates::token(scrutinee)?)?
    else {
        return Err("Missing complete scalar enum type".into());
    };
    if node.children.len() != variants.len() + 1 || syntax.arms.len() != variants.len() {
        return Err("Enum match requires exactly one arm per variant".into());
    }

    context.correspondence(syntax.expr.span(), scrutinee)?;
    let mut names = std::collections::BTreeSet::new();
    let mut arms = Vec::new();
    for (syntax, arm) in syntax.arms.iter().zip(&node.children[1..]) {
        let checked = matched_arm(&context, syntax, arm, scrutinee)?;
        if !names.insert(checked.tag.clone()) {
            return Err("Enum match repeats a variant".into());
        }
        arms.push(checked);
    }
    Ok(Match { scrutinee, arms })
}

fn matched_arm<'a>(
    context: &SourceContext<'_>,
    syntax: &syn::Arm,
    arm: &'a HirNode,
    scrutinee: &HirNode,
) -> Result<Arm<'a>, String> {
    let [pattern, body] = arm.children.as_slice() else {
        return Err("Enum arm requires an exact pattern and body without guards".into());
    };
    let syn::Pat::TupleStruct(syntax_pattern) = &syntax.pat else {
        return Err("Enum arm requires a tuple-variant pattern".into());
    };
    if arm.kind != "match_arm"
        || syntax.guard.is_some()
        || !syntax.attrs.is_empty()
        || syntax_pattern.qself.is_some()
        || syntax_pattern.elems.len() != 1
        || pattern.kind != "pat_local_variant"
        || pattern.children.len() != 1
        || pattern.expansion.is_some()
        || crate::data_aggregates::token(pattern)? != crate::data_aggregates::token(scrutinee)?
    {
        return Err("Enum arm loses original scalar payload or variant evidence".into());
    }

    context.correspondence(syntax.pat.span(), pattern)?;

    context.correspondence(syntax.body.span(), body)?;

    let (tag, payload) = variant(
        &syntax_pattern.path,
        scrutinee,
        "resolved_local_variant",
        &SourceContext {
            node: pattern,
            ..*context
        },
    )?;

    let binding = payload_binding(
        &syntax_pattern.elems[0],
        &pattern.children[0],
        &payload,
        context.facts,
    )?;

    context.correspondence(syntax_pattern.elems[0].span(), &pattern.children[0])?;
    Ok(Arm {
        pattern,
        tag,
        binding,
    })
}

fn payload_binding<'a>(
    syntax: &syn::Pat,
    node: &'a HirNode,
    payload: &TypeToken,
    facts: &dyn EvidenceFacts,
) -> Result<Option<&'a HirNode>, String> {
    if crate::data_aggregates::token(node)? != payload.text()? {
        return Err("Enum payload pattern type disagrees".into());
    }
    if matches!(syntax, syn::Pat::Wild(_)) && node.kind == "pat_wild" {
        return Ok(None);
    }
    let syn::Pat::Ident(syntax) = syntax else {
        return Err("Enum payload requires a scalar binding or wildcard".into());
    };

    let binding = facts
        .bindings
        .iter()
        .find(|binding| Some(&binding.symbol) == node.binding.as_ref())
        .ok_or("Missing enum payload binding declaration")?;
    if syntax.by_ref.is_some()
        || syntax.mutability.is_some()
        || syntax.subpat.is_some()
        || node.kind != "pat_binding"
        || node.binding_mut != Some(false)
        || binding.is_mut
        || binding.is_param
        || binding.hir != node.hir
        || syntax.ident != binding.name
        || binding.decl_span != node.span
        || binding.ty.normalized.as_deref() != Some(payload.text()?.as_str())
    {
        return Err("Enum payload binding is not the original immutable scalar declaration".into());
    }
    Ok(Some(node))
}

fn variant(
    path: &syn::Path,
    value: &HirNode,
    resolution_kind: &str,
    context: &SourceContext<'_>,
) -> Result<(String, TypeToken), String> {
    let TypeToken::Enum { nominal, variants } =
        crate::data_types::parse(crate::data_aggregates::token(value)?)?
    else {
        return Err("Expected complete scalar enum value".into());
    };
    if path.segments.len() < 2
        || path
            .segments
            .iter()
            .any(|part| !matches!(part.arguments, syn::PathArguments::None))
    {
        return Err("Enum variant requires an exact enum-qualified path".into());
    }

    let mut owner = path.clone();
    let tag = owner
        .segments
        .pop()
        .ok_or("Missing enum variant")?
        .into_value()
        .ident
        .to_string();
    owner.segments.pop_punct();
    let resolved = context
        .facts
        .nominal_resolution()?
        .resolve(&syn::TypePath {
            qself: None,
            path: owner,
        })?;
    if crate::data_types::TypeToken::from_tree(&resolved)
        != crate::data_types::parse(crate::data_aggregates::token(value)?)?
    {
        return Err("Enum path disagrees with the complete nominal declaration".into());
    }

    verify_variant_identity(context, resolution_kind, &nominal, &tag)?;
    let (_, payload) = variants
        .into_iter()
        .find(|(name, _)| name == &tag)
        .ok_or("Unknown enum variant")?;
    Ok((tag, payload))
}

fn verify_variant_identity(
    context: &SourceContext<'_>,
    resolution_kind: &str,
    nominal: &crate::concrete_types::Nominal,
    tag: &str,
) -> Result<(), String> {
    let resolution = context
        .node
        .res
        .as_ref()
        .ok_or("Missing enum variant resolution")?;
    let identity: crate::data_library::Callee =
        serde_json::from_str(&resolution.description).map_err(|error| error.to_string())?;
    if resolution.kind != resolution_kind
        || resolution.target.is_some()
        || identity.path != format!("{}::{tag}", nominal.definition_path)
        || Some(identity.crate_name.as_str())
            != context
                .facts
                .build
                .frontend_options
                .get("rstyle.nominal_selected_unit")
                .and_then(|unit| unit.get("crate_name"))
                .and_then(serde_json::Value::as_str)
        || identity.crate_hash != "local-unencoded"
        || identity.definition_hash.is_empty()
        || identity.implementation.is_some()
    {
        return Err("Enum variant compiler identity disagrees with its source declaration".into());
    }
    Ok(())
}

fn source<'a>(node: &HirNode, facts: &'a dyn EvidenceFacts) -> Result<&'a str, String> {
    facts
        .source
        .bytes
        .get(node.span.start as usize..node.span.end as usize)
        .ok_or_else(|| "Missing original enum source".into())
}

struct SourceContext<'a> {
    node: &'a HirNode,
    facts: &'a dyn EvidenceFacts,
}

impl SourceContext<'_> {
    fn correspondence(&self, span: proc_macro2::Span, child: &HirNode) -> Result<(), String> {
        crate::source_evidence::verify_child_span(self.node, child, span, self.facts)
    }
}
