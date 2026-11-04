//! `compat.cpg` documents from rustc. Each compilation writes one bundled unit
//! document, and beside it the FnFacts the driver derives itself, so core's
//! derivation from the document can be checked against the driver's own.

mod body;
mod compiler;
mod namespaces;
mod places;
pub(crate) mod sysroot;
mod unit;

use crate::{extract, ids};
use rstyle_core::advisory::runtime_bounds::Input;
use rstyle_pdg::nominal_binding::PackageContext;
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
    unit.record_local_files();
    let document = document(tcx, (unit, true), functions, &name);
    write(&dir.join(format!("{name}.cpg.json")), &document)?;
    write(
        &dir.join(format!("{name}.facts.json")),
        &serde_json::to_value(reference).map_err(|e| e.to_string())?,
    )
}

/// Each function's runtime-bound program by owner identity, as the core lowers it
/// from the crate's documents. An owner the document cannot record keeps its reason.
pub fn runtime_bounds(
    tcx: TyCtxt<'_>,
    origin: (&Path, Option<&PackageContext>),
) -> Result<BTreeMap<String, Input>, String> {
    let mut programs = BTreeMap::new();
    for (document, unrecorded) in documents(tcx, origin) {
        let unavailable = unrecorded.into_iter().map(|(id, reason)| {
            (
                ids::def_id_str(tcx, id.to_def_id()),
                Input::Unavailable { reason },
            )
        });
        programs.extend(unavailable);
        programs.extend(rstyle_core::cpg::run_bounds(vec![document])?);
    }
    Ok(programs)
}

/// The crate's function documents, each with the owners it could not record and why.
/// Each function reads from its own body and the tables it references, so a batch of
/// functions per document bounds memory by the batch rather than the crate; a batch
/// keeps to one source file, which each document then carries once.
/// `origin` is the workspace root paths are relative to, and the package the capture
/// selected, if any.
pub fn documents<'tcx, 'r>(
    tcx: TyCtxt<'tcx>,
    origin: (&'r Path, Option<&'r PackageContext>),
) -> impl Iterator<Item = (Value, Vec<(LocalDefId, String)>)> + use<'tcx, 'r> {
    documents_in(tcx, origin, Capture::default())
}

/// A frontend's own observations of a function, recorded as its extensions.
pub type Observe<'r> = dyn Fn(LocalDefId) -> serde_json::Map<String, Value> + 'r;

/// What a capture adds to the crate's documents: its session, each function's own
/// observations, and the unit's, which the first document records.
#[derive(Default)]
pub struct Capture<'r> {
    pub session: Option<Value>,
    pub observe: Option<&'r Observe<'r>>,
    pub unit: serde_json::Map<String, Value>,
}

// rstyle-allow(side_effect_density): documents are produced lazily, one batch at a time
/// The crate's function documents, each recording the capture `session` when given:
/// the compiler invocation, the selected profile and unit, and each file's path.
pub fn documents_in<'tcx, 'r>(
    tcx: TyCtxt<'tcx>,
    (root, package): (&'r Path, Option<&'r PackageContext>),
    capture: Capture<'r>,
) -> impl Iterator<Item = (Value, Vec<(LocalDefId, String)>)> + use<'tcx, 'r> {
    let package = cargo_package(package);
    batches(tcx)
        .into_iter()
        .enumerate()
        .map(move |(index, batch)| {
            let origin = (root, package.clone(), capture.session.clone());
            // The first document records every file of the unit, and the unit's observations.
            let unit = (index == 0).then(|| capture.unit.clone());
            batch_document(tcx, (origin, capture.observe, unit), &batch)
        })
}

/// The package a unit belongs to, as Cargo names it: the selected package of a
/// capture, else the one Cargo compiles, with its manifest's digest.
fn cargo_package(selected: Option<&PackageContext>) -> Option<Value> {
    if let Some(package) = selected {
        return Some(json!({"name": package.name, "version": package.version,
                           "manifest_digest": package.manifest_digest, "package_id": package.package_id}));
    }
    let name = std::env::var("CARGO_PKG_NAME").ok()?;
    let version = std::env::var("CARGO_PKG_VERSION").ok()?;
    let directory = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let manifest = std::fs::read_to_string(Path::new(&directory).join("Cargo.toml")).ok()?;
    let digest = rstyle_pdg::SourceSnapshot::from(manifest)
        .checksum()
        .to_owned();

    Some(json!({"name": name, "version": version, "manifest_digest": digest}))
}

const BATCH: usize = 256;

/// The crate's function owners in order, in batches of consecutive owners of one file.
fn batches(tcx: TyCtxt<'_>) -> Vec<Vec<LocalDefId>> {
    let source_map = tcx.sess.source_map();
    let file = |id: LocalDefId| {
        source_map
            .lookup_source_file(tcx.def_span(id).lo())
            .start_pos
    };
    let mut batches: Vec<(rustc_span::BytePos, Vec<LocalDefId>)> = Vec::new();
    for id in crate::hir_extraction::function_owners(tcx) {
        let file = file(id);
        match batches.last_mut() {
            Some((current, batch)) if *current == file && batch.len() < BATCH => batch.push(id),
            _ => batches.push((file, vec![id])),
        }
    }

    batches.into_iter().map(|(_, batch)| batch).collect()
}

/// Where a batch's document comes from: the workspace root, the Cargo package and
/// the capture session it records, if any.
type Origin<'r> = (&'r Path, Option<Value>, Option<Value>);

// rstyle-allow(mutated_targets): assembles one document from the builder that accumulates it
fn batch_document(
    tcx: TyCtxt<'_>,
    ((root, package, session), observe, observed): (
        Origin<'_>,
        Option<&Observe<'_>>,
        Option<serde_json::Map<String, Value>>,
    ),
    owners: &[LocalDefId],
) -> (Value, Vec<(LocalDefId, String)>) {
    let mut unit = unit::Unit::new(tcx, root.to_path_buf());
    // The first document records the whole unit: every file, namespace and import.
    let whole = observed.is_some();
    if whole {
        unit.record_local_files();
    }
    let mut unrecorded = Vec::new();
    let mut recorded = Vec::new();
    for &id in owners {
        match function(&mut unit, id) {
            Ok(mut function) => {
                let extensions = observe.map(|observe| observe(id)).unwrap_or_default();
                if !extensions.is_empty() {
                    function["extensions"] = Value::Object(extensions);
                }
                recorded.push(function);
            }
            Err(reason) => unrecorded.push((id, reason)),
        }
    }

    let paths = unit.local_paths();
    let mut document = document(tcx, (unit, whole), recorded, &unit_name(tcx));
    if let Some(package) = package {
        document["unit"]["extensions"]["rust:cargo-package"] = package;
    }
    if let Some(mut session) = session {
        session["files"] = json!(paths);
        document["unit"]["extensions"]["rust:compiler-session"] = session;
    }
    for (name, observation) in observed.into_iter().flatten() {
        document["unit"]["extensions"][name] = observation;
    }

    (document, unrecorded)
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

/// The compiler's version, without the tool's name `rustc --version` begins with.
fn toolchain_version() -> &'static str {
    let version = env!("RSTYLE_RUSTC_VERSION");
    version.strip_prefix("rustc ").unwrap_or(version)
}

/// The crate root file's ID, as the unit names files.
fn root_file(tcx: TyCtxt<'_>, root: &Path) -> Option<String> {
    let path = tcx.sess.local_crate_source_file()?;
    let file = tcx
        .sess
        .source_map()
        .get_source_file(&rustc_span::FileName::Real(path))?;
    crate::extract::source_path(tcx, &file, root).map(|path| format!("file:{path}"))
}

/// A unit document of `functions`; a `whole` one, which has recorded every file of the
/// unit, also records its namespaces and imports.
fn document(
    tcx: TyCtxt<'_>,
    (mut unit, whole): (unit::Unit<'_>, bool),
    functions: Vec<Value>,
    name: &str,
) -> Value {
    let (namespaces, imports) = match whole {
        true => namespaces::record(&mut unit),
        false => (Vec::new(), Vec::new()),
    };
    let root = unit.root.clone();
    let mut extensions = json!({"rust:llvm-target": tcx.sess.target.llvm_target.to_string()});
    if let Some(conversion) = reflexive_conversion(&mut unit) {
        extensions["rust:reflexive-conversion"] = conversion;
    }
    let (files, definitions, types, values) = unit.tables();
    let session = tcx.sess;

    json!({
        "schema_version": "0.1.0",
        "profile": {"id": "compat.cpg", "version": "0.1.0"},
        "layout": "bundled",
        "document": "unit",
        "producer": {"name": "rstyle-driver", "version": env!("CARGO_PKG_VERSION"), "namespace": "rust",
                     "revision": env!("RSTYLE_PRODUCER_REVISION")},
        "coverage": unit::coverage(),
        "unit": {
            "id": format!("unit:{name}"),
            "root_file": root_file(tcx, &root),
            "language": {"name": "rust", "version": session.edition().to_string()},
            "toolchain": {"name": "rustc", "version": toolchain_version(),
                          "commit": env!("RSTYLE_RUSTC_COMMIT")},
            "target": {"triple": session.opts.target_triple.tuple(),
                       "pointer_width": session.target.pointer_width,
                       "data_layout": session.target.data_layout.to_string()},
            "configuration": crate::hir_extraction::cfgs(tcx),
            "semantics": {"integer_overflow": if session.overflow_checks() { "checked" } else { "wrapping" },
                          "exceptions": if format!("{:?}", session.panic_strategy()) == "Abort" { "abort" } else { "unwind" },
                          "debug_assertions": session.opts.debug_assertions,
                          "optimization": format!("{:?}", session.opts.optimize)},
            "files": files, "namespaces": namespaces, "imports": imports, "definitions": definitions, "types": types,
            "values": values, "instances": [], "functions": functions,
            // The LLVM target the compiler builds for, which the frontend's facts name.
            "extensions": extensions,
        },
    })
}

/// The conversion `?` applies to an error of the same type: `From::from`, and the
/// implementation every type's conversion to itself selects.
fn reflexive_conversion(unit: &mut unit::Unit<'_>) -> Option<Value> {
    let tcx = unit.tcx;
    let method = tcx.get_diagnostic_item(rustc_span::Symbol::intern("from_fn"))?;
    let arguments = tcx.mk_args(&[tcx.types.u8.into(), tcx.types.u8.into()]);
    let environment = rustc_middle::ty::TypingEnv::fully_monomorphized();
    let instance =
        rustc_middle::ty::Instance::try_resolve(tcx, environment, method, arguments).ok()??;
    let implementation = unit.definition(instance.def_id());

    Some(json!({"method": unit.definition(method), "implementation": implementation}))
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
            let annotation = sig.decl.inputs.first().map(|ty| ty.span);
            receiver = Some(annotated(
                builder.unit,
                json!({"symbol": symbol, "type": ty}),
                annotation,
            ));
            continue;
        }
        let position = index - usize::from(has_self);
        let record = json!({"position": position, "symbol": symbol, "type": ty, "bindings": bindings,
                            "pattern": builder.id(param.pat.hir_id),
                            "original_kind": compiler::original_kind(tcx, param.pat.hir_id)});
        let annotation = sig.decl.inputs.get(index).map(|ty| ty.span);
        parameters.push(located_parameter(
            builder.unit,
            record,
            (param.pat.span, annotation),
        ));
    }

    let mut signature = json!({"parameters": parameters, "result": builder.unit.ty(fn_sig.output()),
                               "generics": generics(builder.unit, id), "lifetimes": lifetimes(builder.unit, id),
                               "constants": constants(builder.unit, id),
                               "outlives": [], "modifiers": fn_modifiers(sig.header)});
    result(builder.unit, sig, &mut signature);
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

/// A parameter with the spans its pattern and its type are written at.
fn located_parameter(
    unit: &mut unit::Unit<'_>,
    mut record: Value,
    (pattern, annotation): (rustc_span::Span, Option<rustc_span::Span>),
) -> Value {
    if let Some(span) = unit.span(pattern) {
        record["span"] = span;
    }
    // A library macro's parameter is written at the workspace invocation that expands it.
    let annotation = annotation.and_then(|span| {
        unit.span(span)
            .or_else(|| unit.span(span.source_callsite()))
    });
    if let Some(annotation) = annotation {
        record["annotation"] = annotation;
    }
    record
}

/// A receiver with the span its type is written at, located at the outermost call site.
fn annotated(
    unit: &mut unit::Unit<'_>,
    mut record: Value,
    annotation: Option<rustc_span::Span>,
) -> Value {
    if let Some(span) = annotation.and_then(|span| unit.span(span.source_callsite())) {
        record["annotation"] = span;
    }
    record
}

/// Where the result type is written, or the empty span where an implicit one would be.
fn result(unit: &mut unit::Unit<'_>, sig: &rustc_hir::FnSig<'_>, signature: &mut Value) {
    let (span, implicit) = match sig.decl.output {
        rustc_hir::FnRetTy::Return(ty) => (ty.span, false),
        rustc_hir::FnRetTy::DefaultReturn(span) => (span, true),
    };
    if let Some(span) = unit.span(span.source_callsite()) {
        signature["result_span"] = span;
    }
    if implicit {
        signature["implicit_result"] = json!(true);
    }
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
            let mut record = json!({"name": param.name.as_str(), "definition": unit.definition(param.def_id),
                                    "bounds": bounds.into_iter().collect::<Vec<_>>()});
            if let Some((position, declared)) = declared(unit, id, param.def_id) {
                record["position"] = json!(position);
                if let Some(span) = unit.span(declared.span.source_callsite()) {
                    record["span"] = span;
                }
            }
            record
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

/// The function's own declared generic parameter for `def_id`, with its position.
fn declared<'tcx>(
    unit: &unit::Unit<'tcx>,
    id: LocalDefId,
    def_id: rustc_hir::def_id::DefId,
) -> Option<(usize, &'tcx rustc_hir::GenericParam<'tcx>)> {
    let generics = unit.tcx.hir_get_generics(id)?;
    generics
        .params
        .iter()
        .enumerate()
        .find(|(_, param)| param.def_id.to_def_id() == def_id)
}

/// The function's own lifetime parameters, declared or introduced by elision.
fn lifetimes(unit: &mut unit::Unit<'_>, id: LocalDefId) -> Vec<Value> {
    let Some(generics) = unit.tcx.hir_get_generics(id) else {
        return Vec::new();
    };
    let mut lifetimes = Vec::new();
    for (position, param) in generics.params.iter().enumerate() {
        let rustc_hir::GenericParamKind::Lifetime { kind } = param.kind else {
            continue;
        };
        let elided = !matches!(kind, rustc_hir::LifetimeParamKind::Explicit);
        let mut record = json!({"id": format!("lifetime:{}", crate::ids::def_id_str(unit.tcx, param.def_id.to_def_id())),
                                "name": param.name.ident().to_string(), "position": position, "elided": elided});
        if let Some(span) = unit.span(param.span.source_callsite()) {
            record["span"] = span;
        }
        lifetimes.push(record);
    }
    lifetimes
}

/// The function's own constant parameters, with their types.
fn constants(unit: &mut unit::Unit<'_>, id: LocalDefId) -> Vec<Value> {
    let tcx = unit.tcx;
    let Some(generics) = tcx.hir_get_generics(id) else {
        return Vec::new();
    };
    let mut constants = Vec::new();
    for (position, param) in generics.params.iter().enumerate() {
        if !matches!(param.kind, rustc_hir::GenericParamKind::Const { .. }) {
            continue;
        }
        let ty = tcx
            .type_of(param.def_id)
            .instantiate_identity()
            .skip_norm_wip();
        let mut record = json!({"name": param.name.ident().to_string(),
                                "definition": unit.definition(param.def_id.to_def_id()),
                                "type": unit.ty(ty), "position": position});
        if let Some(span) = unit.span(param.span.source_callsite()) {
            record["span"] = span;
        }
        constants.push(record);
    }

    constants
}
