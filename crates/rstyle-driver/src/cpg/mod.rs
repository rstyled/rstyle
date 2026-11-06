//! `compat.cpg` documents from rustc. `cargo rstyle cpg emit` writes one bundled
//! unit document per compilation; captures write the crate in batches.

mod accesses;
mod body;
mod compiler;
mod flow;
mod generated_root;
mod namespaces;
mod places;
pub mod publish;
mod source;
pub(crate) mod sysroot;
mod unit;

use rstyle_capture::PackageContext;
use rustc_hir::def_id::LocalDefId;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::LOCAL_CRATE;
use serde_json::{Value, json};
use std::path::Path;

/// Directory the documents are written to; setting it selects this mode.
pub const DIR_ENV: &str = "RSTYLE_CPG_DIR";
/// Workspace root that file paths are relative to.
pub const ROOT_ENV: &str = "RSTYLE_CPG_ROOT";

/// Writes the crate's documents into `dir`: every function the compiler holds, a macro's
/// included, as a capture records them. What a consumer reads of them is core's to decide.
pub fn emit(tcx: TyCtxt<'_>, root: &Path, dir: &Path) -> Result<(), String> {
    let name = unit_name(tcx);
    let encoding = rstyle_core::cpg::transport::Encoding::from_environment()?;
    let documents = documents_in(tcx, (root, None), None);

    for (index, (document, unrecorded)) in documents.enumerate() {
        if let Some((id, reason)) = unrecorded.first() {
            return Err(format!("{}: {reason}", tcx.def_path_str(id.to_def_id())));
        }
        let path = dir.join(format!("{name}.{index:06}{}", encoding.suffix()));
        rstyle_core::cpg::transport::write_json(&path, encoding, &document)?;
    }
    Ok(())
}

// rstyle-allow(side_effect_density): documents are produced lazily, one batch at a time
/// The crate's function documents, each recording the capture `session` when given:
/// the compiler invocation, the selected unit, and each file's path.
pub fn documents_in<'tcx, 'r>(
    tcx: TyCtxt<'tcx>,
    (root, package): (&'r Path, Option<&'r PackageContext>),
    session: Option<Value>,
) -> impl Iterator<Item = (Value, Vec<(LocalDefId, String)>)> + use<'tcx, 'r> {
    let package = cargo_package(package);
    batches(tcx)
        .into_iter()
        .enumerate()
        .map(move |(index, batch)| {
            let origin = (root, package.clone(), session.clone());
            // The first document records every file of the unit.
            batch_document(tcx, (origin, index == 0), &batch)
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
    let digest = rstyle_capture::checksum(manifest.as_bytes());

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
    for id in function_owners(tcx) {
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
    ((root, package, session), whole): (Origin<'_>, bool),
    owners: &[LocalDefId],
) -> (Value, Vec<(LocalDefId, String)>) {
    let mut unit = unit::Unit::new(tcx, root.to_path_buf());
    // The first document records the whole unit: every file, namespace and import.
    if whole {
        unit.record_local_files();
    }
    let mut unrecorded = Vec::new();
    let mut recorded = Vec::new();
    for &id in owners {
        match function(&mut unit, id) {
            Ok(function) => recorded.push(function),
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

    (document, unrecorded)
}

fn unit_name(tcx: TyCtxt<'_>) -> String {
    format!(
        "{}-{:016x}",
        tcx.crate_name(LOCAL_CRATE),
        tcx.stable_crate_id(LOCAL_CRATE).as_u64()
    )
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
    crate::paths::source_path(tcx, &file, root).map(|path| format!("file:{path}"))
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
            "configuration": configuration(tcx),
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
    let span = written_span(span, &signature["span"], (sig, body), &builder.nodes);
    let places = places::places(tcx, id, &mut builder);
    let implements = tcx
        .opt_associated_item(id.to_def_id())
        .and_then(|item| item.trait_item_def_id())
        .map(|trait_fn| builder.unit.definition(trait_fn));
    let definition = builder.unit.definition(id.to_def_id());
    let (cfg, accesses) = control_flow(&builder, &root, &places);

    let mut function = json!({
        "id": format!("fn:{}", crate::ids::def_id_str(tcx, id.to_def_id())), "definition": definition,
        "name": tcx.def_path_str(id.to_def_id()), "span": span, "signature": signature, "root": root,
        "nodes": builder.nodes, "cfg": cfg, "symbols": builder.symbols,
        "accesses": accesses, "refs": builder.refs, "places": places, "values": [],
    });
    if let Some(implements) = implements {
        function["implements"] = json!(implements);
    }
    Ok(function)
}

/// The function's control flow and who reads and writes its bindings.
fn control_flow(
    builder: &body::Builder<'_, '_>,
    root: &str,
    places: &[Value],
) -> (Value, Vec<Value>) {
    let cfg = flow::cfg(
        &builder.nodes,
        flow::Inputs {
            root,
            diverging: &builder.diverging,
            refs: &builder.refs,
        },
    );
    let accesses = accesses::accesses(&accesses::Records {
        nodes: &builder.nodes,
        symbols: &builder.symbols,
        refs: &builder.refs,
        places,
    });

    (cfg, accesses)
}

/// Where a function is written. A macro that wraps a function keeps its author's signature
/// and body under its own root, whose span is the macro's: the function then runs from
/// its signature to the last code its author wrote.
fn written_span(
    span: Value,
    signature: &Value,
    (sig, body): (&rustc_hir::FnSig<'_>, &rustc_hir::Body<'_>),
    nodes: &[Value],
) -> Value {
    if sig.span.from_expansion() || !body.value.span.from_expansion() {
        return span;
    }
    let authored = nodes
        .iter()
        .filter(|node| node["provenance"]["origin"] == "source" && node.get("synthetic").is_none())
        .map(|node| &node["span"])
        .filter(|written| {
            written["file"] == signature["file"]
                && written["start"].as_u64() >= signature["start"].as_u64()
        })
        .filter_map(|written| written["end"].as_u64());
    let end = authored.chain(signature["end"].as_u64()).max();

    let mut written = signature.clone();
    written["end"] = json!(end);
    written
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
    let (parameters, receiver) = parameters(builder, (sig, body), fn_sig, has_self);

    let mut signature = json!({"parameters": parameters, "result": builder.unit.ty(fn_sig.output()),
                               "generics": generics(builder.unit, id), "lifetimes": lifetimes(builder.unit, id),
                               "constants": constants(builder.unit, id),
                               "outlives": [], "modifiers": fn_modifiers(sig.header)});
    if let Some(provenance) = declared_provenance(builder, sig.span) {
        signature["provenance"] = provenance;
    }
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

// rstyle-allow(mutated_targets, mutation_per_target): records each parameter through the builder that accumulates the document
/// The parameters in order and the receiver, each with its symbol, type and spans.
fn parameters<'tcx>(
    builder: &mut body::Builder<'_, 'tcx>,
    (sig, body): (&'tcx rustc_hir::FnSig<'tcx>, &'tcx rustc_hir::Body<'tcx>),
    fn_sig: rustc_middle::ty::FnSig<'tcx>,
    has_self: bool,
) -> (Vec<Value>, Option<Value>) {
    let tcx = builder.unit.tcx;
    let mut parameters = Vec::new();
    let mut receiver = None;
    for (index, param) in body.params.iter().enumerate() {
        let (symbol, bindings) = builder.parameter(param.pat, index);
        let ty = builder.unit.ty(fn_sig.inputs()[index]);
        let declared = declared_provenance(builder, param.pat.span);
        if has_self && index == 0 {
            let annotation = sig.decl.inputs.first().map(|ty| ty.span);
            let mut record = json!({"symbol": symbol, "type": ty});
            if let Some(declared) = declared {
                record["provenance"] = declared;
            }
            receiver = Some(annotated(builder.unit, record, annotation));
            continue;
        }
        let position = index - usize::from(has_self);
        let mut record = json!({"position": position, "symbol": symbol, "type": ty, "bindings": bindings,
                            "pattern": builder.id(param.pat.hir_id),
                            "original_kind": compiler::original_kind(tcx, param.pat.hir_id)});
        if let Some(declared) = declared {
            record["provenance"] = declared;
        }
        let annotation = sig.decl.inputs.get(index).map(|ty| ty.span);
        parameters.push(located_parameter(
            builder.unit,
            record,
            (param.pat.span, annotation),
        ));
    }

    (parameters, receiver)
}

/// Where a parameter is declared, when a macro expansion or a desugaring writes it.
fn declared_provenance(
    builder: &mut body::Builder<'_, '_>,
    span: rustc_span::Span,
) -> Option<Value> {
    let provenance = builder.provenance(span);
    (provenance["origin"] != "source").then_some(provenance)
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

fn configuration(tcx: TyCtxt<'_>) -> Vec<String> {
    let cfgs: Vec<String> = tcx
        .sess
        .config
        .iter()
        .map(|(name, value)| {
            value.map_or_else(|| name.to_string(), |value| format!("{name}=\"{value}\""))
        })
        .collect();
    let mut cfgs = cfgs;
    cfgs.sort();
    cfgs
}

fn function_owners(tcx: TyCtxt<'_>) -> impl Iterator<Item = LocalDefId> {
    tcx.hir_body_owners().filter(move |id| {
        matches!(
            tcx.def_kind(*id),
            rustc_hir::def::DefKind::Fn | rustc_hir::def::DefKind::AssocFn
        )
    })
}
