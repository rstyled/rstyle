use super::*;
use crate::{
    BindingFact, ByteSpan, Dispatch, HirFnFacts, HirNode, HirRef, ParamFact, PathRes, ResolvedTy,
};

const SIMPLE: &str = "pub fn f(target:&mut u32){*target=1;}";

fn span(source: &str, text: &str) -> ByteSpan {
    let start = source.find(text).unwrap() as u32;
    ByteSpan {
        start,
        end: start + text.len() as u32,
    }
}

fn ty(value: Option<&str>) -> ResolvedTy {
    ResolvedTy {
        normalized: value.map(str::to_owned),
        observed: value.unwrap_or("&mut u32").into(),
        spelling: String::new(),
    }
}

struct NodeType {
    kind: &'static str,
    scalar: Option<&'static str>,
}

impl NodeType {
    fn new(kind: &'static str, scalar: Option<&'static str>) -> Self {
        Self { kind, scalar }
    }
}

struct Nodes<'a> {
    source: &'a str,
    next: u32,
}

impl Nodes<'_> {
    fn node(&mut self, shape: NodeType, span: ByteSpan, children: Vec<HirNode>) -> HirNode {
        self.next += 1;
        HirNode {
            hir: HirRef {
                krate: 0,
                owner: 1,
                local: self.next,
            },
            kind: shape.kind.into(),
            span,
            snippet: crate::source_evidence::byte_range(self.source, span)
                .unwrap()
                .into(),
            ty: Some(ty(shape.scalar)),
            res: None,
            dispatch: None,
            expansion: None,
            binding: None,
            binding_mut: None,
            lit: None,
            param_index: None,
            is_unsafe_block: false,
            children,
        }
    }

    fn assignment(&mut self, text: &str, root: &HirRef) -> HirNode {
        let assignment = span(self.source, text);
        let deref = self.pointee(assignment, root);
        let rhs_start = self.source[assignment.start as usize..assignment.end as usize]
            .find('=')
            .unwrap() as u32
            + assignment.start
            + 1;
        let rhs = &self.source[rhs_start as usize..assignment.end as usize];
        let kind = if rhs.contains('(') {
            "expr_call"
        } else {
            "expr_lit"
        };
        let value = self.node(
            NodeType::new(kind, Some("u32")),
            ByteSpan {
                start: rhs_start,
                end: assignment.end,
            },
            vec![],
        );
        let compound = text.contains("+=");
        let mut result = self.node(
            NodeType::new(
                if compound {
                    "expr_assign_op"
                } else {
                    "expr_assign"
                },
                Some("unit"),
            ),
            assignment,
            vec![deref, value],
        );
        if compound {
            result.dispatch = Some(dispatch("+=", "unit"));
        }
        result
    }

    fn pointee(&mut self, assignment: ByteSpan, root: &HirRef) -> HirNode {
        let target = ByteSpan {
            start: assignment.start + 1,
            end: assignment.start + 7,
        };
        let mut path = self.node(NodeType::new("expr_path", None), target, vec![]);
        path.res = Some(PathRes {
            kind: "local".into(),
            target: Some(root.clone()),
            description: String::new(),
        });
        let mut deref = self.node(
            NodeType::new("expr_unary", Some("u32")),
            ByteSpan {
                start: assignment.start,
                end: target.end,
            },
            vec![path],
        );
        deref.dispatch = Some(dispatch("*", "u32"));
        deref
    }

    fn owned_assignment(&mut self, text: &str, root: &HirRef) -> HirNode {
        let assignment = span(self.source, text);
        let target = ByteSpan {
            start: assignment.start,
            end: assignment.start + text.find('=').unwrap() as u32,
        };
        let mut path = self.node(NodeType::new("expr_path", Some("u32")), target, vec![]);
        path.res = Some(PathRes {
            kind: "local".into(),
            target: Some(root.clone()),
            description: String::new(),
        });
        let rhs = self.node(
            NodeType::new("expr_lit", Some("u32")),
            ByteSpan {
                start: assignment.end - 1,
                end: assignment.end,
            },
            vec![],
        );
        self.node(
            NodeType::new("expr_assign", Some("unit")),
            assignment,
            vec![path, rhs],
        )
    }
}

fn dispatch(op: &str, result: &str) -> Dispatch {
    Dispatch {
        kind: "builtin".into(),
        op: op.into(),
        operand_tys: vec![],
        result_ty: result.into(),
    }
}

// Deliberately author native records separately from the consumer's traversal.
fn fixture(source: &str, writes: &[&str]) -> HirFnFacts {
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    let mut nodes = Nodes { source, next: 0 };
    let pattern = formal(&mut nodes, &mut facts);
    let children = writes
        .iter()
        .map(|text| nodes.assignment(text, &pattern.hir))
        .collect();
    let body_span = ByteSpan {
        start: source.find('{').unwrap() as u32,
        end: source.len() as u32,
    };
    facts.body = nodes.node(
        NodeType::new("expr_block", Some("unit")),
        body_span,
        children,
    );
    source_metadata(&mut facts, source, body_span);
    facts
}

fn formal(nodes: &mut Nodes<'_>, facts: &mut HirFnFacts) -> HirNode {
    let source = nodes.source;
    let annotation = if source.contains("&mut u32") {
        "&mut u32"
    } else {
        "u32"
    };
    let scalar = (annotation == "u32").then_some("u32");
    let declaration = span(source, "target");
    let mut pattern = nodes.node(NodeType::new("pat_binding", scalar), declaration, vec![]);
    pattern.binding = Some("s:target".into());
    let mutable = source.contains("mut target:");
    pattern.binding_mut = Some(mutable);
    pattern.param_index = Some(0);
    pattern.ty.as_mut().unwrap().spelling = annotation.into();
    facts.bindings = vec![BindingFact {
        symbol: "s:target".into(),
        name: "target".into(),
        decl_span: declaration,
        scope: format!("scope:source:fn:0:{}", source.len()),
        hir: pattern.hir.clone(),
        is_param: true,
        is_mut: mutable,
        ty: ty(scalar),
    }];
    facts.signature.params = vec![ParamFact {
        symbol: "s:target".into(),
        position: 0,
        name_span: declaration,
        ty_span: span(source, annotation),
        ty: ty(scalar),
        pat: pattern,
    }];
    facts.signature.params[0].pat.clone()
}

fn source_metadata(facts: &mut HirFnFacts, source: &str, body_span: ByteSpan) {
    facts.signature.has_self = false;
    facts.signature.receiver = None;
    facts.signature.is_variadic = false;
    facts.signature.sig_span = ByteSpan {
        start: 0,
        end: body_span.start,
    };
    facts.source.bytes = source.into();
    facts.source.digest = facts.source.bytes.checksum().into();
    facts.fn_span = ByteSpan {
        start: 0,
        end: source.len() as u32,
    };
    facts.source_diagnostics.clear();
    facts.build.frontend_options = Default::default();
}

fn assess(facts: &HirFnFacts) -> Result<Assessment, String> {
    source::inspect(facts, &facts.source.bytes, Assessment::empty())
}

#[test]
fn direct_pointee_site_keeps_original_root_and_call_visibility() {
    let facts = fixture(SIMPLE, &["*target=1"]);
    let result = assess(&facts).unwrap();
    assert_eq!(result.coverage, Coverage::Complete);
    assert_eq!(result.sites.len(), 1);
    let site = &result.sites[0];
    assert_eq!(site.span, span(SIMPLE, "*target=1"));
    assert_eq!(site.root.declaration_span, span(SIMPLE, "target"));
    assert_eq!(site.root.native, facts.bindings[0].hir);
    assert_eq!(site.root.provenance, Provenance::FormalBorrowed);
    assert_eq!(site.root.formal, Some(0));
    assert!(site.caller_visible);
}

#[test]
fn builtin_compound_is_one_site_and_overload_is_not_a_witness() {
    let source = SIMPLE.replace("=1", "+=1");
    let mut facts = fixture(&source, &["*target+=1"]);
    let result = assess(&facts).unwrap();
    assert_eq!(result.sites.len(), 1);
    assert_eq!(result.sites[0].operation, Operation::CompoundAssignment);
    facts.body.children[0].dispatch.as_mut().unwrap().kind = "overloaded".into();
    let result = assess(&facts).unwrap();
    assert!(result.sites.is_empty());
    assert_eq!(result.coverage, Coverage::Incomplete);
}

#[test]
fn explicit_store_with_opaque_rhs_keeps_only_a_lower_bound() {
    let source = "pub fn f(target:&mut u32){*target=opaque();}";
    let result = assess(&fixture(source, &["*target=opaque()"])).unwrap();
    assert_eq!(result.sites.len(), 1);
    assert_eq!(result.coverage, Coverage::Incomplete);
    assert!(
        result
            .boundaries
            .iter()
            .any(|boundary| boundary.reason.contains("call"))
    );
}

#[test]
fn display_type_is_not_primitive_authority() {
    let mut facts = fixture(SIMPLE, &["*target=1"]);
    let resolved = facts.body.children[0].children[0].ty.as_mut().unwrap();
    resolved.normalized = None;
    resolved.observed = "u32".into();
    let result = assess(&facts).unwrap();
    assert!(result.sites.is_empty());
    assert_eq!(result.coverage, Coverage::Incomplete);
}

#[test]
fn contradictory_scope_identity_and_native_operand_spans_fail() {
    let mut facts = fixture(SIMPLE, &["*target=1"]);
    facts.bindings[0].scope = "scope:source:fn:0:1".into();
    assert!(assess(&facts).is_err());
    let mut facts = fixture(SIMPLE, &["*target=1"]);
    facts.body.children[0].children.swap(0, 1);
    assert!(assess(&facts).is_err());
    let mut facts = fixture(SIMPLE, &["*target=1"]);
    facts.body.children[0].children[0].children[0]
        .res
        .as_mut()
        .unwrap()
        .target
        .as_mut()
        .unwrap()
        .local += 100;
    assert!(assess(&facts).is_err());
}

#[test]
fn dereference_requires_the_exact_original_operand_roster() {
    let mut facts = fixture(SIMPLE, &["*target=1"]);
    facts.body.children[0].children[0].children.clear();
    assert!(
        assess(&facts)
            .unwrap_err()
            .contains("dereference operand roster")
    );
    let mut facts = fixture(SIMPLE, &["*target=1"]);
    let path = &mut facts.body.children[0].children[0].children[0];
    path.span = span(SIMPLE, "1");
    path.snippet = "1".into();
    assert!(
        assess(&facts)
            .unwrap_err()
            .contains("dereference operand roster")
    );
}

#[test]
fn missing_native_assignment_is_unavailable_instead_of_zero_exact() {
    let result = assess(&fixture(SIMPLE, &[])).unwrap();
    assert!(result.sites.is_empty());
    assert_eq!(result.coverage, Coverage::Incomplete);
}

#[test]
fn formal_signature_and_binding_type_claims_must_agree() {
    let mut facts = fixture(SIMPLE, &["*target=1"]);
    facts.signature.params[0].ty.normalized = Some("u32".into());
    assert!(assess(&facts).is_err());
}

#[test]
fn opaque_shadowing_does_not_turn_valid_unsupported_syntax_into_corruption() {
    for prefix in [
        "shadow!(target);",
        "let whole @ (mut target,_)=(0u32,1u32);",
        "let shadow!()=0u32;",
        "#[opaque] let mut value=0u32;",
        "#[opaque] 0u32;",
    ] {
        let source = format!("pub fn f(mut target:u32){{{prefix}target=1;}}");
        let mut facts = fixture(&source, &[]);
        let mut nodes = Nodes {
            source: &source,
            next: 100,
        };
        let other = HirRef {
            krate: 0,
            owner: 1,
            local: 99,
        };
        facts.body.children = vec![nodes.owned_assignment("target=1", &other)];
        let result = assess(&facts).unwrap();
        assert_eq!(result.coverage, Coverage::Incomplete);
        assert!(result.sites.is_empty());
    }
}

#[test]
fn unsupported_destructuring_still_shadows_the_outer_root() {
    let source = "pub fn f(target:&mut u32){let (target,_)=(&mut 0u32,0);*target=1;}";
    let mut facts = fixture(source, &["*target=1"]);
    facts.body.children[0].children[0].children[0]
        .res
        .as_mut()
        .unwrap()
        .target
        .as_mut()
        .unwrap()
        .local = 99;
    let result = assess(&facts).unwrap();
    assert!(result.sites.is_empty());
    assert_eq!(result.coverage, Coverage::Incomplete);
}

fn local_binding(nodes: &mut Nodes<'_>, text: &str, scope: ByteSpan) -> (HirNode, BindingFact) {
    let declaration = span(nodes.source, text);
    let declaration = ByteSpan {
        start: declaration.start + 4,
        end: declaration.start + 9,
    };
    let mut pattern = nodes.node(
        NodeType::new("pat_binding", Some("u32")),
        declaration,
        vec![],
    );
    pattern.binding = Some(format!("s:{}", pattern.hir.local));
    pattern.binding_mut = Some(true);
    let binding = BindingFact {
        symbol: pattern.binding.clone().unwrap(),
        name: "value".into(),
        decl_span: declaration,
        scope: format!("scope:source:block:{}:{}", scope.start, scope.end),
        hir: pattern.hir.clone(),
        is_param: false,
        is_mut: true,
        ty: ty(Some("u32")),
    };
    (pattern, binding)
}

#[test]
fn nested_scalar_shadowing_preserves_distinct_declaration_and_scope_roots() {
    let source =
        "pub fn f(target:&mut u32){let mut value=0u32;value=1;{let mut value=2u32;value=3;}}";
    let mut facts = fixture(source, &[]);
    let mut nodes = Nodes { source, next: 100 };
    let outer_scope = facts.body.span;
    let inner_scope = span(source, "{let mut value=2u32;value=3;}");
    let (outer, outer_binding) = local_binding(&mut nodes, "mut value=0u32", outer_scope);
    let (inner, inner_binding) = local_binding(&mut nodes, "mut value=2u32", inner_scope);
    let write_outer = nodes.owned_assignment("value=1", &outer.hir);
    let write_inner = nodes.owned_assignment("value=3", &inner.hir);
    let zero = nodes.node(
        NodeType::new("expr_lit", Some("u32")),
        span(source, "0u32"),
        vec![],
    );
    let two = nodes.node(
        NodeType::new("expr_lit", Some("u32")),
        span(source, "2u32"),
        vec![],
    );
    let block = nodes.node(
        NodeType::new("expr_block", Some("unit")),
        inner_scope,
        vec![inner, two, write_inner],
    );
    facts.body.children = vec![outer, zero, write_outer, block];
    facts.bindings.extend([outer_binding, inner_binding]);
    let result = assess(&facts).unwrap();
    assert_eq!(result.coverage, Coverage::Complete);
    assert_eq!(result.sites.len(), 2);
    assert_ne!(result.sites[0].root, result.sites[1].root);
    assert_eq!(result.sites[0].root.provenance, Provenance::LocalOwned);
    assert_eq!(result.sites[0].root.scope_span, outer_scope);
    assert_eq!(result.sites[1].root.scope_span, inner_scope);
}

#[test]
fn deferred_closure_does_not_mint_outer_write_sites() {
    let source = "pub fn f(target:&mut u32){let callback=||{*target=1;};}";
    let result = assess(&fixture(source, &["*target=1"])).unwrap();
    assert!(result.sites.is_empty());
    assert_eq!(result.coverage, Coverage::Incomplete);
}

#[test]
fn opaque_regions_cannot_preserve_stale_pointee_roots() {
    for prefix in [
        "macro_rules! redirect {()=>{target=other;}} redirect!();",
        "unsafe { redirect_raw(); }",
        "callback(&raw mut target);",
        "(target)=target;",
        "swap(&mut (target), &mut (other));",
        "(target,)=(target,);",
        "let ref mut alias=target; *alias=other;",
        "if false { #[opaque] 0u32; }",
    ] {
        let source = format!("pub fn f(mut target:&mut u32){{{prefix} *target=1;}}");
        let result = assess(&fixture(&source, &["*target=1"])).unwrap();
        assert!(result.sites.is_empty(), "{source}");
        assert_eq!(result.coverage, Coverage::Incomplete);
    }
}

#[test]
fn function_attributes_preclude_all_original_body_witnesses() {
    let source = format!("#[opaque] {SIMPLE}");
    let result = assess(&fixture(&source, &["*target=1"])).unwrap();
    assert!(result.sites.is_empty());
    assert_eq!(result.coverage, Coverage::Incomplete);
}

#[test]
fn delimiter_free_recursive_syntax_is_bounded_before_parsing() {
    assert!(!source::within_budget(&format!(
        "fn f(){{{}true}}",
        "!".repeat(1000)
    )));
    assert!(!source::within_budget(&format!(
        "fn f(){{{}0}}",
        "return ".repeat(1000)
    )));
    assert!(source::within_budget(SIMPLE));
}

#[test]
fn corrupt_oversized_source_cannot_skip_identity_checks() {
    let mut facts = fixture(SIMPLE, &["*target=1"]);
    facts.source.bytes = format!("/*{}*/{SIMPLE}", " ".repeat(256 * 1024)).into();
    assert!(inspect(&facts).is_err());
}

#[test]
fn unknown_calls_and_unsupported_closures_keep_distinct_boundaries() {
    let source = "pub fn f(target:&mut u32){drop(target); ||{};}";
    let result = assess(&fixture(source, &[])).unwrap();
    assert_eq!(result.coverage, Coverage::Incomplete);
    assert!(result.sites.is_empty());
    assert!(
        result
            .boundaries
            .iter()
            .any(|boundary| { boundary.availability == BoundaryAvailability::Unknown })
    );
    assert!(
        result
            .boundaries
            .iter()
            .any(|boundary| { boundary.availability == BoundaryAvailability::Unsupported })
    );
}

#[test]
fn oversized_binding_inventory_is_bounded_before_building_the_index() {
    let mut facts = fixture(SIMPLE, &["*target=1"]);
    facts.bindings = vec![facts.bindings[0].clone(); 16385];
    let result = assess(&facts).unwrap();
    assert_eq!(result.execution, Execution::BudgetExhausted);
    assert!(result.sites.is_empty());
    assert_eq!(result.coverage, Coverage::Incomplete);
}
