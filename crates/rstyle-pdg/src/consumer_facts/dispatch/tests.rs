use super::*;
use crate::{
    ByteSpan, HirFnFacts, SourceText,
    nominal_boundary::{self, census},
};
use std::sync::Arc;

fn fixture(source: &str, name: &str) -> HirFnFacts {
    let mut facts: HirFnFacts = serde_json::from_str(include_str!(
        "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    let parsed = syn::parse_file(source).unwrap();
    let function = parsed
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == name => Some(function.clone()),
            syn::Item::Impl(item) => item.items.iter().find_map(|member| match member {
                syn::ImplItem::Fn(function) if function.sig.ident == name => Some(syn::ItemFn {
                    attrs: function.attrs.clone(),
                    vis: function.vis.clone(),
                    sig: function.sig.clone(),
                    block: Box::new(function.block.clone()),
                }),
                _ => None,
            }),
            _ => None,
        })
        .unwrap();
    let range = shapes::function_range(source, &function);
    facts.source.bytes =
        SourceText::from(crate::source_evidence::byte_range(source, range).unwrap());
    facts.source.digest = facts.source.bytes.checksum().into();
    facts.fn_span = range;
    facts.signature.sig_span = ByteSpan {
        start: 0,
        end: shapes::function_header(source, &function).end - range.start,
    };
    facts.signature.has_self = function.sig.receiver().is_some();
    facts.signature.receiver = function.sig.receiver().map(|_| crate::ReceiverFact {
        span: ByteSpan { start: 0, end: 0 },
        snippet: "self".into(),
    });
    facts.signature.params = (0..function.sig.inputs.len())
        .map(|position| crate::ParamFact {
            symbol: format!("p{position}"),
            position: position as u32,
            name_span: ByteSpan { start: 0, end: 0 },
            ty_span: ByteSpan { start: 0, end: 0 },
            ty: facts.signature.return_ty.clone(),
            pat: facts.body.clone(),
        })
        .collect();
    facts.source_diagnostics.clear();
    own(&mut facts, source);
    facts
}

fn own(facts: &mut HirFnFacts, source: &str) {
    let mut context = nominal_boundary::tests::context();
    context.package.manifest_digest = context.package.manifest_bytes.checksum().into();
    let path = context.root_source.clone();
    facts.build.frontend_options = Default::default();
    facts.build.frontend_options.insert(
        nominal_boundary::CONTEXT_KEY.into(),
        serde_json::to_value(&context).unwrap(),
    );
    facts
        .build
        .frontend_options
        .insert("source_file".into(), serde_json::json!(path));
    let bytes: crate::SourceSnapshot = source.into();
    let digest = bytes.checksum().into();
    facts
        .build
        .frontend_options
        .insert_data(Arc::new(census::Census {
            identity: census::IDENTITY.into(),
            context,
            files: vec![census::UnitFile {
                path,
                bytes,
                digest,
            }],
            expansions: vec![],
            documentation: vec![],
            external_imports: vec![],
        }));
}

#[test]
fn independently_authored_dispatch_labels() {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../eval/clones-capability/enum-dispatch-cases.json"
    ))
    .unwrap();
    for case in cases["cases"].as_array().unwrap() {
        let facts = fixture(
            case["source"].as_str().unwrap(),
            case["function"].as_str().unwrap(),
        );
        let index = Index::from_facts(&[&facts]).unwrap();
        let assessment = index.evaluate(&facts).unwrap();
        let observed = match assessment.availability {
            super::super::Availability::Known(Some(_)) => "finding",
            super::super::Availability::Known(None) => "evaluated_no_finding",
            _ => "not_evaluated",
        };
        assert_eq!(
            observed,
            case["expected"].as_str().unwrap(),
            "{}",
            case["id"]
        );
    }
}

fn positive_source() -> &'static str {
    "pub struct Alpha; pub struct Beta; impl Alpha { pub fn measure(&self)->u32 {1} } impl Beta { pub fn amount(&self)->u32 {let f=||2; f()} } pub enum Item { A(Alpha), B(Beta) } pub fn dispatch(item:&Item)->u32 {match item {Item::A(a)=>a.measure(),Item::B(b)=>b.amount()}}"
}

#[test]
fn positive_retains_original_headers_arms_and_signature_without_body_admission() {
    let source = positive_source();
    let facts = fixture(source, "dispatch");
    let index = Index::from_facts(&[&facts]).unwrap();
    let super::super::Availability::Known(Some(candidate)) =
        index.evaluate(&facts).unwrap().availability
    else {
        panic!("expected dispatch")
    };
    assert_eq!(candidate.enum_name, "Item");
    assert_eq!(candidate.signature.return_type, "u32");
    assert!(candidate.signature.shared_self);
    assert_eq!(candidate.signature.ordinary_parameters, 0);
    assert_eq!(
        crate::source_evidence::byte_range(source, candidate.caller_header).unwrap(),
        "pub fn dispatch(item:&Item)->u32"
    );
    assert_eq!(
        crate::source_evidence::byte_range(source, candidate.arms[1].callee_header).unwrap(),
        "pub fn amount(&self)->u32"
    );
    assert!(
        crate::source_evidence::byte_range(source, candidate.arms[0].arm_span)
            .unwrap()
            .starts_with("Item::A(a)=>a.measure()")
    );
}

#[test]
fn ordinary_opaque_method_is_evaluated_without_inspecting_its_body() {
    let source = positive_source();
    let facts = fixture(source, "amount");
    let index = Index::from_facts(&[&facts]).unwrap();
    assert!(matches!(
        index.evaluate(&facts).unwrap().availability,
        super::super::Availability::Known(None)
    ));
}

#[test]
fn missing_ownership_is_unknown_and_wrong_bytes_are_hard_errors() {
    let mut facts = fixture(positive_source(), "dispatch");
    facts.build.frontend_options = Default::default();
    assert!(matches!(
        Index::from_facts(&[&facts])
            .unwrap()
            .evaluate(&facts)
            .unwrap()
            .availability,
        super::super::Availability::Unknown { .. }
    ));
    let mut facts = fixture(positive_source(), "dispatch");
    let index = Index::from_facts(&[&facts]).unwrap();
    facts.source.bytes = SourceText::from("fn changed(){}");
    assert!(index.evaluate(&facts).is_err());
}

#[test]
fn primitive_shadowing_and_import_aliases_are_unsupported() {
    for prefix in [
        "struct u32;",
        "enum u32 { X }",
        "trait u32 {}",
        "type Alias=Alpha;",
        "use super::*;",
    ] {
        let source = format!("{prefix} {}", positive_source());
        let facts = fixture(&source, "dispatch");
        assert!(matches!(
            Index::from_facts(&[&facts])
                .unwrap()
                .evaluate(&facts)
                .unwrap()
                .availability,
            super::super::Availability::Unsupported { .. }
        ));
    }
}

#[test]
fn parser_budget_is_an_explicit_execution_marker() {
    let source = format!("/* {} */ {}", "(".repeat(257), positive_source());
    let facts = fixture(&source, "dispatch");
    let assessment = Index::from_facts(&[&facts])
        .unwrap()
        .evaluate(&facts)
        .unwrap();
    assert!(assessment.budget_exhausted);
    assert!(matches!(
        assessment.availability,
        super::super::Availability::Unsupported { .. }
    ));
}

fn assert_unsupported(source: &str) {
    let facts = fixture(source, "dispatch");
    let assessment = Index::from_facts(&[&facts])
        .unwrap()
        .evaluate(&facts)
        .unwrap();
    assert!(
        matches!(
            assessment.availability,
            super::super::Availability::Unsupported { .. }
        ),
        "{source}"
    );
}

#[test]
fn namespace_closure_covers_nonlocal_and_nested_declarations() {
    for suffix in [
        "mod extra { impl super::Alpha { pub fn measure(&self)->u32 {3} } }",
        "mod extra;",
        "mod extra { use super::Alpha; impl Alpha { pub fn measure(&self)->u32 {3} } }",
        "fn install() { impl Alpha { pub fn measure(&self)->u32 {3} } }",
        "fn install() { generate_impl!(); }",
        "#[derive_impl] fn install() {}",
        "const INSTALL: () = { impl Alpha { pub fn measure(&self)->u32 {3} } };",
    ] {
        assert_unsupported(&format!("{} {suffix}", positive_source()));
    }
}

#[test]
fn attributed_ordinary_method_has_driver_style_span_and_no_dispatch_witness() {
    let source = positive_source().replace("pub fn amount", "#[opaque] pub fn amount");
    let facts = fixture(&source, "amount");
    assert!(facts.source.bytes.starts_with("pub fn amount"));
    assert!(matches!(
        Index::from_facts(&[&facts])
            .unwrap()
            .evaluate(&facts)
            .unwrap()
            .availability,
        super::super::Availability::Known(None)
    ));
    assert_unsupported(&source);
}

#[test]
fn same_free_callee_does_not_hide_opaque_argument_work() {
    let source = positive_source()
        .replace("a.measure()", "common({ let f=||1; f() })")
        .replace("b.amount()", "common(b.value)");
    assert_unsupported(&source);
}

#[test]
fn original_source_and_declaration_limits_are_explicit() {
    let exact = format!("/*{}*/", " ".repeat(256 * 1024 - 4));
    assert!(shapes::parse(&exact).is_ok());
    for source in [
        format!("{exact} "),
        "struct Unit;".repeat(4097),
        format!("{}{}", "mod nested {".repeat(33), "}".repeat(33)),
        format!("trait Many {{{}}}", "fn member(&self);".repeat(4097)),
    ] {
        let failure = match shapes::parse(&source) {
            Ok(_) => panic!("expected budget failure"),
            Err(failure) => failure,
        };
        assert!(failure.budget_exhausted, "{}", failure.reason);
    }
}

#[test]
fn shared_source_queries_keep_exact_ranges_and_reject_an_unindexed_owner() {
    let dispatcher = fixture(positive_source(), "dispatch");
    let mut method = fixture(positive_source(), "amount");
    method.build.frontend_options = dispatcher.build.frontend_options.clone();
    let index = Index::from_facts(&[&dispatcher, &method]).unwrap();
    assert!(matches!(
        index.evaluate(&dispatcher).unwrap().availability,
        super::super::Availability::Known(Some(_))
    ));
    assert!(matches!(
        index.evaluate(&method).unwrap().availability,
        super::super::Availability::Known(None)
    ));
    let separate_owner = fixture(positive_source(), "dispatch");
    assert!(index.evaluate(&separate_owner).is_err());
}

#[test]
fn competing_mutable_inherent_and_shared_trait_receivers_are_unavailable() {
    let source = "pub struct Alpha; pub struct Beta; pub trait Metric { fn measure(&self)->u32; } impl Metric for Alpha { fn measure(&self)->u32 {1} } impl Alpha { pub fn measure(&mut self)->u32 {2} } impl Beta { pub fn measure(&self)->u32 {3} } pub enum Item { A(Alpha), B(Beta) } pub fn dispatch(item:&Item)->u32 { match item { Item::A(a)=>a.measure(), Item::B(b)=>b.measure() } }";
    let facts = fixture(source, "dispatch");
    let assessment = Index::from_facts(&[&facts])
        .unwrap()
        .evaluate(&facts)
        .unwrap();
    assert!(
        matches!(assessment.availability, super::super::Availability::Unsupported { reason } if reason.contains("Competing"))
    );
}

fn wrapper_source() -> &'static str {
    "struct Alpha; impl Alpha { fn used(&self)->u32 {let opaque=||1; opaque()} fn limit(&self)->u32 {2} } fn alpha(x:&Alpha)->(u32,u32){(x.used(),x.limit())}"
}

#[test]
fn capability_wrapper_retains_order_and_original_headers_with_opaque_bodies() {
    let source = wrapper_source();
    let facts = fixture(source, "alpha");
    let index = Index::from_facts(&[&facts]).unwrap();
    let super::super::Availability::Known(Some(wrapper)) = index
        .evaluate_capability_wrapper(&facts)
        .unwrap()
        .availability
    else {
        panic!("expected wrapper")
    };
    assert_eq!(wrapper.owner.name, "Alpha");
    assert_eq!(wrapper.owner_name, "Alpha");
    assert_eq!(
        wrapper
            .methods
            .iter()
            .map(|method| method.name.as_str())
            .collect::<Vec<_>>(),
        ["used", "limit"]
    );
    assert_eq!(
        crate::source_evidence::byte_range(source, wrapper.methods[0].call_span).unwrap(),
        "x.used()"
    );
    assert_eq!(
        crate::source_evidence::byte_range(source, wrapper.methods[1].callee_header).unwrap(),
        "fn limit(&self)->u32"
    );
    assert_eq!(
        crate::source_evidence::byte_range(source, wrapper.caller_header).unwrap(),
        "fn alpha(x:&Alpha)->(u32,u32)"
    );
    assert!(matches!(
        index.evaluate(&facts).unwrap().availability,
        super::super::Availability::Known(None)
    ));
}

#[test]
fn capability_wrappers_require_two_distinct_direct_receiver_calls() {
    for body in [
        "(x.used(),x.used())",
        "(x.used(),other.limit())",
        "(x.used(),x.limit(),x.used())",
    ] {
        let source = wrapper_source().replace("(x.used(),x.limit())", body);
        let facts = fixture(&source, "alpha");
        assert!(
            matches!(
                Index::from_facts(&[&facts])
                    .unwrap()
                    .evaluate_capability_wrapper(&facts)
                    .unwrap()
                    .availability,
                super::super::Availability::Known(None)
            ),
            "{body}"
        );
    }
}

#[test]
fn capability_wrapper_reuses_closed_namespace_and_method_selection() {
    for source in [
        format!("use extra::*; {}", wrapper_source()),
        wrapper_source().replace("struct Alpha;", "type Alpha=Other;"),
        wrapper_source().replace("fn limit(&self)", "fn limit<T>(&self)"),
        wrapper_source().replace("x.limit()", "x.limit::<u8>()"),
        wrapper_source().replace("x.limit()", "{let saved=x.limit(); saved}"),
        wrapper_source().replace("x.limit()", "x.limit(hidden())"),
        wrapper_source().replace("(x.used(),x.limit())", "{let y=x; (y.used(),y.limit())}"),
        wrapper_source().replace("x:&Alpha", "x:&mut Alpha"),
        wrapper_source().replace("->(u32,u32)", "->(u32,u16)"),
    ] {
        let facts = fixture(&source, "alpha");
        assert!(
            matches!(
                Index::from_facts(&[&facts])
                    .unwrap()
                    .evaluate_capability_wrapper(&facts)
                    .unwrap()
                    .availability,
                super::super::Availability::Unsupported { .. }
            ),
            "{source}"
        );
    }
    let source = wrapper_source().replace("fn limit(&self)", "fn limit(&mut self)");
    let facts = fixture(&source, "alpha");
    assert!(matches!(
        Index::from_facts(&[&facts])
            .unwrap()
            .evaluate_capability_wrapper(&facts)
            .unwrap()
            .availability,
        super::super::Availability::Known(None)
    ));
}

#[test]
fn capability_ownership_errors_and_budget_match_dispatch_boundary() {
    let mut facts = fixture(wrapper_source(), "alpha");
    let index = Index::from_facts(&[&facts]).unwrap();
    facts.signature.sig_span.end += 1;
    assert!(index.evaluate_capability_wrapper(&facts).is_err());
    let mut facts = fixture(wrapper_source(), "alpha");
    facts.build.frontend_options = Default::default();
    assert!(matches!(
        Index::from_facts(&[&facts])
            .unwrap()
            .evaluate_capability_wrapper(&facts)
            .unwrap()
            .availability,
        super::super::Availability::Unknown { .. }
    ));
    let source = format!("/* {} */ {}", "(".repeat(257), wrapper_source());
    let facts = fixture(&source, "alpha");
    let assessment = Index::from_facts(&[&facts])
        .unwrap()
        .evaluate_capability_wrapper(&facts)
        .unwrap();
    assert!(assessment.budget_exhausted);
    assert!(matches!(
        assessment.availability,
        super::super::Availability::Unsupported { .. }
    ));
}

#[test]
fn capability_owner_slots_distinguish_equal_names_in_independent_sources() {
    let first = fixture(wrapper_source(), "alpha");
    let second = fixture(wrapper_source(), "alpha");
    let index = Index::from_facts(&[&first, &second]).unwrap();
    let super::super::Availability::Known(Some(left)) = index
        .evaluate_capability_wrapper(&first)
        .unwrap()
        .availability
    else {
        panic!("left")
    };
    let super::super::Availability::Known(Some(right)) = index
        .evaluate_capability_wrapper(&second)
        .unwrap()
        .availability
    else {
        panic!("right")
    };
    assert_ne!(left.owner, right.owner);
    assert_eq!(left.owner.name, right.owner.name);
}

#[test]
fn opaque_methods_and_nontuple_free_functions_are_not_wrapper_candidates() {
    let source = format!(
        "{} fn opaque()->u32 {{let value=3; value}}",
        wrapper_source()
    );
    let wrapper = fixture(&source, "alpha");
    let mut method = fixture(&source, "used");
    let mut ordinary = fixture(&source, "opaque");
    method.build.frontend_options = wrapper.build.frontend_options.clone();
    ordinary.build.frontend_options = wrapper.build.frontend_options.clone();
    let index = Index::from_facts(&[&wrapper, &method, &ordinary]).unwrap();
    assert!(matches!(
        index
            .evaluate_capability_wrapper(&wrapper)
            .unwrap()
            .availability,
        super::super::Availability::Known(Some(_))
    ));
    for facts in [&method, &ordinary] {
        assert!(matches!(
            index
                .evaluate_capability_wrapper(facts)
                .unwrap()
                .availability,
            super::super::Availability::Known(None)
        ));
    }
    ordinary.source.bytes = SourceText::from("corrupt original");
    assert!(index.evaluate_capability_wrapper(&ordinary).is_err());
}

fn external_facts(source: &str, name: &str) -> HirFnFacts {
    let mut facts = fixture(source, name);
    let original = facts
        .build
        .frontend_options
        .data::<Arc<census::Census>>()
        .unwrap()
        .into_owned();
    let mut census = (*original).clone();
    let path = census
        .context
        .root_source
        .parent()
        .unwrap()
        .join("worker.rs");
    census.files[0].path = path.clone();
    let bytes: crate::SourceSnapshot = "mod worker;".into();
    census.files.push(census::UnitFile {
        path: census.context.root_source.clone(),
        digest: bytes.checksum().into(),
        bytes,
    });

    facts.build.frontend_options.insert_data(Arc::new(census));
    facts
        .build
        .frontend_options
        .insert("source_file".into(), serde_json::json!(path));
    facts
}

#[test]
fn external_review_location_does_not_extend_dispatch_or_capability_permissions() {
    let facts = external_facts(positive_source(), "dispatch");
    let index = Index::from_facts(&[&facts]).unwrap();
    assert_eq!(
        index.review_location(&facts).unwrap(),
        ("crate::worker::dispatch".into(), "free-function")
    );
    assert!(matches!(
        index.evaluate(&facts).unwrap().availability,
        super::super::Availability::Unsupported { .. }
    ));
    assert!(matches!(
        index
            .evaluate_capability_wrapper(&facts)
            .unwrap()
            .availability,
        super::super::Availability::Unsupported { .. }
    ));
}

#[test]
fn external_location_cache_checks_current_context_and_original_bytes() {
    let mut facts = external_facts("fn selected(value:u32)->u32{value}", "selected");
    let index = Index::from_facts(&[&facts]).unwrap();
    assert!(index.review_location(&facts).is_ok());
    facts.build.cfgs.push("changed_cfg".into());
    assert!(index.review_location(&facts).is_err());
    facts.build.cfgs.pop();
    facts.signature.sig_span.end += 1;
    assert!(index.review_location(&facts).is_err());
    facts.signature.sig_span.end -= 1;
    facts.source.bytes = SourceText::from("different original bytes");
    assert!(index.review_location(&facts).is_err());
}
