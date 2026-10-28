use super::*;
use crate::{ByteSpan, HirNode, HirRef};
use std::sync::Arc;
use syn::spanned::Spanned;

fn span(source: &str, syntax: &impl Spanned) -> ByteSpan {
    crate::source_evidence::original_span(source, syntax.span())
}

struct NodeShape {
    kind: &'static str,
    token: Option<&'static str>,
}

fn node(source: &str, shape: NodeShape, span: ByteSpan, local: u32) -> HirNode {
    HirNode {
        hir: HirRef {
            krate: 0,
            owner: 1,
            local,
        },
        kind: shape.kind.into(),
        span,
        snippet: crate::source_evidence::byte_range(source, span)
            .unwrap()
            .into(),
        ty: Some(crate::ResolvedTy {
            normalized: shape.token.map(str::to_owned),
            observed: String::new(),
            spelling: String::new(),
        }),
        res: None,
        dispatch: None,
        expansion: None,
        binding: None,
        binding_mut: None,
        lit: None,
        param_index: None,
        is_unsafe_block: false,
        children: vec![],
    }
}

struct Fixture<'a> {
    source: &'a str,
    facts: HirFnFacts,
}

impl<'a> Fixture<'a> {
    fn new(source: &'a str) -> Self {
        let mut facts: HirFnFacts = serde_json::from_str(include_str!(
            "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
        ))
        .unwrap();
        facts.source.bytes = source.into();
        facts.source.digest = facts.source.bytes.checksum().into();
        facts.fn_span = ByteSpan {
            start: 0,
            end: source.len() as u32,
        };
        facts.source_diagnostics.clear();
        facts.bindings.clear();
        facts.signature.params.clear();

        Self { source, facts }
    }

    fn signature(&mut self, function: &syn::ItemFn) -> Vec<super::super::formal_types::Formal> {
        self.facts.signature.sig_span = super::super::header_span(self.source, function);
        self.facts.signature.has_self = false;
        self.facts.signature.receiver = None;
        self.facts.signature.is_variadic = false;
        self.facts.signature.return_ty.normalized = Some("unit".into());

        function
            .sig
            .inputs
            .iter()
            .enumerate()
            .filter_map(|(ordinal, argument)| self.formal(ordinal, argument))
            .collect()
    }

    fn formal(
        &mut self,
        ordinal: usize,
        syntax: &syn::FnArg,
    ) -> Option<super::super::formal_types::Formal> {
        let syn::FnArg::Typed(argument) = syntax else {
            unreachable!()
        };
        let syn::Pat::Ident(ident) = &*argument.pat else {
            unreachable!()
        };
        let scalar = crate::source_evidence::plain_type_path(&argument.ty, "u32").then_some("u32");
        let declaration = span(self.source, &ident.ident);
        let mut pattern = node(
            self.source,
            NodeShape {
                kind: "pat_binding",
                token: scalar,
            },
            declaration,
            ordinal as u32 + 1,
        );
        pattern.param_index = Some(ordinal as u32);
        pattern.binding = Some(format!("s:{ordinal}"));
        pattern.binding_mut = Some(ident.mutability.is_some());

        self.facts.bindings.push(crate::BindingFact {
            symbol: format!("s:{ordinal}"),
            name: ident.ident.to_string(),
            decl_span: declaration,
            scope: format!("scope:source:fn:0:{}", self.source.len()),
            hir: pattern.hir.clone(),
            is_param: true,
            is_mut: ident.mutability.is_some(),
            ty: pattern.ty.clone().unwrap(),
        });
        self.facts.signature.params.push(crate::ParamFact {
            symbol: format!("s:{ordinal}"),
            position: ordinal as u32,
            name_span: declaration,
            ty_span: span(self.source, &argument.ty),
            ty: pattern.ty.clone().unwrap(),
            pat: pattern.clone(),
        });

        matches!(&*argument.ty, syn::Type::Reference(_)).then_some(
            super::super::formal_types::Formal {
                parameter: ordinal as u32,
                binding: pattern.hir,
            },
        )
    }

    fn body(&mut self, function: &syn::ItemFn) {
        let mut body = node(
            self.source,
            NodeShape {
                kind: "expr_block",
                token: Some("unit"),
            },
            span(self.source, &function.block),
            1000,
        );
        body.children = function
            .block
            .stmts
            .iter()
            .enumerate()
            .filter_map(|(ordinal, statement)| {
                let syn::Stmt::Expr(syn::Expr::Call(call), _) = statement else {
                    return None;
                };
                Some(self.call(ordinal, call))
            })
            .collect();

        self.facts.body = body;
    }

    fn call(&self, ordinal: usize, call: &syn::ExprCall) -> HirNode {
        let mut captured = node(
            self.source,
            NodeShape {
                kind: "expr_call",
                token: Some("unit"),
            },
            span(self.source, call),
            100 + ordinal as u32 * 10,
        );
        captured.children = call
            .args
            .iter()
            .enumerate()
            .map(|(position, argument)| {
                self.argument(argument, 101 + ordinal as u32 * 10 + position as u32)
            })
            .collect();
        captured.dispatch = Some(crate::Dispatch {
            kind: "resolved_direct".into(),
            op: "sample::leaf".into(),
            operand_tys: captured
                .children
                .iter()
                .map(|node| {
                    node.ty
                        .as_ref()
                        .unwrap()
                        .normalized
                        .clone()
                        .unwrap_or("&mut u32".into())
                })
                .collect(),
            result_ty: "unit".into(),
        });
        captured.res = Some(crate::PathRes {
            kind: "resolved_direct_callee".into(), target: None,
            description: serde_json::json!({"local":true,"path":"sample::leaf","crate":"sample","definition_identity":"leaf-id"}).to_string(),
        });

        captured
    }

    fn argument(&self, argument: &syn::Expr, local: u32) -> HirNode {
        let mut captured = node(
            self.source,
            NodeShape {
                kind: "expr_path",
                token: None,
            },
            span(self.source, argument),
            local,
        );
        if matches!(argument, syn::Expr::Lit(_)) {
            captured.kind = "expr_lit".into();
            captured.ty.as_mut().unwrap().normalized = Some("u32".into());
        }
        if let syn::Expr::Path(path) = argument {
            let name = path.path.get_ident().unwrap().to_string();
            if let Some(binding) = self
                .facts
                .bindings
                .iter()
                .find(|binding| binding.name == name)
            {
                captured.ty = Some(binding.ty.clone());
                captured.res = Some(crate::PathRes {
                    kind: "local".into(),
                    target: Some(binding.hir.clone()),
                    description: String::new(),
                });
            }
        }

        captured
    }
}

fn fixture(source: &str) -> HirFnFacts {
    let function: syn::ItemFn = syn::parse_str(source).unwrap();
    let mut fixture = Fixture::new(source);
    let formals = fixture.signature(&function);
    fixture.body(&function);
    ownership(&mut fixture.facts, source);
    fixture.facts.build.frontend_options.insert(
        super::super::formal_types::KEY.into(),
        serde_json::to_value(super::super::formal_types::Observations {
            identity: super::super::formal_types::IDENTITY.into(),
            mutable_u32: formals,
        })
        .unwrap(),
    );

    fixture.facts
}

pub(super) fn ownership(facts: &mut HirFnFacts, source: &str) {
    facts.build.frontend_options = Default::default();
    let mut context = crate::nominal_boundary::tests::context();
    context.package.manifest_digest = context.package.manifest_bytes.checksum().into();
    let path = context.root_source.clone();
    facts.build.frontend_options.insert(
        crate::nominal_boundary::CONTEXT_KEY.into(),
        serde_json::to_value(&context).unwrap(),
    );
    facts
        .build
        .frontend_options
        .insert("source_file".into(), serde_json::json!(path));
    facts
        .build
        .frontend_options
        .insert_data(Arc::new(crate::nominal_boundary::census::Census {
            identity: crate::nominal_boundary::census::IDENTITY.into(),
            context,
            files: vec![crate::nominal_boundary::census::UnitFile {
                path,
                bytes: source.into(),
                digest: facts.source.digest.clone(),
            }],
            expansions: vec![],
            documentation: vec![],
            external_imports: vec![],
        }));
}

#[test]
fn direct_call_keeps_boundary_and_exact_mutable_formal_argument() {
    let facts = fixture("pub fn caller(target:&mut u32) { leaf(target); }");
    let observed = inspect_calls(&facts).unwrap();
    assert!(matches!(observed.signature, Availability::Known(())));
    assert_eq!(observed.calls.len(), 1);
    assert_eq!(
        observed.calls[0].target_path.as_deref(),
        Some("sample::leaf")
    );
    let call = &observed.calls[0];
    assert_eq!(
        observed.direct.boundaries[call.boundary_index].span,
        call.span
    );
    assert!(matches!(&call.target,Availability::Known(target) if target=="leaf-id"));
    assert!(
        matches!(&call.arguments[0].value,Availability::Known(ArgumentValue::MutableFormal(root)) if root.formal==Some(0) && root.native==facts.signature.params[0].pat.hir)
    );
    assert_eq!(observed.direct.sites.len(), 0);
    assert_eq!(
        serde_json::to_value(inspect(&facts).unwrap()).unwrap(),
        serde_json::to_value(observed.direct).unwrap()
    );
}

#[test]
fn repeated_calls_preserve_distinct_boundaries_and_roots() {
    let facts = fixture("pub fn caller(target:&mut u32) { leaf(target); leaf(target); }");
    let observed = inspect_calls(&facts).unwrap();
    assert_eq!(observed.calls.len(), 2);
    assert_ne!(observed.calls[0].span, observed.calls[1].span);
    assert_ne!(
        observed.calls[0].boundary_index,
        observed.calls[1].boundary_index
    );
}

#[test]
fn resealed_argument_target_roster_dispatch_and_formals_fail() {
    let original = fixture("pub fn caller(left:&mut u32,right:&mut u32) { leaf(left,right); }");
    let mut changed = original.clone();
    changed.body.children[0].children[0]
        .res
        .as_mut()
        .unwrap()
        .target = Some(changed.signature.params[1].pat.hir.clone());
    assert!(inspect_calls(&changed).is_err());
    let mut changed = original.clone();
    changed.body.children[0].children.swap(0, 1);
    assert!(inspect_calls(&changed).is_err());
    let mut changed = original.clone();
    changed.body.children[0]
        .dispatch
        .as_mut()
        .unwrap()
        .operand_tys
        .pop();
    assert!(inspect_calls(&changed).is_err());
    let mut changed = original;
    changed.signature.params[0].position = 1;
    assert!(inspect_calls(&changed).is_err());
}

#[test]
fn absent_reference_observation_and_unsupported_signature_stay_unavailable() {
    let mut facts = fixture("pub fn caller(target:&mut u32,callback:fn()) { leaf(target); }");
    let observed = inspect_calls(&facts).unwrap();
    assert!(matches!(
        observed.signature,
        Availability::Unsupported { .. }
    ));
    assert!(matches!(
        observed.calls[0].arguments[0].value,
        Availability::Known(ArgumentValue::MutableFormal(_))
    ));
    facts
        .build
        .frontend_options
        .remove(super::super::formal_types::KEY);
    let observed = inspect_calls(&facts).unwrap();
    assert!(matches!(
        observed.calls[0].arguments[0].value,
        Availability::Unknown { .. }
    ));
}

#[test]
fn complex_argument_retains_opaque_evaluation_and_external_target_unknown() {
    let mut facts = fixture("pub fn caller(value:u32) { leaf({value}); }");
    let observed = inspect_calls(&facts).unwrap();
    assert!(matches!(
        observed.calls[0].arguments[0].value,
        Availability::Unsupported { .. }
    ));
    facts.body.children[0].res.as_mut().unwrap().description =
        serde_json::json!({"local":false}).to_string();
    assert!(matches!(
        inspect_calls(&facts).unwrap().calls[0].target,
        Availability::Unknown { .. }
    ));
}

fn external_record(facts: &mut HirFnFacts) {
    facts.body.children[0].res.as_mut().unwrap().description = serde_json::json!({
        "local":false,"safe":true,"path":"sample::leaf","crate":"sample",
        "definition_identity":"external-leaf-id"
    })
    .to_string();
}

#[test]
fn external_resolution_keeps_identity_without_local_or_argument_permission() {
    let mut facts = fixture("pub fn caller(value:u32) { leaf({value}); }");
    external_record(&mut facts);
    let observed = inspect_calls(&facts).unwrap();
    let call = &observed.calls[0];
    assert_eq!(
        call.resolved_callee,
        Availability::Known(ResolvedCallee {
            path: "sample::leaf".into(),
            local: false,
            safe: true,
            result_type: "unit".into(),
        })
    );
    assert!(matches!(call.target, Availability::Unknown { .. }));
    assert!(call.target_path.is_none());
    assert!(matches!(
        call.arguments[0].value,
        Availability::Unsupported { .. }
    ));
    assert_eq!(observed.direct.coverage, Coverage::Incomplete);
}

#[test]
fn never_result_requires_matching_native_type_and_dispatch() {
    let mut facts = fixture("pub fn caller() { leaf(); }");
    external_record(&mut facts);
    let call = &mut facts.body.children[0];
    call.ty.as_mut().unwrap().normalized = None;
    call.ty.as_mut().unwrap().observed = "!".into();
    call.dispatch.as_mut().unwrap().result_ty = "!".into();
    let observed = inspect_calls(&facts).unwrap();
    assert!(
        matches!(&observed.calls[0].resolved_callee, Availability::Known(callee) if callee.result_type == "!")
    );
    for observed in ["", "unit", "Alias"] {
        facts.body.children[0].ty.as_mut().unwrap().observed = observed.into();
        assert!(matches!(
            inspect_calls(&facts).unwrap().calls[0].resolved_callee,
            Availability::Unknown { .. }
        ));
    }
}

#[test]
fn external_resolution_rejects_path_contradiction_before_missing_safety() {
    let mut facts = fixture("pub fn caller() { leaf(); }");
    external_record(&mut facts);
    facts.body.children[0].dispatch.as_mut().unwrap().op = "sample::other".into();
    let resolution = facts.body.children[0].res.as_mut().unwrap();
    let mut record: serde_json::Value = serde_json::from_str(&resolution.description).unwrap();
    record.as_object_mut().unwrap().remove("safe");
    resolution.description = record.to_string();
    assert!(inspect_calls(&facts).is_err());
}

#[test]
fn missing_callee_metadata_preserves_older_local_summaries() {
    let facts = fixture("pub fn caller() { leaf(); }");
    let observed = inspect_calls(&facts).unwrap();
    assert!(matches!(observed.calls[0].target, Availability::Known(_)));
    assert!(matches!(
        observed.calls[0].resolved_callee,
        Availability::Unknown { .. }
    ));
    for missing in ["safe", "crate", "definition_identity", "path"] {
        let mut changed = facts.clone();
        external_record(&mut changed);
        let resolution = changed.body.children[0].res.as_mut().unwrap();
        let mut record: serde_json::Value = serde_json::from_str(&resolution.description).unwrap();
        record.as_object_mut().unwrap().remove(missing);
        resolution.description = record.to_string();
        assert!(matches!(
            inspect_calls(&changed).unwrap().calls[0].resolved_callee,
            Availability::Unknown { .. }
        ));
    }
}

#[test]
fn resolved_callees_exclude_generic_and_associated_selection() {
    let mut facts = fixture("pub fn caller() { leaf::<u32>(); }");
    external_record(&mut facts);
    assert!(matches!(
        inspect_calls(&facts).unwrap().calls[0].resolved_callee,
        Availability::Unsupported { .. }
    ));
    let mut facts = fixture("pub fn caller() { leaf(); }");
    external_record(&mut facts);
    let resolution = facts.body.children[0].res.as_mut().unwrap();
    let mut record: serde_json::Value = serde_json::from_str(&resolution.description).unwrap();
    record["inherent"] = serde_json::json!({"owner":"sample::Owner"});
    resolution.description = record.to_string();
    assert!(matches!(
        inspect_calls(&facts).unwrap().calls[0].resolved_callee,
        Availability::Unsupported { .. }
    ));
}

#[test]
fn owned_reads_require_original_binding_type_and_dispatch_correspondence() {
    let original = fixture("pub fn caller(value:u32) { leaf(value); }");
    let observed = inspect_calls(&original).unwrap();
    assert!(matches!(observed.signature, Availability::Known(())));
    assert!(matches!(
        observed.calls[0].arguments[0].value,
        Availability::Known(ArgumentValue::OwnedU32)
    ));
    let mut changed = original.clone();
    changed.body.children[0].children[0]
        .ty
        .as_mut()
        .unwrap()
        .normalized = Some("bool".into());
    assert!(inspect_calls(&changed).is_err());
    let mut changed = original;
    changed.body.children[0]
        .dispatch
        .as_mut()
        .unwrap()
        .operand_tys[0] = "u64".into();
    assert!(inspect_calls(&changed).is_err());
}

#[test]
fn missing_native_argument_type_is_unavailable_not_a_known_reference() {
    let mut facts = fixture("pub fn caller(target:&mut u32) { leaf(target); }");
    facts.body.children[0].children[0].ty = None;
    assert!(matches!(
        inspect_calls(&facts).unwrap().calls[0].arguments[0].value,
        Availability::Unknown { .. }
    ));
}

#[test]
fn unsupported_alias_argument_never_inherits_a_known_formal_root() {
    let facts = fixture("pub fn caller(target:&mut u32) { leaf(&mut *target); }");
    let observed = inspect_calls(&facts).unwrap();
    assert!(matches!(
        observed.calls[0].arguments[0].value,
        Availability::Unsupported { .. }
    ));
    assert_eq!(observed.direct.coverage, Coverage::Incomplete);
}

#[test]
fn selected_definition_changes_retain_the_checked_original_path_for_join() {
    let mut facts = fixture("pub fn caller(value:u32) { leaf(value); }");
    facts.body.children[0].res.as_mut().unwrap().description = serde_json::json!({
        "local":true,"path":"sample::leaf","crate":"sample","definition_identity":"different-leaf-id"
    }).to_string();
    let observed = inspect_calls(&facts).unwrap();
    assert!(
        matches!(&observed.calls[0].target, Availability::Known(id) if id=="different-leaf-id")
    );
    assert_eq!(
        observed.calls[0].target_path.as_deref(),
        Some("sample::leaf")
    );
}

#[test]
fn direct_owned_argument_retains_checked_original_formal_ordinal() {
    let facts = fixture("pub fn caller(left:u32,right:u32) { leaf(right,left); }");
    let observed = inspect_calls(&facts).unwrap();
    assert_eq!(
        observed.calls[0].arguments[0].formal,
        Availability::Known(Some(1))
    );
    assert_eq!(
        observed.calls[0].arguments[1].formal,
        Availability::Known(Some(0))
    );
    let facts = fixture("pub fn caller(value:u32) { leaf({value}); }");
    let observed = inspect_calls(&facts).unwrap();
    assert!(matches!(
        observed.calls[0].arguments[0].formal,
        Availability::Unsupported { .. }
    ));
}

#[test]
fn unchanged_formal_requires_an_immutable_owned_original_read() {
    let facts = fixture("pub fn caller(left:u32,right:u32) { leaf(right,left); }");
    let observed = inspect_calls(&facts).unwrap();
    assert_eq!(
        observed.calls[0].arguments[0].unchanged_formal,
        Availability::Known(Some(1))
    );
    assert_eq!(
        observed.calls[0].arguments[1].unchanged_formal,
        Availability::Known(Some(0))
    );
    for source in [
        "pub fn caller(mut value:u32) { leaf(value); }",
        "pub fn caller(value:&mut u32) { leaf(value); }",
    ] {
        let facts = fixture(source);
        let observed = inspect_calls(&facts).unwrap();
        assert_eq!(
            observed.calls[0].arguments[0].formal,
            Availability::Known(Some(0))
        );
        assert_eq!(
            observed.calls[0].arguments[0].unchanged_formal,
            Availability::Known(None)
        );
    }
}

#[test]
fn unchanged_formal_rejects_resealed_binding_modes_and_targets() {
    let original = fixture("pub fn caller(left:u32,right:u32) { leaf(left); }");
    let mut changed = original.clone();
    changed.bindings[0].is_mut = true;
    changed.signature.params[0].pat.binding_mut = Some(true);
    assert!(inspect_calls(&changed).is_err());
    let mut changed = original;
    changed.body.children[0].children[0]
        .res
        .as_mut()
        .unwrap()
        .target = Some(changed.signature.params[1].pat.hir.clone());
    assert!(inspect_calls(&changed).is_err());
    let mut changed = fixture("pub fn caller(mut value:u32) { leaf(value); }");
    changed.bindings[0].is_mut = false;
    changed.signature.params[0].pat.binding_mut = Some(false);
    assert!(inspect_calls(&changed).is_err());
}

#[test]
fn unchanged_formal_preserves_missing_and_unsupported_evidence() {
    let mut facts = fixture("pub fn caller(value:u32) { leaf(value); }");
    facts.body.children[0].children[0].ty = None;
    let observed = inspect_calls(&facts).unwrap();
    assert!(matches!(
        observed.calls[0].arguments[0].unchanged_formal,
        Availability::Unknown { .. }
    ));
    let facts = fixture("pub fn caller(value:u32) { leaf({value}); }");
    let observed = inspect_calls(&facts).unwrap();
    assert!(matches!(
        observed.calls[0].arguments[0].unchanged_formal,
        Availability::Unsupported { .. }
    ));
}

fn local_alias_binding(pattern: &HirNode, scope: ByteSpan) -> crate::BindingFact {
    crate::BindingFact {
        symbol: "local-saved".into(),
        name: "saved".into(),
        decl_span: pattern.span,
        scope: format!("scope:source:block:{}:{}", scope.start, scope.end),
        hir: pattern.hir.clone(),
        is_param: false,
        is_mut: false,
        ty: pattern.ty.clone().unwrap(),
    }
}

fn checked_local_alias() -> HirFnFacts {
    let source = "pub fn caller(value:u32) { let saved=value; leaf(saved); }";
    let mut facts = fixture(source);
    let function: syn::ItemFn = syn::parse_str(source).unwrap();
    let syn::Stmt::Local(local) = &function.block.stmts[0] else {
        unreachable!()
    };
    let declaration = span(source, &local.pat);
    let mut pattern = node(
        source,
        NodeShape {
            kind: "pat_binding",
            token: Some("u32"),
        },
        declaration,
        300,
    );
    pattern.binding = Some("local-saved".into());
    pattern.binding_mut = Some(false);
    facts
        .bindings
        .push(local_alias_binding(&pattern, facts.body.span));

    let argument = &mut facts.body.children[0].children[0];
    argument.ty = pattern.ty.clone();
    argument.res = Some(crate::PathRes {
        kind: "local".into(),
        target: Some(pattern.hir.clone()),
        description: String::new(),
    });
    let mut initializer = node(
        source,
        NodeShape {
            kind: "expr_path",
            token: Some("u32"),
        },
        span(source, &*local.init.as_ref().unwrap().expr),
        301,
    );
    initializer.res = Some(crate::PathRes {
        kind: "local".into(),
        target: Some(facts.signature.params[0].pat.hir.clone()),
        description: String::new(),
    });
    facts.body.children[0]
        .dispatch
        .as_mut()
        .unwrap()
        .operand_tys[0] = "u32".into();
    facts.body.children.extend([pattern, initializer]);
    facts
}

#[test]
fn checked_local_alias_and_literal_are_distinct_from_unchanged_formals() {
    for facts in [
        checked_local_alias(),
        fixture("pub fn caller() { leaf(3u32); }"),
    ] {
        let observed = inspect_calls(&facts).unwrap();
        let argument = &observed.calls[0].arguments[0];
        assert!(matches!(
            argument.value,
            Availability::Known(ArgumentValue::OwnedU32)
        ));
        assert_eq!(argument.formal, Availability::Known(None));
        assert_eq!(argument.unchanged_formal, Availability::Known(None));
    }
}

fn text_span(source: &str, text: &str) -> ByteSpan {
    let start = source.find(text).unwrap() as u32;
    ByteSpan {
        start,
        end: start + text.len() as u32,
    }
}

fn native_read(facts: &HirFnFacts, formal: usize, range: ByteSpan, local: u32) -> HirNode {
    let mut read = node(
        &facts.source.bytes,
        NodeShape {
            kind: "expr_path",
            token: Some("u32"),
        },
        range,
        local,
    );
    read.res = Some(crate::PathRes {
        kind: "local".into(),
        target: Some(facts.signature.params[formal].pat.hir.clone()),
        description: String::new(),
    });
    read
}

fn repeated_read_fixture() -> HirFnFacts {
    let source = "pub fn caller(left:u32,right:u32) { left+left; }";
    let mut facts = fixture(source);
    let range = text_span(source, "left+left");
    let left = native_read(
        &facts,
        0,
        ByteSpan {
            start: range.start,
            end: range.start + 4,
        },
        501,
    );
    let right = native_read(
        &facts,
        0,
        ByteSpan {
            start: range.end - 4,
            end: range.end,
        },
        502,
    );
    let mut binary = node(
        source,
        NodeShape {
            kind: "expr_binary",
            token: Some("u32"),
        },
        range,
        500,
    );
    binary.children = vec![left, right];
    binary.dispatch = Some(crate::Dispatch {
        kind: "builtin".into(),
        op: "+".into(),
        operand_tys: vec!["u32".into(), "u32".into()],
        result_ty: "u32".into(),
    });
    facts.body.children = vec![binary];
    facts
}

#[test]
fn formal_use_inventory_retains_unused_declarations_and_repeated_reads() {
    let facts = repeated_read_fixture();
    let observed = inspect_calls_with_uses(&facts).unwrap();
    assert_eq!(observed.uses.availability, Availability::Known(()));
    assert_eq!(observed.uses.coverage, Coverage::Complete);
    assert_eq!(observed.uses.formals.len(), 2);
    assert_eq!(observed.uses.occurrences.len(), 2);
    assert!(
        observed
            .uses
            .occurrences
            .iter()
            .all(|use_| use_.formal == 0 && use_.role == UseRole::Read)
    );
    assert_eq!(observed.uses.formals[1].ordinal, 1);
    assert_ne!(
        observed.uses.occurrences[0].native,
        observed.uses.occurrences[1].native
    );
}

fn owned_write(facts: &HirFnFacts, text: &str, local: u32) -> HirNode {
    let source = &facts.source.bytes;
    let range = text_span(source, text);
    let left = native_read(
        facts,
        0,
        ByteSpan {
            start: range.start,
            end: range.start + 5,
        },
        local + 1,
    );
    let right = node(
        source,
        NodeShape {
            kind: "expr_lit",
            token: Some("u32"),
        },
        ByteSpan {
            start: range.end - 1,
            end: range.end,
        },
        local + 2,
    );
    let compound = text.contains("+=");
    let mut assignment = node(
        source,
        NodeShape {
            kind: if compound {
                "expr_assign_op"
            } else {
                "expr_assign"
            },
            token: Some("unit"),
        },
        range,
        local,
    );
    assignment.children = vec![left, right];
    if compound {
        assignment.dispatch = Some(crate::Dispatch {
            kind: "builtin".into(),
            op: "+=".into(),
            operand_tys: vec!["u32".into(), "u32".into()],
            result_ty: "unit".into(),
        });
    }
    assignment
}

#[test]
fn formal_use_inventory_records_mutable_targets_once_without_changing_mutations() {
    let mut facts = fixture("pub fn caller(mut value:u32) { value=1; value+=2; }");
    facts.body.children = vec![
        owned_write(&facts, "value=1", 500),
        owned_write(&facts, "value+=2", 600),
    ];
    let before = inspect_calls(&facts).unwrap();
    let after = inspect_calls_with_uses(&facts).unwrap();
    assert_eq!(
        serde_json::to_value(before.direct).unwrap(),
        serde_json::to_value(after.direct).unwrap()
    );
    assert_eq!(after.uses.coverage, Coverage::Complete);
    assert!(after.uses.formals[0].mutable);
    assert_eq!(
        after
            .uses
            .occurrences
            .iter()
            .map(|use_| use_.role)
            .collect::<Vec<_>>(),
        vec![UseRole::Write, UseRole::ReadWrite]
    );
    let mut changed = facts.clone();
    changed.body.children[0].children[0]
        .res
        .as_mut()
        .unwrap()
        .target
        .as_mut()
        .unwrap()
        .local = 999;
    assert!(inspect_calls_with_uses(&changed).is_err());
    facts.body.children.clear();
    assert_eq!(
        inspect_calls_with_uses(&facts).unwrap().uses.coverage,
        Coverage::Incomplete
    );
}

#[test]
fn formal_use_inventory_links_exact_call_positions_and_keeps_witnessed_prefixes() {
    let facts = fixture("pub fn caller(left:u32,right:u32) { leaf(right,left); noop!(); }");
    let observed = inspect_calls_with_uses(&facts).unwrap();
    assert_eq!(observed.uses.coverage, Coverage::Incomplete);
    assert_eq!(observed.uses.occurrences.len(), 2);
    assert_eq!(observed.uses.occurrences[0].formal, 1);
    assert_eq!(
        observed.uses.occurrences[0].role,
        UseRole::CallArgument {
            call_index: 0,
            argument_index: 0,
            unchanged: true
        }
    );
    assert_eq!(observed.uses.occurrences[1].formal, 0);
    let facts = checked_local_alias();
    let observed = inspect_calls_with_uses(&facts).unwrap();
    assert_eq!(observed.uses.coverage, Coverage::Complete);
    assert_eq!(observed.uses.occurrences.len(), 1);
    assert_eq!(observed.uses.occurrences[0].role, UseRole::Read);
}

fn wrapped_use(source: &str, text: &str, kind: &'static str) -> HirFnFacts {
    let mut facts = fixture(source);
    let wrapper_span = text_span(source, text);
    let read_span = ByteSpan {
        start: wrapper_span.end - 5,
        end: wrapper_span.end,
    };
    let read = native_read(&facts, 0, read_span, 501);
    let mut wrapper = node(source, NodeShape { kind, token: None }, wrapper_span, 500);
    wrapper.children = vec![read];
    facts.body.children = vec![wrapper];
    if source.contains("->u32") {
        facts.signature.return_ty.normalized = Some("u32".into());
    }
    facts
}

#[test]
fn formal_use_inventory_checks_return_and_borrow_children_independently_of_effects() {
    for (facts, role) in [
        (
            wrapped_use(
                "pub fn caller(value:u32) { &value; }",
                "&value",
                "expr_addr_of",
            ),
            UseRole::SharedBorrow,
        ),
        (
            wrapped_use(
                "pub fn caller(mut value:u32) { &mut value; }",
                "&mut value",
                "expr_addr_of",
            ),
            UseRole::MutableBorrow,
        ),
        (
            wrapped_use(
                "pub fn caller(value:u32)->u32 { return value; }",
                "return value",
                "expr_ret",
            ),
            UseRole::Read,
        ),
    ] {
        let observed = inspect_calls_with_uses(&facts).unwrap();
        assert_eq!(observed.direct.coverage, Coverage::Incomplete);
        assert_eq!(observed.uses.coverage, Coverage::Complete);
        assert_eq!(observed.uses.occurrences[0].role, role);
        let mut changed = facts;
        changed.body.children[0].children.clear();
        assert!(inspect_calls_with_uses(&changed).is_err());
    }
}

#[test]
fn formal_use_inventory_never_treats_missing_or_unrequested_evidence_as_zero_clean() {
    let facts = fixture("pub fn caller(value:u32) { leaf(value); }");
    let unrequested = inspect_calls(&facts).unwrap();
    assert_eq!(unrequested.uses.coverage, Coverage::Incomplete);
    assert!(matches!(
        unrequested.uses.availability,
        Availability::Unknown { .. }
    ));
    let mut changed = facts;
    changed.body.children[0].children[0].res = None;
    let observed = inspect_calls_with_uses(&changed).unwrap();
    assert_eq!(observed.uses.coverage, Coverage::Incomplete);
    assert!(observed.uses.occurrences.is_empty());
    let changed = fixture("pub fn caller(value:u32) { leaf({value}); }");
    assert_eq!(
        inspect_calls_with_uses(&changed).unwrap().uses.coverage,
        Coverage::Incomplete
    );
    let changed = fixture("pub fn caller(value:&mut u32) { leaf(value); }");
    assert_eq!(
        inspect_calls_with_uses(&changed).unwrap().uses.coverage,
        Coverage::Incomplete
    );
}

#[test]
fn formal_use_syntax_exhaustion_and_duplicate_native_occurrences_never_complete() {
    let source = format!("pub fn caller(value:u32) {{{}}}", ";".repeat(140));
    let facts = fixture(&source);
    let observed = inspect_calls_with_uses(&facts).unwrap();
    assert_eq!(observed.uses.execution, Execution::BudgetExhausted);
    assert_eq!(observed.uses.coverage, Coverage::Incomplete);
    let mut facts = fixture("pub fn caller(value:u32) { leaf(value); }");
    facts.body.children.push(facts.body.children[0].clone());
    assert!(inspect_calls_with_uses(&facts).is_err());
}

mod conditional;
