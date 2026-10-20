//! Opt-in R0 frontend; the legacy extraction path stays independent.

mod assertions;
mod concrete_types;
pub mod diagnostics;
mod evidence;
mod hir;
mod inherent;
mod library_resolution;
mod nominal_census;
mod options;
pub mod output;
mod scalar_constants;
mod size_calls;
pub mod source;
mod sources;
mod tuple_assignment;

use rstyle_pdg::extraction_profile::ExtractionProfile;
use rstyle_pdg::frontend::*;
use rustc_hir::{self as rhir, def::DefKind, def_id::LocalDefId};
use rustc_middle::ty::TyCtxt;

pub struct Options<'a> {
    pub(super) sources: sources::Sources,
    pub profile: Option<ExtractionProfile>,
    pub invocation: &'a [String],
    pub unit_context: Option<&'a rstyle_pdg::nominal_boundary::UnitContext>,
}

impl Options<'_> {
    pub fn selected(&self) -> ExtractionProfile {
        self.profile.unwrap_or_default()
    }
}

pub fn crate_facts(
    tcx: TyCtxt<'_>,
    scanned: &[source::Function],
    options: &Options<'_>,
) -> Result<rstyle_pdg::unit_store::UnitStore, String> {
    let facts = raw_crate_facts(tcx, scanned, options)?;
    diagnostics::snapshot("pre-helper-certification", &facts)?;
    let store = rstyle_pdg::unit_store::UnitStore::capture(facts)?;
    diagnostics::snapshot("post-helper-certification", store.functions())?;

    Ok(store)
}

/// Owned compiler facts before ordinary-library helper certification.
pub fn raw_crate_facts(
    tcx: TyCtxt<'_>,
    scanned: &[source::Function],
    options: &Options<'_>,
) -> Result<Vec<HirFnFacts>, String> {
    let mut facts = tcx.hir_body_owners()
        .filter(|id| matches!(tcx.def_kind(*id), DefKind::Fn | DefKind::AssocFn))
        .map(|id| function(tcx, id, scanned, options).and_then(|facts| facts.ok_or_else(|| format!(
            "Required source/signature evidence unavailable for {}; R0 extraction failed without a certified remainder", tcx.def_path_str(id.to_def_id())
        ))))
        .collect::<Result<Vec<_>, _>>()?;
    capture_unit_evidence(tcx, options, &mut facts)?;
    for function in &mut facts {
        capture_correspondence(function, options.selected());
    }
    Ok(facts)
}

fn capture_unit_evidence(
    tcx: TyCtxt<'_>,
    options: &Options<'_>,
    facts: &mut [HirFnFacts],
) -> Result<(), String> {
    nominal_census::capture(tcx, options, facts)?;
    library_resolution::capture(tcx, options, facts)?;
    inherent::capture_unit(facts, options)
}

fn function(
    tcx: TyCtxt<'_>,
    id: LocalDefId,
    scanned: &[source::Function],
    options: &Options<'_>,
) -> Result<Option<HirFnFacts>, String> {
    let Some((mut facts, ctx)) = function_source(tcx, id, scanned, options) else {
        return Ok(None);
    };
    concrete_types::capture(&ctx, &mut facts)?;
    inherent::capture(&ctx, id, &mut facts)?;
    rstyle_pdg::field_locations::capture(&mut facts)?;

    Ok(Some(facts))
}

fn function_source<'tcx>(
    tcx: TyCtxt<'tcx>,
    id: LocalDefId,
    scanned: &[source::Function],
    options: &Options<'_>,
) -> Option<(HirFnFacts, hir::Context<'tcx>)> {
    let node = tcx.hir_node_by_def_id(id);
    let sig = node.fn_sig()?;
    let body = tcx.hir_body(node.body_id()?);
    let function_span = sig.span.to(body.value.span).source_callsite();
    let result = tcx.fn_sig(id).instantiate_identity().skip_binder().output();
    let (source_bytes, fn_span) = options.sources.range(tcx, function_span)?;
    let ctx = hir::Context {
        tcx,
        typeck: tcx.typeck(id),
        function_span,
        source_bytes: source_bytes.clone(),
        sources: options.sources.clone(),
        result,
        unit_context: options.unit_context.cloned(),
        profile: options.selected(),
    };
    let params = parameters(&ctx, sig, body);
    let body = ctx.expression(body.value);
    let scope = format!("scope:source:fn:0:{}", source_bytes.len());
    let bindings = params
        .iter()
        .flat_map(|p| hir::bindings(&p.pat, &scope))
        .chain(hir::bindings(&body, &scope))
        .collect();
    let scan = find_source_scan(&ctx, scanned, body_source_span(tcx, id));
    let source_diagnostics = source_findings(&ctx, scan, sig.span);

    let facts = HirFnFacts {
        name: tcx.def_path_str(id.to_def_id()),
        fn_span,
        source: SourceFacts {
            digest: source_bytes.identity().to_owned(),
            bytes: source_bytes,
        },
        signature: signature(&ctx, sig, params, id),
        bindings,
        body,
        build: function_build(tcx, id, function_span, options)?,
        source_diagnostics,
        source_scan_complete: scan.is_some_and(|scan| !scan.conditional_source),
    };
    Some((facts, ctx))
}

fn body_source_span(tcx: TyCtxt<'_>, id: LocalDefId) -> rustc_span::Span {
    let node = tcx.hir_node_by_def_id(id);
    tcx.hir_body(node.body_id().expect("function body"))
        .value
        .span
        .source_callsite()
}

fn find_source_scan<'a>(
    ctx: &hir::Context<'_>,
    scanned: &'a [source::Function],
    body_span: rustc_span::Span,
) -> Option<&'a source::Function> {
    scanned.iter().find(|scan| {
        scan.span.source_callsite() == ctx.function_span
            || (ctx
                .profile
                .supports("rust.conditional-source-correspondence")
                && scan.body_span.source_callsite() == body_span)
    })
}

fn capture_correspondence(facts: &mut HirFnFacts, profile: ExtractionProfile) {
    if profile.supports("rust.conditional-source-correspondence") {
        match rstyle_pdg::source_evidence::capture(facts) {
            Ok(evidence) => {
                record_correspondence(facts, evidence);
            }
            Err(error) => {
                facts.build.frontend_options.insert(
                    "source_correspondence_failure".into(),
                    serde_json::json!(error),
                );
            }
        }
    }
}

fn record_correspondence(
    facts: &mut HirFnFacts,
    evidence: rstyle_pdg::source_evidence::Correspondence,
) {
    let diagnostics = rstyle_pdg::source_evidence::diagnostics(facts, &evidence);
    facts.source_diagnostics.extend(diagnostics);
    facts
        .source_diagnostics
        .retain(|diagnostic| !evidence.inactive(diagnostic.span));

    facts.build.frontend_options.insert(
        rstyle_pdg::source_evidence::KEY.into(),
        serde_json::json!(evidence),
    );
}

fn function_build(
    tcx: TyCtxt<'_>,
    id: LocalDefId,
    span: rustc_span::Span,
    options: &Options<'_>,
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
    let mut build = evidence::build(tcx, options.invocation);
    if let Some(selection) = options.profile {
        build.frontend_options.insert(
            rstyle_pdg::extraction_profile::SELECTION_KEY.into(),
            serde_json::json!(selection.token()),
        );
    }
    if let Some(context) = options.unit_context {
        build.frontend_options.insert(
            rstyle_pdg::nominal_boundary::CONTEXT_KEY.into(),
            serde_json::to_value(context).ok()?,
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
    build.frontend_options.extend(
        [
            ("source_file".into(), serde_json::json!(path)),
            ("source_identifier".into(), serde_json::json!(identifier)),
        ]
        .into_iter()
        .chain(definition_identity(tcx, id, options.selected())),
    );

    Some(build)
}

fn definition_identity(
    tcx: TyCtxt<'_>,
    id: LocalDefId,
    profile: ExtractionProfile,
) -> Option<(String, serde_json::Value)> {
    profile.supports("rust.verified-helper-calls").then(|| {
        (
            "definition_identity".into(),
            serde_json::json!(format!("{:?}", tcx.def_path_hash(id.to_def_id()))),
        )
    })
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

pub fn source_inventory(
    tcx: TyCtxt<'_>,
    scanned: &[source::Function],
    facts: &[HirFnFacts],
    options: &Options<'_>,
) -> Result<serde_json::Value, String> {
    let files = inventory_files(tcx, scanned)?;
    let cfgs = options
        .selected()
        .supports("rust.cfg-inactive-definitions")
        .then(|| evidence::cfgs(tcx));
    let context = InventoryContext {
        facts,
        cfgs: cfgs.as_deref(),
    };
    let records = files
        .into_iter()
        .map(|(path, span)| {
            let (source, span) = options
                .sources
                .range(tcx, span)
                .ok_or("Original compiler file buffer unavailable")?;
            inventory_file(&path, source, span, &context)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(serde_json::json!(
        records.into_iter().flatten().collect::<Vec<_>>()
    ))
}

fn inventory_files(
    tcx: TyCtxt<'_>,
    scanned: &[source::Function],
) -> Result<std::collections::BTreeMap<std::path::PathBuf, rustc_span::Span>, String> {
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

    Ok(files)
}

struct InventoryContext<'a> {
    facts: &'a [HirFnFacts],
    cfgs: Option<&'a [String]>,
}

fn inventory_file(
    path: &std::path::Path,
    source: rstyle_pdg::SourceText,
    original_span: ByteSpan,
    context: &InventoryContext<'_>,
) -> Result<Vec<serde_json::Value>, String> {
    let definitions = match rstyle_pdg::source_evidence::configured_functions(&source, context.cfgs)
    {
        Ok(definitions) => definitions,
        Err(reason) => {
            return Ok(vec![
                serde_json::to_value(UnknownInventory {
                    source_file: path,
                    source_digest: evidence::digest(source.as_bytes()),
                    inventory_status: "unknown",
                    reason,
                })
                .map_err(|e| e.to_string())?,
            ]);
        }
    };

    definitions
        .into_iter()
        .map(|mut definition| {
            definition.span.start = definition
                .span
                .start
                .checked_add(original_span.start)
                .ok_or("Source inventory span overflow")?;
            definition.span.end = definition
                .span
                .end
                .checked_add(original_span.start)
                .ok_or("Source inventory span overflow")?;
            definition.function_span.start = definition
                .function_span
                .start
                .checked_add(original_span.start)
                .ok_or("Source inventory span overflow")?;
            definition.function_span.end = definition
                .function_span
                .end
                .checked_add(original_span.start)
                .ok_or("Source inventory span overflow")?;
            inventory_definition(path, definition, context.facts)
        })
        .collect::<Result<Vec<_>, String>>()
}

fn inventory_definition(
    path: &std::path::Path,
    definition: rstyle_pdg::source_evidence::SourceDefinition,
    facts: &[HirFnFacts],
) -> Result<serde_json::Value, String> {
    let active: Vec<_> = facts
        .iter()
        .filter(|fact| {
            fact.fn_span == definition.function_span
                && fact.source.bytes == definition.function_source
                && fact
                    .build
                    .frontend_options
                    .get("source_identifier")
                    .and_then(serde_json::Value::as_str)
                    == Some(definition.name.trim_start_matches("r#"))
                && fact
                    .build
                    .frontend_options
                    .get("source_file")
                    .and_then(serde_json::Value::as_str)
                    == path.to_str()
        })
        .map(|fact| &fact.name)
        .collect();

    let presence = inventory_presence(definition.inactive_by_cfg, active.len())?;
    serde_json::to_value(DefinitionInventory {
        source_file: path,
        name: definition.name,
        span: definition.span,
        source_digest: evidence::digest(definition.source.as_bytes()),
        source: definition.source,
        function_span: definition.function_span,
        function_source_digest: evidence::digest(definition.function_source.as_bytes()),
        active_functions: active,
        presence,
        inventory_status: "complete_for_source_definitions_outside_macro_tokens",
    })
    .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
struct UnknownInventory<'a> {
    source_file: &'a std::path::Path,
    source_digest: String,
    inventory_status: &'static str,
    reason: String,
}

#[derive(serde::Serialize)]
struct DefinitionInventory<'a> {
    source_file: &'a std::path::Path,
    name: String,
    span: rstyle_pdg::ByteSpan,
    source_digest: String,
    source: String,
    function_span: rstyle_pdg::ByteSpan,
    function_source_digest: String,
    active_functions: Vec<&'a String>,
    presence: &'static str,
    inventory_status: &'static str,
}

fn inventory_presence(inactive: bool, active_count: usize) -> Result<&'static str, String> {
    if active_count > 1 {
        return Err("Ambiguous exact owned-source function correspondence".into());
    }

    if inactive && active_count != 0 {
        return Err("Cfg-inactive definition contradicts exact active HIR correspondence".into());
    }

    let presence = if inactive {
        "inactive_by_cfg"
    } else if active_count == 0 {
        "absent_from_resolved_hir"
    } else {
        "active"
    };
    Ok(presence)
}
