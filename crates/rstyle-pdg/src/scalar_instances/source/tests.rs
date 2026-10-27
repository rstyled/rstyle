//! Fixed compiler fixture plus independent adversarial mutations; no producer equality oracle.
use super::*;
use serde::Deserialize;

#[derive(Deserialize)]
struct Sample {
    name: String,
    fn_span: crate::ByteSpan,
    source: crate::SourceFacts,
    signature: crate::SignatureFacts,
    bindings: Vec<BindingFact>,
    body: HirNode,
}

fn fixture(scalar: &str) -> (HirFnFacts, HirFnFacts) {
    let mut original: HirFnFacts = serde_json::from_str(include_str!(
        "../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    let sample: Sample = serde_json::from_str(ORIGINAL).unwrap();
    original.name = sample.name;
    original.fn_span = sample.fn_span;
    original.source = sample.source;
    original.signature = sample.signature;
    original.bindings = sample.bindings;
    original.body = sample.body;
    let mut projected = original.clone();
    for parameter in &mut projected.signature.params {
        parameter.ty.normalized = Some(scalar.into());
        parameter.ty.observed = scalar.into();
        parameter.pat.ty.as_mut().unwrap().normalized = Some(scalar.into());
    }
    projected.signature.return_ty.normalized = Some(scalar.into());
    for binding in &mut projected.bindings {
        binding.ty.normalized = Some(scalar.into());
    }
    project_nodes(&mut projected.body, scalar);
    (original, projected)
}

fn project_nodes(node: &mut HirNode, scalar: &str) {
    // Independently fixed concrete-result sites from the original compiler fixture.
    if SCALAR_SITES.contains(&node.hir.local) {
        node.ty.as_mut().unwrap().normalized = Some(scalar.into());
    }
    if let Some(dispatch) = &mut node.dispatch {
        dispatch.operand_tys = vec![scalar.into(), scalar.into()];
    }
    for child in &mut node.children {
        project_nodes(child, scalar);
    }
}

#[test]
fn original_clamp_min_substitution_retains_generic_source_and_ignores_displays() {
    for scalar in ["i32", "u32"] {
        let (mut original, projected) = fixture(scalar);
        assert!(verify(&original, &projected, scalar).is_ok());
        original.signature.params[0].ty.observed = "display text is not a type proof".into();
        assert!(verify(&original, &projected, scalar).is_ok());
        assert_eq!(projected.signature.generics, original.signature.generics);
    }
}

#[test]
fn wrong_signature_binding_and_branch_types_reject_independently() {
    let (original, projected) = fixture("i32");
    let mut changed = projected.clone();
    changed.signature.params[0].ty.normalized = Some("u32".into());
    assert!(verify(&original, &changed, "i32").is_err());
    changed = projected.clone();
    changed.bindings[0].ty.normalized = Some("bool".into());
    assert!(verify(&original, &changed, "i32").is_err());
    changed = projected.clone();
    changed.body.children[1].children[1]
        .ty
        .as_mut()
        .unwrap()
        .normalized = Some("bool".into());
    assert!(verify(&original, &changed, "i32").is_err());
    assert!(verify(&original, &projected, "f32").is_err());
}

#[test]
fn missing_assert_unknown_call_and_suffix_never_certify_a_supported_prefix() {
    let (original, projected) = fixture("i32");
    let mut changed = projected.clone();
    changed.body.children.remove(0);
    assert!(verify(&original, &changed, "i32").is_err());
    let mut missing = original.clone();
    missing.body.children.remove(0);
    assert!(verify(&missing, &changed, "i32").is_err());
    let mut hidden = original.clone();
    hidden.body.children[1].children[1].children[0].kind = "expr_call".into();
    changed = projected.clone();
    changed.body.children[1].children[1].children[0].kind = "expr_call".into();
    assert!(verify(&hidden, &changed, "i32").is_err());
    changed = projected.clone();
    changed.body.children.push(changed.body.children[0].clone());
    hidden = original.clone();
    hidden.body.children.push(hidden.body.children[0].clone());
    assert!(verify(&hidden, &changed, "i32").is_err());
}

#[test]
fn resealed_local_resolution_dispatch_and_build_transplants_reject() {
    let (mut original, mut projected) = fixture("i32");
    projected.body.children[1].children[0]
        .dispatch
        .as_mut()
        .unwrap()
        .kind = "builtin".into();
    assert!(verify(&original, &projected, "i32").is_err());
    (original, projected) = fixture("i32");
    for facts in [&mut original, &mut projected] {
        facts.body.children[1].children[1].children[0]
            .res
            .as_mut()
            .unwrap()
            .target
            .as_mut()
            .unwrap()
            .local = 999;
    }
    assert!(verify(&original, &projected, "i32").is_err());
    (original, projected) = fixture("i32");
    projected.build.cfgs.push("unrelated".into());
    assert!(verify(&original, &projected, "i32").is_err());
}

#[test]
fn indexed_offsets_count_unicode_columns_and_preserve_bom_crlf_boundaries() {
    let offsets = SpanIndex::new("\u{feff}αβ\r\n値🦀\nZ");
    for (line, expected) in [
        (1, vec![3, 5, 7, 8]),
        (2, vec![9, 12, 16]),
        (3, vec![17, 18]),
    ] {
        for (column, byte) in expected.into_iter().enumerate() {
            assert_eq!(
                offsets
                    .offset(proc_macro2::LineColumn { line, column })
                    .unwrap(),
                byte
            );
        }
    }
    for location in [
        proc_macro2::LineColumn { line: 0, column: 0 },
        proc_macro2::LineColumn { line: 4, column: 0 },
        proc_macro2::LineColumn { line: 2, column: 3 },
    ] {
        assert!(offsets.offset(location).is_err());
    }
}

#[test]
fn indexed_parser_spans_select_exact_multiline_unicode_bytes_with_or_without_bom() {
    for prefix in ["", "\u{feff}"] {
        for newline in ["\n", "\r\n"] {
            let source = format!(
                "{prefix}// αβ{newline}pub fn café(値: bool) -> bool {{{newline}    // 🦀{newline}    値{newline}}}"
            );
            let syntax = syn::parse_file(&source).unwrap();
            let syn::Item::Fn(function) = &syntax.items[0] else {
                unreachable!()
            };
            let offsets = SpanIndex::new(&source);
            let function_span = offsets.span(function.span()).unwrap();
            assert_eq!(function_span.start as usize, source.find("pub fn").unwrap());
            assert_eq!(function_span.end as usize, source.len());
            let name = offsets.span(function.sig.ident.span()).unwrap();
            assert_eq!(name.start as usize, source.find("café").unwrap());
            assert_eq!(name.end - name.start, 5);
            let syn::Stmt::Expr(value, None) = &function.block.stmts[0] else {
                unreachable!()
            };
            let value = offsets.span(value.span()).unwrap();
            assert_eq!(value.start as usize, source.rfind('値').unwrap());
            assert_eq!(value.end - value.start, 3);
        }
    }
}

const SCALAR_SITES: &[u32] = &[49, 12, 14, 35, 37, 39, 44, 42, 45, 47];

const ORIGINAL: &str = r###"{"name":"clamp_min","fn_span":{"start":16734,"end":16912},"source":{"bytes":"pub fn clamp_min<T: PartialOrd>(input: T, min: T) -> T {\n    debug_assert!(min == min, \"min must not be NAN\");\n    if input < min {\n        min\n    } else {\n        input\n    }\n}","digest":"sha256:c7bb6aac5295cbf37afb28864746471278390e2ec9cc578551ac1f90c8ddc54b"},"signature":{"sig_span":{"start":0,"end":54},"params":[{"symbol":"s:740:2","position":0,"name_span":{"start":32,"end":37},"ty_span":{"start":39,"end":40},"ty":{"normalized":null,"observed":"T","spelling":"T"},"pat":{"hir":{"krate":0,"owner":740,"local":2},"kind":"pat_binding","span":{"start":32,"end":37},"snippet":"input","ty":{"normalized":null,"observed":"T","spelling":""},"res":null,"dispatch":null,"expansion":null,"binding":"s:740:2","binding_mut":false,"lit":null,"param_index":0,"is_unsafe_block":false,"children":[]}},{"symbol":"s:740:4","position":1,"name_span":{"start":42,"end":45},"ty_span":{"start":47,"end":48},"ty":{"normalized":null,"observed":"T","spelling":"T"},"pat":{"hir":{"krate":0,"owner":740,"local":4},"kind":"pat_binding","span":{"start":42,"end":45},"snippet":"min","ty":{"normalized":null,"observed":"T","spelling":""},"res":null,"dispatch":null,"expansion":null,"binding":"s:740:4","binding_mut":false,"lit":null,"param_index":1,"is_unsafe_block":false,"children":[]}}],"receiver":null,"return_ty":{"normalized":null,"observed":"T","spelling":"T"},"return_ty_span":{"start":53,"end":54},"has_self":false,"abi":{"status":"known","value":"Rust"},"generics":[{"name":"T","span":{"start":17,"end":18}}],"is_variadic":false,"is_async":false,"is_unsafe":false,"is_const":false},"bindings":[{"symbol":"s:740:2","name":"input","decl_span":{"start":32,"end":37},"scope":"scope:source:fn:0:178","hir":{"krate":0,"owner":740,"local":2},"is_param":true,"is_mut":false,"ty":{"normalized":null,"observed":"T","spelling":""}},{"symbol":"s:740:4","name":"min","decl_span":{"start":42,"end":45},"scope":"scope:source:fn:0:178","hir":{"krate":0,"owner":740,"local":4},"is_param":true,"is_mut":false,"ty":{"normalized":null,"observed":"T","spelling":""}}],"body":{"hir":{"krate":0,"owner":740,"local":49},"kind":"expr_block","span":{"start":55,"end":178},"snippet":"{\n    debug_assert!(min == min, \"min must not be NAN\");\n    if input < min {\n        min\n    } else {\n        input\n    }\n}","ty":{"normalized":null,"observed":"T","spelling":""},"res":null,"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[{"hir":{"krate":0,"owner":740,"local":34},"kind":"stmt_semi","span":{"start":61,"end":109},"snippet":"debug_assert!(min == min, \"min must not be NAN\")","ty":null,"res":null,"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[{"hir":{"krate":0,"owner":740,"local":6},"kind":"expr_assert","span":{"start":61,"end":109},"snippet":"debug_assert!(min == min, \"min must not be NAN\")","ty":{"normalized":"unit","observed":"()","spelling":""},"res":{"kind":"standard_assertion","target":null,"description":"core::debug_assert"},"dispatch":null,"expansion":{"callsite":{"start":61,"end":109},"snippet":"debug_assert!(min == min, \"min must not be NAN\")"},"binding":null,"binding_mut":null,"lit":"true","param_index":null,"is_unsafe_block":false,"children":[{"hir":{"krate":0,"owner":740,"local":11},"kind":"expr_binary","span":{"start":75,"end":85},"snippet":"min == min","ty":{"normalized":"bool","observed":"bool","spelling":""},"res":null,"dispatch":{"kind":"overloaded","op":"==","operand_tys":["T","T"],"result_ty":"bool"},"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[{"hir":{"krate":0,"owner":740,"local":12},"kind":"expr_path","span":{"start":75,"end":78},"snippet":"min","ty":{"normalized":null,"observed":"T","spelling":""},"res":{"kind":"local","target":{"krate":0,"owner":740,"local":4},"description":"Local(HirId(DefId(0:740 ~ num_traits[1956]::clamp_min).4))"},"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[]},{"hir":{"krate":0,"owner":740,"local":14},"kind":"expr_path","span":{"start":82,"end":85},"snippet":"min","ty":{"normalized":null,"observed":"T","spelling":""},"res":{"kind":"local","target":{"krate":0,"owner":740,"local":4},"description":"Local(HirId(DefId(0:740 ~ num_traits[1956]::clamp_min).4))"},"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[]}]}]}]},{"hir":{"krate":0,"owner":740,"local":35},"kind":"expr_if","span":{"start":115,"end":176},"snippet":"if input < min {\n        min\n    } else {\n        input\n    }","ty":{"normalized":null,"observed":"T","spelling":""},"res":null,"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[{"hir":{"krate":0,"owner":740,"local":36},"kind":"expr_binary","span":{"start":118,"end":129},"snippet":"input < min","ty":{"normalized":"bool","observed":"bool","spelling":""},"res":null,"dispatch":{"kind":"overloaded","op":"<","operand_tys":["T","T"],"result_ty":"bool"},"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[{"hir":{"krate":0,"owner":740,"local":37},"kind":"expr_path","span":{"start":118,"end":123},"snippet":"input","ty":{"normalized":null,"observed":"T","spelling":""},"res":{"kind":"local","target":{"krate":0,"owner":740,"local":2},"description":"Local(HirId(DefId(0:740 ~ num_traits[1956]::clamp_min).2))"},"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[]},{"hir":{"krate":0,"owner":740,"local":39},"kind":"expr_path","span":{"start":126,"end":129},"snippet":"min","ty":{"normalized":null,"observed":"T","spelling":""},"res":{"kind":"local","target":{"krate":0,"owner":740,"local":4},"description":"Local(HirId(DefId(0:740 ~ num_traits[1956]::clamp_min).4))"},"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[]}]},{"hir":{"krate":0,"owner":740,"local":44},"kind":"expr_block","span":{"start":130,"end":149},"snippet":"{\n        min\n    }","ty":{"normalized":null,"observed":"T","spelling":""},"res":null,"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[{"hir":{"krate":0,"owner":740,"local":42},"kind":"expr_path","span":{"start":140,"end":143},"snippet":"min","ty":{"normalized":null,"observed":"T","spelling":""},"res":{"kind":"local","target":{"krate":0,"owner":740,"local":4},"description":"Local(HirId(DefId(0:740 ~ num_traits[1956]::clamp_min).4))"},"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[]}]},{"hir":{"krate":0,"owner":740,"local":45},"kind":"expr_block","span":{"start":155,"end":176},"snippet":"{\n        input\n    }","ty":{"normalized":null,"observed":"T","spelling":""},"res":null,"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[{"hir":{"krate":0,"owner":740,"local":47},"kind":"expr_path","span":{"start":165,"end":170},"snippet":"input","ty":{"normalized":null,"observed":"T","spelling":""},"res":{"kind":"local","target":{"krate":0,"owner":740,"local":2},"description":"Local(HirId(DefId(0:740 ~ num_traits[1956]::clamp_min).2))"},"dispatch":null,"expansion":null,"binding":null,"binding_mut":null,"lit":null,"param_index":null,"is_unsafe_block":false,"children":[]}]}]}]}}"###;
