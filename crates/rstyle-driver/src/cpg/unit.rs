//! Unit tables of an emitted `compat.cpg` document: files, definitions and types,
//! interned as functions reference them.

use crate::ids;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, Ty, TyCtxt, consts::ConstExt};
use rustc_span::{FileName, Span};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub struct Unit<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub root: PathBuf,
    files: BTreeMap<String, Value>,
    definitions: BTreeMap<String, Value>,
    types: BTreeMap<String, Value>,
}

impl<'tcx> Unit<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, root: PathBuf) -> Self {
        Self {
            tcx,
            root,
            files: BTreeMap::new(),
            definitions: BTreeMap::new(),
            types: BTreeMap::new(),
        }
    }

    /// A span of a workspace source file, as file-relative byte offsets.
    pub fn span(&mut self, span: Span) -> Option<Value> {
        let source_map = self.tcx.sess.source_map();
        let (lo, hi) = (
            source_map.lookup_byte_offset(span.lo()),
            source_map.lookup_byte_offset(span.hi()),
        );
        if lo.sf.start_pos != hi.sf.start_pos || !matches!(lo.sf.name, FileName::Real(_)) {
            return None;
        }
        let path = crate::extract::source_path(self.tcx, &lo.sf, &self.root)?;
        let content = lo.sf.src.as_ref()?;
        let id = format!("file:{path}");
        self.files.entry(id.clone()).or_insert_with(|| {
            let digest: String = Sha256::digest(content.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            json!({"id": id, "path": path, "digest": format!("sha256:{digest}"), "origin": "source",
                   "content": content.as_str()})
        });

        Some(json!({"file": id, "start": lo.pos.0, "end": hi.pos.0}))
    }

    /// `span` without parentheses that enclose all of it, and the whitespace inside them.
    pub fn without_parentheses(&self, mut span: Value) -> Value {
        let Some(content) = self
            .files
            .get(span["file"].as_str().unwrap_or_default())
            .and_then(|f| f["content"].as_str())
        else {
            return span;
        };
        let bytes = content.as_bytes();
        loop {
            let (start, end) = (
                span["start"].as_u64().unwrap() as usize,
                span["end"].as_u64().unwrap() as usize,
            );
            if end < start + 2
                || bytes[start] != b'('
                || bytes[end - 1] != b')'
                || closing(bytes, start) != Some(end - 1)
            {
                return span;
            }
            let inner = &content[start + 1..end - 1];
            let leading = inner.len() - inner.trim_start().len();
            let trailing = inner.len() - inner.trim_end().len();
            span["start"] = json!(start + 1 + leading);
            span["end"] = json!(end - 1 - trailing);
        }
    }

    /// The definition ID of `def_id`, recording the definition on first use.
    pub fn definition(&mut self, def_id: DefId) -> String {
        let stable = ids::def_id_str(self.tcx, def_id);
        let id = format!("def:{stable}");
        if !self.definitions.contains_key(&id) {
            let record = self.definition_record(def_id, &id, stable);
            self.definitions.insert(id.clone(), record);
            self.relations(def_id, &id);
            if matches!(self.tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn) {
                let callable = self.callable(def_id);
                self.definitions.get_mut(&id).unwrap()["callable"] = callable;
            }
        }
        id
    }

    // rstyle-allow(mutation_per_target): the unit interns referenced records as it builds them
    /// What a call to a function or method sees: its owner type, receiver, parameter
    /// and result types, own type parameter count and modifiers.
    fn callable(&mut self, def_id: DefId) -> Value {
        let tcx = self.tcx;
        let signature = tcx.fn_sig(def_id).instantiate_identity().skip_binder();
        let inputs = signature.inputs();
        let has_self = tcx
            .opt_associated_item(def_id)
            .is_some_and(|item| item.is_method());
        let foreign = tcx.def_kind(tcx.parent(def_id)) == DefKind::ForeignMod;
        let modifiers: Vec<&str> = [
            (tcx.asyncness(def_id).is_async(), "async"),
            (signature.safety().is_unsafe(), "unsafe"),
            (tcx.is_const_fn(def_id), "const"),
            (foreign, "foreign"),
            (signature.abi() != rustc_abi::ExternAbi::Rust, "rust:foreign-abi"),
        ]
        .into_iter()
        .filter_map(|(present, modifier)| present.then_some(modifier))
        .collect();
        let mut record = json!({
            "parameters": self.types_of(inputs.iter().skip(usize::from(has_self)).copied()),
            "result": self.ty(signature.output()),
            "generics": tcx.generics_of(def_id).own_counts().types,
            "modifiers": modifiers,
        });
        if let Some(receiver) = inputs.first().filter(|_| has_self) {
            record["receiver"] = json!(self.ty(*receiver));
        }
        if let Some(implementation) = tcx.impl_of_assoc(def_id) {
            let owner = tcx
                .type_of(implementation)
                .instantiate_identity()
                .skip_norm_wip();
            record["owner_type"] = json!(self.ty(owner));
        }

        record
    }

    fn definition_record(&self, def_id: DefId, id: &str, stable: String) -> Value {
        let tcx = self.tcx;
        let kind = tcx.def_kind(def_id);
        let scope = if def_id.is_local() {
            "unit"
        } else if ids::in_workspace(tcx, def_id, &self.root) {
            "workspace"
        } else {
            "library"
        };
        let compilation = format!("{:016x}", tcx.stable_crate_id(def_id.krate).as_u64());
        let mut record = json!({"id": id, "kind": definition_kind(tcx, def_id, kind), "name": tcx.def_path_str(def_id),
               "stable_identity": stable, "scope": scope, "modifiers": modifiers(tcx, def_id, kind),
               "compilation": compilation});
        if super::compiler::standard(tcx, def_id) {
            record["standard"] = json!(true);
        }
        if matches!(kind, DefKind::Struct | DefKind::Enum | DefKind::Union)
            && tcx.adt_destructor(def_id).is_some()
        {
            record["destructor"] = json!(true);
        }
        record
    }

    /// The item an associated item belongs to, and the interface member it implements.
    fn relations(&mut self, def_id: DefId, id: &str) {
        let Some(item) = self.tcx.opt_associated_item(def_id) else {
            return;
        };
        let owner = self.definition(self.tcx.parent(def_id));
        let implements = item.trait_item_def_id().map(|member| self.definition(member));
        let record = self.definitions.get_mut(id).unwrap();
        record["owner"] = json!(owner);
        if let Some(implements) = implements {
            record["implements"] = json!(implements);
        }
    }

    /// A placeholder target for a call through a computed value.
    pub fn value_definition(&mut self) -> String {
        let id = "def:<value>".to_string();
        self.definitions.entry(id.clone()).or_insert_with(|| {
            json!({"id": id, "kind": "other", "name": "<value>", "stable_identity": "<value>", "scope": "unit"})
        });
        id
    }

    /// The type ID of `ty`, recording a structured type on first use.
    pub fn ty(&mut self, ty: Ty<'tcx>) -> String {
        let spelling = ty.to_string();
        let id = type_id(&spelling, ty);
        if !self.types.contains_key(&id) {
            self.types.insert(id.clone(), json!({}));
            let mut record = self.type_record(ty);
            record["id"] = json!(id);
            record["spelling"] = json!(spelling);
            if let Some(key) = crate::hir_extraction::runtime_bounds::sysroot::type_key(self.tcx, ty) {
                record["key"] = json!(format!("rust:{key}"));
            }
            self.types.insert(id.clone(), record);
        }
        id
    }

    /// IDs of the records `intern` makes for the items, in order.
    fn interned<T>(
        &mut self,
        items: impl IntoIterator<Item = T>,
        intern: fn(&mut Self, T) -> String,
    ) -> Vec<String> {
        items.into_iter().map(|item| intern(self, item)).collect()
    }

    fn types_of(&mut self, types: impl IntoIterator<Item = Ty<'tcx>>) -> Vec<String> {
        self.interned(types, Self::ty)
    }

    /// Definition IDs, sorted and without repeats.
    fn definitions_of(&mut self, def_ids: impl IntoIterator<Item = DefId>) -> Vec<String> {
        let mut ids = self.interned(def_ids, Self::definition);
        ids.sort();
        ids.dedup();
        ids
    }

    // rstyle-allow(mutation_per_target): the unit interns referenced records as it builds them
    fn type_record(&mut self, ty: Ty<'tcx>) -> Value {
        match *ty.kind() {
            ty::Adt(adt, args) => self.nominal(ty, adt, args),
            ty::FnPtr(..) | ty::FnDef(..) => {
                let signature = ty.fn_sig(self.tcx).skip_binder();
                let parameters = self.types_of(signature.inputs().iter().copied());
                let mut record = json!({"kind": "function", "parameters": parameters,
                                        "result": self.ty(signature.output())});
                if let ty::FnDef(def_id, _) = *ty.kind() {
                    record["definition"] = json!(self.definition(def_id));
                }
                let modifiers: Vec<&str> = [
                    (signature.safety().is_unsafe(), "unsafe"),
                    (signature.abi() != rustc_abi::ExternAbi::Rust, "rust:foreign-abi"),
                ]
                .into_iter()
                .filter_map(|(present, modifier)| present.then_some(modifier))
                .collect();
                if !modifiers.is_empty() {
                    record["modifiers"] = json!(modifiers);
                }
                record
            }
            ty::Closure(_, args) => {
                let signature = args.as_closure().sig().skip_binder();
                let parameters = self.types_of(signature.inputs().iter().copied());
                json!({"kind": "closure", "parameters": parameters, "result": self.ty(signature.output())})
            }
            ty::Ref(_, target, mutability) => {
                let mode = if mutability.is_mut() {
                    "exclusive"
                } else {
                    "shared"
                };
                json!({"kind": "reference", "target": self.ty(target), "mode": mode})
            }
            ty::RawPtr(target, _) => {
                json!({"kind": "pointer", "target": self.ty(target), "mode": "raw"})
            }
            ty::Tuple(..) | ty::Slice(..) | ty::Array(..) => self.aggregate(ty),
            ty::Dynamic(predicates, ..) => self.interface(predicates),
            ty::Alias(
                _,
                ty::AliasTy {
                    kind: ty::AliasTyKind::Opaque { def_id },
                    ..
                },
            ) => self.opaque(def_id),
            ty::Param(param) => json!({"kind": "parameter", "index": param.index}),
            _ => self.scalar(ty),
        }
    }

    /// A struct, enum or union with its type arguments; `Box` is an owned pointer.
    fn nominal(
        &mut self,
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        args: ty::GenericArgsRef<'tcx>,
    ) -> Value {
        let definition = self.definition(adt.did());
        let arguments = self.types_of(args.types());
        match ty.boxed_ty() {
            Some(target) => json!({"kind": "pointer", "mode": "owned", "target": self.ty(target),
                                   "definition": definition, "arguments": arguments}),
            None => {
                let kind = if adt.is_enum() { "sum" } else { "record" };
                json!({"kind": kind, "definition": definition, "arguments": arguments})
            }
        }
    }

    // rstyle-allow(mutation_per_target): the unit interns referenced records as it builds them
    /// Tuples, slices and arrays.
    fn aggregate(&mut self, ty: Ty<'tcx>) -> Value {
        match *ty.kind() {
            ty::Tuple(items) if items.is_empty() => json!({"kind": "unit"}),
            ty::Tuple(items) => json!({"kind": "tuple", "items": self.types_of(items.iter())}),
            ty::Array(element, length) => {
                let mut record =
                    json!({"kind": "sequence", "element": self.ty(element), "fixed": true});
                if let Some(length) = length.try_to_target_usize(self.tcx) {
                    record["length"] = json!(length);
                }
                record
            }
            ty::Slice(element) => json!({"kind": "sequence", "element": self.ty(element)}),
            _ => json!({"kind": "opaque"}),
        }
    }

    /// `impl Trait`: the hidden type, bounded by its traits.
    fn opaque(&mut self, def_id: DefId) -> Value {
        let bounds = self.tcx.explicit_item_bounds(def_id).skip_binder().iter();
        let traits: Vec<DefId> = bounds
            .filter_map(|(clause, _)| clause.as_trait_clause().map(|t| t.def_id()))
            .collect();
        match self.definitions_of(traits) {
            bounds if bounds.is_empty() => json!({"kind": "opaque"}),
            bounds => json!({"kind": "opaque", "bounds": bounds}),
        }
    }

    /// `dyn Trait`: its principal trait, and every trait bounding it.
    fn interface(
        &mut self,
        predicates: &'tcx ty::List<ty::PolyExistentialPredicate<'tcx>>,
    ) -> Value {
        let principal = predicates.principal_def_id();
        let bounds = self.definitions_of(principal.into_iter().chain(predicates.auto_traits()));

        if bounds.is_empty() {
            return json!({"kind": "opaque"});
        }
        let mut record = json!({"kind": "interface", "bounds": bounds});
        if let Some(principal) = principal {
            record["definition"] = json!(self.definition(principal));
        }
        record
    }

    fn scalar(&self, ty: Ty<'tcx>) -> Value {
        let pointer_width = self.tcx.sess.target.pointer_width;
        let integer = |bits: Option<u64>, signed: bool| match bits {
            Some(bits) => json!({"kind": "integer", "bits": bits, "signed": signed}),
            None => {
                json!({"kind": "integer", "bits": pointer_width, "signed": signed, "pointer_sized": true})
            }
        };
        match ty.kind() {
            ty::Int(int) => integer(int.bit_width(), true),
            ty::Uint(uint) => integer(uint.bit_width(), false),
            ty::Float(float) => json!({"kind": "float", "bits": float.bit_width()}),
            ty::Bool => json!({"kind": "bool"}),
            ty::Char => json!({"kind": "char"}),
            ty::Str => json!({"kind": "string"}),
            ty::Never => json!({"kind": "bottom"}),
            _ => json!({"kind": "opaque"}),
        }
    }

    pub fn tables(self) -> (Vec<Value>, Vec<Value>, Vec<Value>) {
        (
            self.files.into_values().collect(),
            self.definitions.into_values().collect(),
            self.types.into_values().collect(),
        )
    }
}

/// A type's ID: its spelling, and for a type mentioning type parameters their indices,
/// since the same spelling names different parameters in different functions.
fn type_id(spelling: &str, ty: Ty<'_>) -> String {
    let parameters: BTreeSet<String> = ty
        .walk()
        .filter_map(|arg| arg.as_type())
        .filter_map(|ty| match ty.kind() {
            ty::Param(param) => Some(format!("{}={}", param.name, param.index)),
            _ => None,
        })
        .collect();
    if parameters.is_empty() {
        return format!("type:{spelling}");
    }
    let parameters: Vec<String> = parameters.into_iter().collect();
    format!("type:{spelling}#{}", parameters.join(","))
}

fn definition_kind(tcx: TyCtxt<'_>, def_id: DefId, kind: DefKind) -> &'static str {
    match kind {
        DefKind::Fn => "function",
        DefKind::AssocFn if tcx.associated_item(def_id).is_method() => "method",
        DefKind::AssocFn => "function",
        DefKind::Static { .. } => "static",
        DefKind::Const | DefKind::AssocConst | DefKind::ConstParam => "constant",
        DefKind::Struct | DefKind::Enum | DefKind::Union | DefKind::TyAlias => "type",
        DefKind::Trait => "trait",
        DefKind::Variant => "variant",
        DefKind::Field => "field",
        DefKind::Mod => "module",
        DefKind::TyParam => "type_parameter",
        _ => "other",
    }
}

fn modifiers(tcx: TyCtxt<'_>, def_id: DefId, kind: DefKind) -> Vec<&'static str> {
    match kind {
        DefKind::Fn | DefKind::AssocFn
            if tcx
                .fn_sig(def_id)
                .skip_binder()
                .skip_binder()
                .safety()
                .is_unsafe() =>
        {
            vec!["unsafe"]
        }
        DefKind::Static { mutability, .. } if mutability.is_mut() => vec!["mutable"],
        _ => vec![],
    }
}

/// Coverage this producer declares; every partial layer says what it leaves out.
pub fn coverage() -> Value {
    let declare =
        |status: &str, description: &str| json!({"status": status, "description": description});
    json!({
        "capabilities": {
            "resolution": declare("approximated", "Paths and calls are resolved by rustc; built-in operators carry no references"),
            "dispatch": declare("supported", "Static, method, dynamic and function-value dispatch from typeck; selected trait implementations from instance resolution"),
            "generic-instances": declare("unsupported", "Instances are not recorded"),
            "lifetimes": declare("unsupported", "Lifetimes are not emitted"),
            "types": declare("approximated", "Types are recorded for signatures, places and called values only"),
            "integer-semantics": declare("approximated", "Integer widths and signedness are recorded where types are"),
            "mutability": declare("supported", "Binding mutability from HIR binding modes"),
            "aliasing": declare("approximated", "Places come from rustc's expression-use visitor for writes and exclusive borrows only"),
            "implicit-calls": declare("unsupported", "Destructors and implicit conversions are not emitted"),
            "constant-values": declare("unsupported", "Constant values are not emitted"),
            "evaluation-order": declare("unknown", "No control flow is emitted"),
            "exceptional-flow": declare("unsupported", "No control flow is emitted"),
            "macro-provenance": declare("supported", "Macro-expanded and desugared nodes carry their outermost call site"),
        },
        "layers": {
            "unit": declare("supported", "Unit files with content, target and edition"),
            "method": declare("approximated", "Identity, spans, parameters and receiver; generics and lifetimes are not emitted"),
            "ast": declare("supported", "The HIR tree with for, while and ? restored to their source shape"),
            "cfg": declare("unsupported", "No control flow is emitted"),
            "symbol": declare("approximated", "Bindings without access records"),
            "ref": declare("approximated", "Paths, calls and method calls"),
            "type": declare("approximated", "Signature, place and called-value types"),
            "value": declare("unsupported", "No compile-time values are emitted"),
            "place": declare("approximated", "Writes and exclusive borrows from rustc's expression-use visitor"),
            "evidence": declare("approximated", "Coverage declarations only"),
            "extension": declare("supported", "Reference patterns use rust:reference-pattern"),
        },
    })
}

/// Offset of the parenthesis closing the one at `open`, ignoring string contents.
fn closing(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut index = open;
    while index < bytes.len() {
        match (bytes[index], in_string) {
            (b'\\', true) => index += 1,
            (b'"', _) => in_string = !in_string,
            (b'(', false) => depth += 1,
            (b')', false) => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }

    None
}
