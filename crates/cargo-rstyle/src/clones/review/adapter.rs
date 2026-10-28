//! Stable review values for completed helper findings, never fresh proof claims.
use super::*;
use crate::clones::capture;
use rstyle_pdg::{HirFnFacts, HirNode, StringFact, consumer_facts::Availability};
use std::{
    path::{Component, Path},
    sync::Arc,
};
use syn::spanned::Spanned;

pub(crate) struct Policies<'a> {
    pub matching: &'a MatchingPolicy,
    pub consumer: &'a ConsumerPolicy,
}

/// The owning caller supplies endpoints only after a completed helper finding.
/// Cache this conversion per report-local endpoint; do not repeat it per edge.
pub(crate) fn member(
    facts: &HirFnFacts,
    unit: &capture::Unit,
    plan: &capture::Plan,
    policies: Policies<'_>,
) -> Result<MemberSnapshot, String> {
    let local = rstyle_pdg::consumer_facts::inspect(facts)?;
    if !matches!(local.source, Availability::Known(_)) {
        return Err("Review requires available exact original source ownership".into());
    }
    let source = original_free_function(facts)?;
    let package = unit
        .target
        .package_context
        .as_ref()
        .ok_or("Review package context is missing")?;
    if package.package_id != unit.target.package
        || package.manifest_path.parent() != Some(unit.target.manifest_dir.as_path())
        || unit.extraction_profile != plan.extraction_profile
    {
        return Err("Review package or extraction context is contradictory".into());
    }

    let locator = Locator {
        package: Package {
            identity: relative(&package.manifest_path, &plan.root)?,
            name: package.name.clone(),
            version: package.version.clone(),
        },
        source: relative(Path::new(&source), &unit.target.manifest_dir)?,
        qualified_name: facts.name.clone(),
        declaration_kind: "free-function".into(),
        discriminator: "unique-declaration".into(),
    };
    Ok(MemberSnapshot {
        key: MemberKey {
            locator,
            context: context(facts, unit, policies.matching, policies.consumer)?,
        },
        signature: signature(facts)?,
        expression: expression(&facts.body, facts, 0, 512)?.value,
    })
}

fn relative(path: &Path, root: &Path) -> Result<String, String> {
    let suffix = path
        .strip_prefix(root)
        .map_err(|_| "Review locator lies outside its declared root")?;
    let components = suffix
        .components()
        .map(|component| match component {
            Component::Normal(value) => value.to_str().ok_or("Review locator is not UTF-8"),
            _ => Err("Review locator contains an ambiguous path component"),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if components.is_empty() {
        return Err("Review locator is empty".into());
    }
    Ok(components.join("/"))
}

fn original_free_function(facts: &HirFnFacts) -> Result<String, String> {
    use rstyle_pdg::nominal_boundary::census;
    let options = &facts.build.frontend_options;
    let path = options
        .get("source_file")
        .and_then(serde_json::Value::as_str)
        .ok_or("Review source path is missing")?;
    let census = options.data::<Arc<census::Census>>()?;
    let file = census
        .files
        .iter()
        .find(|file| file.path == Path::new(path))
        .ok_or("Review original source file is missing")?;
    let parsed = syn::parse_file(&file.bytes).map_err(|error| error.to_string())?;
    let declarations = free_functions(&parsed.items, &[], 4096)?.functions;

    let matching = declarations
        .iter()
        .filter(|(_, function)| function_range(&file.bytes, function) == Some(facts.fn_span))
        .collect::<Vec<_>>();
    let [(namespace, function)] = matching.as_slice() else {
        return Err("Review requires one original free-function declaration".into());
    };
    if declarations
        .iter()
        .filter(|(scope, other)| scope == namespace && other.sig.ident == function.sig.ident)
        .count()
        != 1
    {
        return Err("Review declaration is ambiguous within its original module".into());
    }
    Ok(path.into())
}

#[derive(Default)]
struct Declarations<'a> {
    functions: Vec<(Vec<String>, &'a syn::ItemFn)>,
    items: usize,
}

fn free_functions<'a>(
    items: &'a [syn::Item],
    namespace: &[String],
    budget: usize,
) -> Result<Declarations<'a>, String> {
    if namespace.len() > 64 {
        return Err("Review original module depth budget exhausted".into());
    }

    items
        .iter()
        .try_fold(Declarations::default(), |mut found, item| {
            let next = free_item(item, namespace, budget - found.items)?;
            found.functions.extend(next.functions);
            found.items += next.items;
            Ok(found)
        })
}

fn free_item<'a>(
    item: &'a syn::Item,
    namespace: &[String],
    budget: usize,
) -> Result<Declarations<'a>, String> {
    let remaining = budget
        .checked_sub(1)
        .ok_or("Review original item budget exhausted")?;
    match item {
        syn::Item::Fn(function) => Ok(Declarations {
            functions: vec![(namespace.to_vec(), function)],
            items: 1,
        }),
        syn::Item::Mod(syn::ItemMod {
            ident,
            content: Some((_, items)),
            ..
        }) => {
            let nested = namespace
                .iter()
                .cloned()
                .chain([ident.to_string()])
                .collect::<Vec<_>>();
            let mut found = free_functions(items, &nested, remaining)?;
            found.items += 1;
            Ok(found)
        }
        _ => Ok(Declarations {
            functions: vec![],
            items: 1,
        }),
    }
}

fn function_range(source: &str, function: &syn::ItemFn) -> Option<rstyle_pdg::ByteSpan> {
    let start = match function.vis {
        syn::Visibility::Inherited => function.sig.span().start(),
        _ => function.vis.span().start(),
    };
    let end = function.block.span().end();
    Some(rstyle_pdg::ByteSpan {
        start: u32::try_from(offset(source, start.line, start.column)?).ok()?,
        end: u32::try_from(offset(source, end.line, end.column)?).ok()?,
    })
}

fn offset(source: &str, line: usize, column: usize) -> Option<usize> {
    let mut lines = source.split_inclusive('\n');
    let prefix = lines
        .by_ref()
        .take(line.checked_sub(1)?)
        .map(str::len)
        .sum::<usize>();
    let text = lines.next().unwrap_or("");
    let suffix = text
        .char_indices()
        .map(|(offset, _)| offset)
        .chain([text.len()])
        .nth(column)?;

    Some(prefix + suffix)
}

fn context(
    facts: &HirFnFacts,
    unit: &capture::Unit,
    matching_policy: &MatchingPolicy,
    consumer_policy: &ConsumerPolicy,
) -> Result<Context, String> {
    let build = &facts.build;
    let width = (build.pointer_width.status == rstyle_pdg::FactStatus::Known)
        .then_some(build.pointer_width.value)
        .flatten()
        .and_then(|width| u16::try_from(width).ok())
        .filter(|width| *width != 0)
        .ok_or("Review target pointer width is unavailable")?;
    let extraction = unit.extraction_profile.policy();
    Ok(Context {
        compiler_commit: text(&build.rustc_commit, "compiler commit")?,
        edition: text(&build.edition, "edition")?,
        data_layout: text(&build.data_layout, "data layout")?,
        compilation_target: CompilationTarget {
            name: unit.target.name.clone(),
            kind: sorted(&unit.target.kind),
            test: unit.test,
        },
        target_triple: text(&build.target_triple, "target triple")?,
        pointer_width: width,
        features: sorted(&build.features),
        cfgs: sorted(&build.cfgs),
        opt_level: text(&build.opt_level, "optimization level")?,
        debug_assertions: boolean(&build.debug_assertions, "debug assertions")?,
        overflow_checks: boolean(&build.overflow_checks, "overflow checks")?,
        panic_strategy: text(&build.panic_strategy, "panic strategy")?,
        extraction_policy: format!("{}/{}", extraction.id, extraction.version),
        matching_policy: matching_policy.clone(),
        consumer_policy: consumer_policy.clone(),
    })
}

fn text(fact: &StringFact, label: &str) -> Result<String, String> {
    match fact.known_value() {
        Some(value) if !value.is_empty() => Ok(value.into()),
        _ => Err(format!("Review {label} is unavailable")),
    }
}

fn boolean(fact: &rstyle_pdg::BoolFact, label: &str) -> Result<bool, String> {
    (fact.status == rstyle_pdg::FactStatus::Known)
        .then_some(fact.value)
        .flatten()
        .ok_or_else(|| format!("Review {label} is unavailable"))
}

fn sorted(values: &[String]) -> Vec<String> {
    let mut values = values.to_vec();
    values.sort();
    values.dedup();
    values
}

fn signature(facts: &HirFnFacts) -> Result<Signature, String> {
    let signature = &facts.signature;
    if signature.has_self
        || signature.params.len() > 64
        || signature.return_ty.normalized.as_deref() != Some("u32")
        || signature
            .params
            .iter()
            .any(|parameter| parameter.ty.normalized.as_deref() != Some("u32"))
    {
        return Err("Review requires a bounded u32 free-function signature".into());
    }
    Ok(Signature {
        parameters: vec![ScalarType::U32; signature.params.len()],
        result: ScalarType::U32,
    })
}

struct NativeExpression {
    value: Expression,
    nodes: usize,
}

fn expression(
    node: &HirNode,
    facts: &HirFnFacts,
    depth: usize,
    budget: usize,
) -> Result<NativeExpression, String> {
    if depth > 64 || budget == 0 {
        return Err("Review native expression budget exhausted".into());
    }
    if node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("u32")
        || node.expansion.is_some()
        || node.is_unsafe_block
        || facts
            .source
            .bytes
            .get(node.span.start as usize..node.span.end as usize)
            != Some(node.snippet.as_str())
    {
        return Err("Review native expression lacks exact original u32 evidence".into());
    }

    let counted = expression_shape(node, facts, depth, budget - 1)?;
    Ok(NativeExpression {
        value: counted.value,
        nodes: counted.nodes + 1,
    })
}

fn expression_shape(
    node: &HirNode,
    facts: &HirFnFacts,
    depth: usize,
    remaining: usize,
) -> Result<NativeExpression, String> {
    match (node.kind.as_str(), node.children.as_slice()) {
        ("expr_block", [child]) => expression(child, facts, depth + 1, remaining),
        ("expr_path", []) => Ok(NativeExpression {
            value: formal(node, facts)?,
            nodes: 0,
        }),
        ("expr_lit", []) => Ok(NativeExpression {
            value: literal(node)?,
            nodes: 0,
        }),
        ("expr_unary", [operand]) if operator(node)? == "!" => {
            let operand = expression(operand, facts, depth + 1, remaining)?;
            Ok(NativeExpression {
                value: Expression::Unary {
                    ty: ScalarType::U32,
                    operator: UnaryOperator::Not,
                    operand: Box::new(operand.value),
                },
                nodes: operand.nodes,
            })
        }
        ("expr_binary", [_, _]) => binary_expression(node, facts, depth, remaining),
        _ => Err("Review expression is outside the completed native helper contract".into()),
    }
}

fn binary_expression(
    node: &HirNode,
    facts: &HirFnFacts,
    depth: usize,
    remaining: usize,
) -> Result<NativeExpression, String> {
    let operator = binary(operator(node)?)?;
    let left = expression(&node.children[0], facts, depth + 1, remaining)?;
    let right = expression(&node.children[1], facts, depth + 1, remaining - left.nodes)?;
    Ok(NativeExpression {
        value: Expression::Binary {
            ty: ScalarType::U32,
            operator,
            left: Box::new(left.value),
            right: Box::new(right.value),
        },
        nodes: left.nodes + right.nodes,
    })
}

fn formal(node: &HirNode, facts: &HirFnFacts) -> Result<Expression, String> {
    let resolution = node
        .res
        .as_ref()
        .filter(|resolution| resolution.kind == "local")
        .ok_or("Review formal resolution is unavailable")?;
    let target = resolution
        .target
        .as_ref()
        .ok_or("Review formal target is missing")?;
    let mut parameters = facts
        .signature
        .params
        .iter()
        .filter(|parameter| &parameter.pat.hir == target);
    let parameter = parameters
        .next()
        .ok_or("Review formal has no signature role")?;

    if parameters.next().is_some() {
        return Err("Review formal has ambiguous signature roles".into());
    }
    Ok(Expression::Formal {
        position: parameter.position,
        ty: ScalarType::U32,
    })
}

fn literal(node: &HirNode) -> Result<Expression, String> {
    let value = node
        .lit
        .as_deref()
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or("Review literal lacks a complete u32 value")?;
    Ok(Expression::Literal {
        ty: ScalarType::U32,
        value: value.to_string(),
        spelling: node.snippet.clone(),
    })
}

fn operator(node: &HirNode) -> Result<&str, String> {
    let dispatch = node
        .dispatch
        .as_ref()
        .ok_or("Review builtin dispatch is missing")?;
    if dispatch.kind != "builtin" || dispatch.result_ty != "u32" {
        return Err("Review requires original builtin u32 dispatch".into());
    }
    Ok(&dispatch.op)
}

fn binary(operator: &str) -> Result<BinaryOperator, String> {
    match operator {
        "+" => Ok(BinaryOperator::Add),
        "-" => Ok(BinaryOperator::Subtract),
        "*" => Ok(BinaryOperator::Multiply),
        "/" => Ok(BinaryOperator::Divide),
        "%" => Ok(BinaryOperator::Remainder),
        "&" => Ok(BinaryOperator::BitAnd),
        "|" => Ok(BinaryOperator::BitOr),
        "^" => Ok(BinaryOperator::BitXor),
        "<<" => Ok(BinaryOperator::ShiftLeft),
        ">>" => Ok(BinaryOperator::ShiftRight),
        _ => Err("Review operator is outside the u32 helper contract".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locators_survive_workspace_relocation_and_reject_escaped_roots() {
        for root in ["/first/workspace", "/second/workspace"] {
            let root = Path::new(root);
            assert_eq!(
                relative(&root.join("crates/sample/Cargo.toml"), root).unwrap(),
                "crates/sample/Cargo.toml"
            );
            let package = root.join("crates/sample");
            assert_eq!(
                relative(&package.join("src/lib.rs"), &package).unwrap(),
                "src/lib.rs"
            );
            assert!(relative(&package.join("../other/lib.rs"), &package).is_err());
        }
        assert!(relative(Path::new("/outside/lib.rs"), Path::new("/workspace")).is_err());
    }

    #[test]
    fn original_declarations_exclude_methods_and_nested_functions_and_keep_duplicates() {
        let source = "mod a { pub fn same() {} } mod b { fn same() {} } fn outer() { fn nested() {} } struct S; impl S { fn method() {} } #[cfg(any())] fn duplicate() {} fn duplicate() {}";
        let parsed = syn::parse_file(source).unwrap();
        let declarations = free_functions(&parsed.items, &[], 4096).unwrap().functions;
        let names = declarations
            .iter()
            .map(|(_, function)| function.sig.ident.to_string())
            .collect::<Vec<_>>();
        assert_eq!(names, ["same", "same", "outer", "duplicate", "duplicate"]);
        assert_eq!(declarations[0].0, ["a"]);
        assert_eq!(declarations[1].0, ["b"]);
        assert!(declarations[3].0.is_empty());
        assert!(declarations[4].0.is_empty());
    }

    #[test]
    fn original_ranges_keep_utf8_crlf_visibility_and_attribute_offsets() {
        let source = "// 🦀\r\n#[inline]\r\npub fn café() -> u32 { 4 }\r\n";
        let parsed = syn::parse_file(source).unwrap();
        let syn::Item::Fn(function) = &parsed.items[0] else {
            panic!("function fixture");
        };
        let range = function_range(source, function).unwrap();
        let expected = "pub fn café() -> u32 { 4 }";
        assert_eq!(range.start as usize, source.find(expected).unwrap());
        assert_eq!(&source[range.start as usize..range.end as usize], expected);
    }

    #[test]
    fn original_item_scan_reports_budget_failure_without_partial_members() {
        let parsed = syn::parse_file("mod m { fn a() {} } fn b() {}").unwrap();
        let complete = free_functions(&parsed.items, &[], 3).unwrap();
        assert_eq!(complete.items, 3);
        assert_eq!(complete.functions.len(), 2);
        assert!(free_functions(&parsed.items, &[], 2).is_err());
        let namespace = vec!["nested".into(); 64];
        assert!(free_functions(&[], &namespace, 0).is_ok());
        let excessive = vec!["nested".into(); 65];
        assert!(free_functions(&[], &excessive, 0).is_err());
    }

    fn budget_facts() -> HirFnFacts {
        // Synthetic counters only; these values do not claim live admission.
        let mut facts: HirFnFacts = serde_json::from_str(include_str!(
            "../../../../rstyle-pdg/src/nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
        )).unwrap();
        facts.source.bytes = rstyle_pdg::SourceText::from("0");
        facts.body = serde_json::from_value(serde_json::json!({
            "hir":{"krate":0,"owner":1,"local":0},"kind":"expr_lit",
            "span":{"start":0,"end":1},"snippet":"0",
            "ty":{"normalized":"u32","observed":"u32","spelling":"u32"},
            "res":null,"dispatch":null,"expansion":null,"binding":null,
            "binding_mut":null,"lit":"0","param_index":null,
            "is_unsafe_block":false,"children":[]
        }))
        .unwrap();
        facts
    }

    fn balanced_tree(leaf: &HirNode, depth: usize) -> HirNode {
        if depth == 0 {
            return leaf.clone();
        }
        HirNode {
            kind: "expr_binary".into(),
            dispatch: Some(rstyle_pdg::Dispatch {
                kind: "builtin".into(),
                op: "+".into(),
                operand_tys: vec!["u32".into(); 2],
                result_ty: "u32".into(),
            }),
            children: vec![
                balanced_tree(leaf, depth - 1),
                balanced_tree(leaf, depth - 1),
            ],
            ..leaf.clone()
        }
    }

    #[test]
    fn native_budget_counts_both_siblings_and_erased_blocks_exactly() {
        let facts = budget_facts();
        let tree = balanced_tree(&facts.body, 8);
        let wrapped = HirNode {
            kind: "expr_block".into(),
            children: vec![tree],
            ..facts.body.clone()
        };
        assert_eq!(expression(&wrapped, &facts, 0, 512).unwrap().nodes, 512);
        assert!(expression(&wrapped, &facts, 0, 511).is_err());
        let excessive = HirNode {
            kind: "expr_block".into(),
            children: vec![wrapped],
            ..facts.body.clone()
        };
        assert!(expression(&excessive, &facts, 0, 512).is_err());
        assert!(expression(&facts.body, &facts, 64, 1).is_ok());
        assert!(expression(&facts.body, &facts, 65, 1).is_err());
    }
}
