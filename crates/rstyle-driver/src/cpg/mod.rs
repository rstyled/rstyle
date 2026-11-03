//! `compat.cpg` documents from rustc. Each compilation writes one bundled unit
//! document, and beside it the FnFacts the driver derives itself, so core's
//! derivation from the document can be checked against the driver's own.

mod body;
mod compiler;
mod places;
pub(crate) mod sysroot;
mod unit;

use crate::{extract, ids};
use rstyle_core::advisory::runtime_bounds::Input;
use rustc_hir::def_id::LocalDefId;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::LOCAL_CRATE;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Directory the documents are written to; setting it selects this mode.
pub const DIR_ENV: &str = "RSTYLE_CPG_DIR";
/// Workspace root that file paths are relative to.
pub const ROOT_ENV: &str = "RSTYLE_CPG_ROOT";

pub fn emit(
    tcx: TyCtxt<'_>,
    complexity: &crate::driver_complexity::Functions,
    root: &Path,
    dir: &Path,
) -> Result<(), String> {
    let options = extract::Options {
        root,
        with_pdg: true,
        complexity: Some(complexity),
    };
    let mut unit = unit::Unit::new(tcx, root.to_path_buf());
    let captured = crate::hir_extraction::function_owners(tcx)
        .map(|id| capture(&mut unit, id, &options))
        .collect::<Result<Vec<_>, String>>()?;
    let (functions, reference): (Vec<_>, Vec<_>) = captured.into_iter().flatten().unzip();
    if functions.is_empty() {
        return Ok(());
    }

    let name = unit_name(tcx);
    let document = document(tcx, unit, functions, &name);
    write(&dir.join(format!("{name}.cpg.json")), &document)?;
    write(
        &dir.join(format!("{name}.facts.json")),
        &serde_json::to_value(reference).map_err(|e| e.to_string())?,
    )
}

/// Each function's runtime-bound program by owner identity, as the core lowers it
/// from the crate's document. An owner the document cannot record keeps its reason.
pub fn runtime_bounds(tcx: TyCtxt<'_>, root: &Path) -> Result<BTreeMap<String, Input>, String> {
    let mut unit = unit::Unit::new(tcx, root.to_path_buf());
    let mut programs = BTreeMap::new();
    let mut recorded = Vec::new();
    for id in crate::hir_extraction::function_owners(tcx) {
        match function(&mut unit, id) {
            Ok(function) => recorded.push(function),
            Err(reason) => {
                _ = programs.insert(
                    ids::def_id_str(tcx, id.to_def_id()),
                    Input::Unavailable { reason },
                )
            }
        }
    }

    let document = document(tcx, unit, recorded, &unit_name(tcx));
    programs.extend(rstyle_core::cpg::run_bounds(vec![document])?);
    Ok(programs)
}

fn unit_name(tcx: TyCtxt<'_>) -> String {
    format!(
        "{}-{:016x}",
        tcx.crate_name(LOCAL_CRATE),
        tcx.stable_crate_id(LOCAL_CRATE).as_u64()
    )
}

/// One function's document record and reference facts.
fn capture(
    unit: &mut unit::Unit<'_>,
    id: LocalDefId,
    options: &extract::Options<'_>,
) -> Result<Option<(Value, rstyle_core::facts::FnFacts)>, String> {
    let Some(facts) = extract::function_facts(unit.tcx, id, options) else {
        return Ok(None);
    };

    Ok(Some((function(unit, id)?, facts)))
}

fn write(path: &PathBuf, value: &Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(value).map_err(|error| error.to_string())?;
    std::fs::write(path, bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn document(tcx: TyCtxt<'_>, unit: unit::Unit<'_>, functions: Vec<Value>, name: &str) -> Value {
    let (files, definitions, types) = unit.tables();
    let session = tcx.sess;
    json!({
        "schema_version": "0.1.0",
        "profile": {"id": "compat.cpg", "version": "0.1.0"},
        "layout": "bundled",
        "document": "unit",
        "producer": {"name": "rstyle-driver", "version": env!("CARGO_PKG_VERSION"), "namespace": "rust"},
        "coverage": unit::coverage(),
        "unit": {
            "id": format!("unit:{name}"),
            "language": {"name": "rust", "version": session.edition().to_string()},
            "toolchain": {"name": "rustc", "version": rustc_interface::util::rustc_version_str().unwrap_or("unknown")},
            "target": {"triple": session.opts.target_triple.tuple(),
                       "pointer_width": session.target.pointer_width},
            "configuration": [],
            "semantics": {"integer_overflow": if session.overflow_checks() { "checked" } else { "wrapping" },
                          "exceptions": if format!("{:?}", session.panic_strategy()) == "Abort" { "abort" } else { "unwind" }},
            "files": files, "namespaces": [], "imports": [], "definitions": definitions, "types": types,
            "values": [], "instances": [], "functions": functions,
        },
    })
}

// rstyle-allow(mutated_targets, mutation_per_target): assembles one document from the builder that accumulates it
fn function(unit: &mut unit::Unit<'_>, id: LocalDefId) -> Result<Value, String> {
    let tcx = unit.tcx;
    let node = tcx.hir_node_by_def_id(id);
    let (sig, body) = match (node.fn_sig(), node.body_id()) {
        (Some(sig), Some(body)) => (sig, tcx.hir_body(body)),
        _ => return Err(format!("{} has no signature or body", tcx.def_path_str(id))),
    };
    let missing = || format!("{} has no workspace source span", tcx.def_path_str(id));
    // A function a library macro generates, as `thread_local!` does, is located at
    // the workspace invocation that expands it.
    let mut located = |span: rustc_span::Span| {
        unit.span(span)
            .or_else(|| unit.span(span.source_callsite()))
            .ok_or_else(missing)
    };
    let span = located(sig.span.to(body.value.span))?;
    let signature_span = located(sig.span)?;
    let has_self = sig.decl.implicit_self().has_implicit_self();
    let params = body.params.iter().map(|param| param.pat.hir_id).collect();
    let mut builder = body::Builder::new(unit, id, params);
    let mut signature = signature(&mut builder, id, (sig, body), has_self);
    signature["span"] = signature_span;

    let root = builder.expr(body.value).ok_or_else(missing)?;
    let places = places::places(tcx, id, &mut builder);
    let implements = tcx
        .opt_associated_item(id.to_def_id())
        .and_then(|item| item.trait_item_def_id())
        .map(|trait_fn| builder.unit.definition(trait_fn));
    let definition = builder.unit.definition(id.to_def_id());

    let mut function = json!({
        "id": format!("fn:{}", crate::ids::def_id_str(tcx, id.to_def_id())), "definition": definition,
        "name": tcx.def_path_str(id.to_def_id()), "span": span, "signature": signature, "root": root,
        "nodes": builder.nodes, "cfg": {"entry": "entry", "exits": [], "edges": []}, "symbols": builder.symbols,
        "accesses": [], "refs": builder.refs, "places": places, "values": [],
    });
    if let Some(implements) = implements {
        function["implements"] = json!(implements);
    }
    Ok(function)
}

// rstyle-allow(mutated_targets, mutation_per_target): assembles one document from the builder that accumulates it
/// Parameters in order, the receiver apart, with their symbols and types.
fn signature<'tcx>(
    builder: &mut body::Builder<'_, 'tcx>,
    id: LocalDefId,
    (sig, body): (&'tcx rustc_hir::FnSig<'tcx>, &'tcx rustc_hir::Body<'tcx>),
    has_self: bool,
) -> Value {
    let tcx = builder.unit.tcx;
    let fn_sig = tcx.fn_sig(id).instantiate_identity().skip_binder();
    let mut parameters = Vec::new();
    let mut receiver = None;
    for (index, param) in body.params.iter().enumerate() {
        let (symbol, bindings) = builder.parameter(param.pat, index);
        let ty = builder.unit.ty(fn_sig.inputs()[index]);
        if has_self && index == 0 {
            receiver = Some(json!({"symbol": symbol, "type": ty}));
            continue;
        }
        let position = index - usize::from(has_self);
        let mut parameter =
            json!({"position": position, "symbol": symbol, "type": ty, "bindings": bindings});
        if let Some(span) = builder.unit.span(param.pat.span) {
            parameter["span"] = span;
        }
        if let Some(annotation) = sig
            .decl
            .inputs
            .get(index)
            .and_then(|ty| builder.unit.span(ty.span))
        {
            parameter["annotation"] = annotation;
        }
        parameters.push(parameter);
    }

    let mut signature = json!({"parameters": parameters, "result": builder.unit.ty(fn_sig.output()),
                               "generics": generics(builder.unit, id), "lifetimes": [], "outlives": [],
                               "modifiers": fn_modifiers(sig.header)});
    if let Some(receiver) = receiver {
        signature["receiver"] = receiver;
    }
    if let Some(implementation) = tcx.impl_of_assoc(id.to_def_id()) {
        let owner = tcx
            .type_of(implementation)
            .instantiate_identity()
            .skip_norm_wip();
        signature["owner_type"] = json!(builder.unit.ty(owner));
    }
    signature
}

/// The function's own type parameters, with the traits bounding each.
fn generics(unit: &mut unit::Unit<'_>, id: LocalDefId) -> Vec<Value> {
    let tcx = unit.tcx;
    let predicates = tcx.clauses_of(id).clauses;
    tcx.generics_of(id)
        .own_params
        .iter()
        .filter(|param| {
            matches!(
                param.kind,
                rustc_middle::ty::GenericParamDefKind::Type { .. }
            )
        })
        .map(|param| {
            let bounds: std::collections::BTreeSet<String> = predicates
                .iter()
                .filter_map(|(clause, _)| clause.as_trait_clause())
                .filter(|bound| {
                    matches!(bound.self_ty().skip_binder().kind(),
                             rustc_middle::ty::Param(own) if own.index == param.index)
                })
                .map(|bound| unit.definition(bound.def_id()))
                .collect();
            json!({"name": param.name.as_str(), "definition": unit.definition(param.def_id),
                   "bounds": bounds.into_iter().collect::<Vec<_>>()})
        })
        .collect()
}

fn fn_modifiers(header: rustc_hir::FnHeader) -> Vec<&'static str> {
    [
        (header.is_async(), "async"),
        (header.is_unsafe(), "unsafe"),
        (header.constness != rustc_hir::Constness::NotConst, "const"),
        (header.abi != rustc_abi::ExternAbi::Rust, "rust:foreign-abi"),
    ]
    .into_iter()
    .filter_map(|(present, modifier)| present.then_some(modifier))
    .collect()
}
