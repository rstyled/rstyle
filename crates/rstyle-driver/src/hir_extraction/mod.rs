//! Opt-in R0 frontend; the legacy extraction path stays independent.

mod evidence;
mod hir;
pub mod output;
pub mod source;

use rstyle_pdg::frontend::*;
use rustc_hir::{self as rhir, def::DefKind, def_id::LocalDefId};
use rustc_middle::ty::TyCtxt;

pub fn crate_facts(
    tcx: TyCtxt<'_>,
    scanned: &[source::Function],
    invocation: &[String],
) -> Result<Vec<HirFnFacts>, String> {
    tcx.hir_body_owners()
        .filter(|id| matches!(tcx.def_kind(*id), DefKind::Fn | DefKind::AssocFn))
        .map(|id| function(tcx, id, scanned, invocation).ok_or_else(|| format!(
            "Required source/signature evidence unavailable for {}; R0 extraction failed without a certified remainder", tcx.def_path_str(id.to_def_id())
        )))
        .collect()
}

fn function(
    tcx: TyCtxt<'_>,
    id: LocalDefId,
    scanned: &[source::Function],
    invocation: &[String],
) -> Option<HirFnFacts> {
    let node = tcx.hir_node_by_def_id(id);
    let sig = node.fn_sig()?;
    let body = tcx.hir_body(node.body_id()?);
    let function_span = sig.span.to(body.value.span).source_callsite();
    let result = tcx.fn_sig(id).instantiate_identity().skip_binder().output();
    let (source_bytes, fn_span) = original_source(tcx, function_span)?;
    let ctx = hir::Context {
        tcx,
        typeck: tcx.typeck(id),
        function_span,
        source_bytes: source_bytes.clone(),
        result,
    };
    let params = parameters(&ctx, sig, body);
    let body = ctx.expression(body.value);
    let scope = format!("scope:source:fn:0:{}", source_bytes.len());
    let bindings = params
        .iter()
        .flat_map(|p| hir::bindings(&p.pat, &scope))
        .chain(hir::bindings(&body, &scope))
        .collect();
    let scan = scanned.iter().find(|scan| {
        scan.span.source_callsite() == function_span
            || (selected_conditional()
                && scan.body_span.source_callsite()
                    == ctx
                        .tcx
                        .hir_body(node.body_id().expect("function body"))
                        .value
                        .span
                        .source_callsite())
    });
    let source_diagnostics = source_findings(&ctx, scan, sig.span);

    let mut facts = HirFnFacts {
        name: tcx.def_path_str(id.to_def_id()),
        fn_span,
        source: SourceFacts {
            digest: evidence::digest(source_bytes.as_bytes()),
            bytes: source_bytes,
        },
        signature: signature(&ctx, sig, params, id),
        bindings,
        body,
        build: function_build(tcx, id, function_span, invocation)?,
        source_diagnostics,
        source_scan_complete: scan.is_some_and(|scan| !scan.conditional_source),
    };
    if selected_conditional() {
        facts
            .source_diagnostics
            .extend(rstyle_pdg::source_evidence::diagnostics(&facts));
        match rstyle_pdg::source_evidence::capture(&facts) {
            Ok(evidence) => {
                facts.build.frontend_options.insert(
                    rstyle_pdg::source_evidence::KEY.into(),
                    serde_json::json!(evidence),
                );
            }
            Err(error) => {
                facts.build.frontend_options.insert(
                    "source_correspondence_failure".into(),
                    serde_json::json!(error),
                );
            }
        }
    }
    Some(facts)
}

fn function_build(
    tcx: TyCtxt<'_>,
    id: LocalDefId,
    span: rustc_span::Span,
    invocation: &[String],
) -> Option<BuildFacts> {
    let file = tcx.sess.source_map().lookup_source_file(span.lo());
    let rustc_span::FileName::Real(real) = &file.name else {
        return None;
    };
    let path = real.local_path()?;
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    let mut build = evidence::build(tcx, invocation);
    if let Ok(selection) = std::env::var("RSTYLE_HIR_EXTRACTION_PROFILE") {
        build.frontend_options.insert(
            rstyle_pdg::extraction_profile::SELECTION_KEY.into(),
            serde_json::json!(selection),
        );
    }
    let identifier = tcx
        .def_ident_span(id.to_def_id())
        .and_then(|span| {
            tcx.sess
                .source_map()
                .span_to_snippet(span.source_callsite())
                .ok()
        })
        .map(|name| name.strip_prefix("r#").unwrap_or(&name).to_owned());
    build
        .frontend_options
        .insert("source_file".into(), serde_json::json!(path));
    build
        .frontend_options
        .insert("source_identifier".into(), serde_json::json!(identifier));

    Some(build)
}

/// Recover the compiler's original input using its recorded normalization map.
/// rustc removes only a leading BOM and the CR in CRLF; no filesystem reread
/// can race with the source buffer actually used for compilation.
fn original_source(tcx: TyCtxt<'_>, span: rustc_span::Span) -> Option<(String, ByteSpan)> {
    let file = tcx.sess.source_map().lookup_source_file(span.lo());
    let normalized = file.src.as_ref()?;
    let mut bytes = normalized.as_bytes().to_vec();

    for change in file.normalized_pos.iter().rev() {
        if change.pos.0 == 0 && change.diff == 3 {
            bytes.splice(0..0, [0xef, 0xbb, 0xbf]);
        } else {
            let position = change.pos.0.checked_sub(1)? as usize;
            if bytes.get(position) != Some(&b'\n') {
                return None;
            }
            bytes.insert(position, b'\r');
        }
    }

    let start = file.original_relative_byte_pos(span.lo()).0 as usize;
    let end = file.original_relative_byte_pos(span.hi()).0 as usize;
    let source = String::from_utf8(bytes.get(start..end)?.to_vec()).ok()?;

    Some((
        source,
        ByteSpan {
            start: start as u32,
            end: end as u32,
        },
    ))
}

fn parameters<'tcx>(
    ctx: &hir::Context<'tcx>,
    sig: &rhir::FnSig<'tcx>,
    body: &rhir::Body<'tcx>,
) -> Vec<ParamFact> {
    sig.decl
        .inputs
        .iter()
        .zip(body.params)
        .enumerate()
        .map(|(position, (ty, param))| {
            let pat = HirNode {
                param_index: Some(position as u32),
                ..ctx.pattern(param.pat)
            };
            let resolved = ctx.typeck.node_type(param.pat.hir_id);
            ParamFact {
                symbol: pat.binding.clone().unwrap_or_default(),
                position: position as u32,
                name_span: pat.span,
                ty_span: ctx.span(ty.span),
                ty: hir::resolved(resolved, ctx.snippet(ty.span)),
                pat,
            }
        })
        .collect()
}

fn signature<'tcx>(
    ctx: &hir::Context<'tcx>,
    sig: &rhir::FnSig<'tcx>,
    params: Vec<ParamFact>,
    id: LocalDefId,
) -> SignatureFacts {
    let (return_span, spelling) = match sig.decl.output {
        rhir::FnRetTy::Return(ty) => (ty.span, ctx.snippet(ty.span)),
        rhir::FnRetTy::DefaultReturn(span) => (span, "()".to_string()),
    };
    let has_self = sig.decl.implicit_self().has_implicit_self();
    let receiver = has_self.then(|| ReceiverFact {
        span: params[0].name_span,
        snippet: params[0].pat.snippet.clone(),
    });
    let generics = ctx
        .tcx
        .hir_get_generics(id)
        .map(|g| {
            g.params
                .iter()
                .map(|p| GenericFact {
                    name: p.name.ident().to_string(),
                    span: ctx.span(p.span),
                })
                .collect()
        })
        .unwrap_or_default();

    SignatureFacts {
        sig_span: ctx.span(sig.span),
        params,
        receiver,
        return_ty: hir::resolved(ctx.result, spelling),
        return_ty_span: ctx.span(return_span),
        has_self,
        abi: known_str(sig.header.abi.to_string().trim_matches('"')),
        generics,
        is_variadic: sig.decl.c_variadic(),
        is_async: sig.header.is_async(),
        is_unsafe: sig.header.is_unsafe(),
        is_const: matches!(sig.header.constness, rhir::Constness::Const { .. }),
    }
}

fn source_findings(
    ctx: &hir::Context<'_>,
    scan: Option<&source::Function>,
    signature_span: rustc_span::Span,
) -> Vec<HirDiagnostic> {
    if signature_span.from_expansion() {
        return vec![HirDiagnostic {
            capability: "rust.macros".to_string(),
            construct: "macro_generated_function".to_string(),
            span: ctx.span(signature_span),
            snippet: ctx.snippet(signature_span),
            severity: "error".to_string(),
            reason: "Macro-generated functions are deferred to rust.macros.".to_string(),
        }];
    }

    scan.into_iter()
        .flat_map(|scan| &scan.findings)
        .map(|finding| HirDiagnostic {
            capability: finding.capability.to_string(),
            construct: finding.construct.to_string(),
            span: ctx.span(finding.span),
            snippet: ctx.snippet(finding.span),
            severity: "error".to_string(),
            reason: format!(
                "Source construct is deferred to {}; remove it or assess it under a later policy.",
                finding.capability
            ),
        })
        .collect()
}

fn selected_conditional() -> bool {
    std::env::var("RSTYLE_HIR_EXTRACTION_PROFILE")
        .ok()
        .and_then(|value| rstyle_pdg::extraction_profile::ExtractionProfile::parse(&value).ok())
        .is_some_and(|profile| profile.conditional_source())
}

pub fn source_inventory(
    tcx: TyCtxt<'_>,
    scanned: &[source::Function],
    facts: &[HirFnFacts],
) -> Result<serde_json::Value, String> {
    let mut files = std::collections::BTreeMap::new();
    for span in scanned.iter().map(|f| f.span.source_callsite()).chain(
        tcx.hir_body_owners()
            .filter(|id| matches!(tcx.def_kind(*id), DefKind::Fn | DefKind::AssocFn))
            .filter_map(|id| {
                tcx.hir_node_by_def_id(id)
                    .fn_sig()
                    .map(|sig| sig.span.source_callsite())
            }),
    ) {
        let file = tcx.sess.source_map().lookup_source_file(span.lo());
        let rustc_span::FileName::Real(real) = &file.name else {
            continue;
        };
        let Some(path) = real.local_path() else {
            continue;
        };
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(path)
        };
        files.insert(
            path,
            rustc_span::Span::with_root_ctxt(file.start_pos, file.end_position()),
        );
    }
    let mut records = Vec::new();
    for (path, span) in files {
        let (source, _) =
            original_source(tcx, span).ok_or("Original compiler file buffer unavailable")?;
        let definitions = rstyle_pdg::source_evidence::functions(&source);
        match definitions {
            Err(reason) => records.push(serde_json::json!({"source_file":path,"source_digest":evidence::digest(source.as_bytes()),"inventory_status":"unknown","reason":reason})),
            Ok(definitions) => for definition in definitions {
                let active: Vec<_> = facts.iter().filter(|fact| fact.fn_span.start >= definition.span.start && fact.fn_span.end <= definition.span.end
                    && fact.build.frontend_options.get("source_file").and_then(serde_json::Value::as_str) == path.to_str()).map(|fact| &fact.name).collect();
                records.push(serde_json::json!({"source_file":path,"name":definition.name,"span":definition.span,"source_digest":evidence::digest(definition.source.as_bytes()),"source":definition.source,"active_functions":active,"presence":if active.is_empty() { "absent_from_resolved_hir" } else { "active" },"inventory_status":"complete_for_source_definitions_outside_macro_tokens"}));
            }
        }
    }
    Ok(serde_json::json!(records))
}
