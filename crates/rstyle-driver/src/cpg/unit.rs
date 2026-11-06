//! Unit tables of an emitted `compat.cpg` document: files, definitions and types,
//! interned as functions reference them.

use crate::ids;
use rustc_hir::def::DefKind;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, Ty, TyCtxt, TypeVisitableExt, consts::ConstExt};
use rustc_span::{FileName, Span};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub struct Unit<'tcx> {
    pub tcx: TyCtxt<'tcx>,
    pub root: PathBuf,
    files: BTreeMap<String, Value>,
    /// Each file's path as the compiler opened it, made absolute.
    local_paths: BTreeMap<String, String>,
    /// The unit's own value item paths, computed once.
    value_paths: Option<std::collections::HashSet<String>>,
    definitions: BTreeMap<String, Value>,
    types: BTreeMap<String, Value>,
    /// The compile-time values of local constants the unit references, by definition.
    values: BTreeMap<String, Value>,
}

impl<'tcx> Unit<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, root: PathBuf) -> Self {
        Self {
            tcx,
            root,
            files: BTreeMap::new(),
            local_paths: BTreeMap::new(),
            value_paths: None,
            definitions: BTreeMap::new(),
            types: BTreeMap::new(),
            values: BTreeMap::new(),
        }
    }

    /// A definition's stable identity: its path, with `!` for a macro and `#type` for a
    /// type that shares its path with one of the unit's value items.
    pub(super) fn stable_identity(&mut self, def_id: DefId) -> String {
        let mut stable = ids::def_id_str(self.tcx, def_id);
        // A macro shares its path with an item of another namespace, as derive macros
        // share the traits they implement; `!` names the macro, as invoking it does.
        if matches!(self.tcx.def_kind(def_id), DefKind::Macro(_)) {
            stable.push('!');
        }
        // A type may share its path with a function of the value namespace, as a braced
        // struct and a function of its name do; `#type` names the type.
        let type_namespace = matches!(
            self.tcx.def_kind(def_id),
            DefKind::Struct | DefKind::Enum | DefKind::Union | DefKind::TyAlias | DefKind::Trait
        );
        let namesake = match def_id.is_local() {
            true => self.value_paths().contains(&stable),
            false => self.foreign_value_namesake(def_id),
        };
        if type_namespace && namesake {
            stable.push_str("#type");
        }

        stable
    }

    /// Whether another crate's item shares its name with a function, constant or static
    /// of its module, as `syn::token::Or` does with the function `Token![|]` calls.
    fn foreign_value_namesake(&self, def_id: DefId) -> bool {
        let tcx = self.tcx;
        let Some(parent) = tcx.opt_parent(def_id) else {
            return false;
        };
        if tcx.def_kind(parent) != DefKind::Mod {
            return false;
        }
        let Some(name) = tcx.opt_item_name(def_id) else {
            return false;
        };

        tcx.module_children(parent).iter().any(|child| {
            child.ident.name == name
                && matches!(
                    child.res,
                    rustc_hir::def::Res::Def(
                        DefKind::Fn | DefKind::Const | DefKind::Static { .. },
                        _
                    )
                )
        })
    }

    /// The paths of the unit's own value items: functions, constants and statics.
    fn value_paths(&mut self) -> &std::collections::HashSet<String> {
        let tcx = self.tcx;
        self.value_paths.get_or_insert_with(|| {
            tcx.iter_local_def_id()
                .filter(|id| {
                    matches!(
                        tcx.def_kind(*id),
                        DefKind::Fn | DefKind::Const | DefKind::Static { .. }
                    )
                })
                .map(|id| ids::def_id_str(tcx, id.to_def_id()))
                .collect()
        })
    }

    /// Each recorded file's absolute path, as the compiler opened it.
    pub fn local_paths(&self) -> BTreeMap<String, String> {
        self.local_paths.clone()
    }

    /// Records every local source file the compiler loaded, as the unit's files.
    pub fn record_local_files(&mut self) {
        let files = self.tcx.sess.source_map().files().clone();
        for file in files.iter() {
            if file.cnum == rustc_hir::def_id::LOCAL_CRATE && matches!(file.name, FileName::Real(_))
            {
                self.span(rustc_span::Span::with_root_ctxt(
                    file.start_pos,
                    file.start_pos,
                ));
            }
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
        let path = crate::paths::document_path(self.tcx, &lo.sf, &self.root)?;
        let id = format!("file:{path}");
        // A file is recorded as written: its original bytes, and offsets into them.
        if !self.files.contains_key(&id) {
            let content = super::source::recover(&lo.sf)?;
            let digest: String = Sha256::digest(content.as_bytes())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            let mut record = json!({"id": id, "path": path, "digest": format!("sha256:{digest}"),
                                    "origin": "source", "content": content});
            if crate::paths::build_output(self.tcx, &lo.sf) {
                record["origin"] = json!("generated");
                record["generated_by"] = json!("build script");
            }
            self.files.insert(id.clone(), record);
            if let FileName::Real(real) = &lo.sf.name
                && let Some(local) = real.local_path()
                && let Ok(directory) = std::env::current_dir()
            {
                let absolute = directory.join(local);
                self.local_paths
                    .insert(id.clone(), absolute.to_string_lossy().into_owned());
            }
        }
        let start = lo.sf.original_relative_byte_pos(span.lo()).0;
        let end = hi.sf.original_relative_byte_pos(span.hi()).0;

        Some(json!({"file": id, "start": start, "end": end}))
    }

    /// `span` without parentheses that enclose all of it, and the whitespace inside them,
    /// with how many pairs enclosed it.
    /// The text a recorded span covers.
    pub fn text(&self, span: &Value) -> Option<&str> {
        let content = self.files.get(span["file"].as_str()?)?["content"].as_str()?;
        let start = usize::try_from(span["start"].as_u64()?).ok()?;
        let end = usize::try_from(span["end"].as_u64()?).ok()?;

        content.get(start..end)
    }

    /// A binding's name as it is written, so that a raw identifier keeps its `r#`.
    pub fn written_name(&mut self, ident: rustc_span::Ident) -> String {
        let written = self.span(ident.span);
        let name = ident.name.as_str();
        written
            .as_ref()
            .and_then(|span| self.text(span))
            .filter(|text| text.strip_prefix("r#") == Some(name))
            .unwrap_or(name)
            .to_owned()
    }

    pub fn without_parentheses(&self, mut span: Value) -> (Value, usize) {
        let Some(content) = self
            .files
            .get(span["file"].as_str().unwrap_or_default())
            .and_then(|f| f["content"].as_str())
        else {
            return (span, 0);
        };
        let (start, end, levels) = crate::spans::unparenthesized(
            content,
            span["start"].as_u64().unwrap_or_default() as usize,
            span["end"].as_u64().unwrap_or_default() as usize,
        );
        span["start"] = json!(start);
        span["end"] = json!(end);

        (span, levels)
    }

    /// A definition's ID from its stable identity. Another crate of the unit's own name,
    /// as a binary's library is, is told apart by its crate identity.
    fn definition_id(&self, def_id: DefId, stable: &str) -> String {
        let tcx = self.tcx;
        let namesake = !def_id.is_local()
            && tcx.crate_name(def_id.krate) == tcx.crate_name(rustc_span::def_id::LOCAL_CRATE);
        match namesake {
            true => format!(
                "def:{stable}@{:016x}",
                tcx.stable_crate_id(def_id.krate).as_u64()
            ),
            false => format!("def:{stable}"),
        }
    }

    // rstyle-allow(mutated_targets, mutation_per_target): the unit interns referenced records as it builds them
    /// The definition ID of `def_id`, recording the definition on first use.
    pub fn definition(&mut self, def_id: DefId) -> String {
        let stable = self.stable_identity(def_id);
        let id = self.definition_id(def_id, &stable);
        if !self.definitions.contains_key(&id) {
            let mut record = self.definition_record(def_id, &id, stable);
            if let Some(span) = self.declaration(def_id) {
                record["span"] = span;
            }
            if let Some(generated) = self.generated_declaration(def_id) {
                record["extensions"]["rust:generated-declaration"] = generated;
            }
            if let Some(copy) = self.builtin_copy(def_id) {
                record["extensions"]["rust:builtin-copy"] = copy;
            }
            // An inherent method names its implementation, recorded as a definition too.
            // A local function's name as written, where its identifier is.
            if matches!(self.tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn)
                && def_id.is_local()
                && let Some(name) = self
                    .tcx
                    .def_ident_span(def_id)
                    .and_then(|span| self.span(span.source_callsite()))
            {
                record["extensions"]["rust:compiler-identity"]["ident_span"] = name;
            }
            if let Some(implementation) = self.inherent_implementation(def_id) {
                record["extensions"]["rust:compiler-identity"]["inherent_impl"] =
                    json!(implementation);
            }
            // A constructor's or variant's enum and variant, which name the value it builds.
            if let Some((owner, variant, definition)) = self.variant_of(def_id) {
                let identity = &mut record["extensions"]["rust:compiler-identity"];
                identity["enum"] = json!(owner);
                identity["variant"] = json!(variant);
                if let Some(definition) = definition {
                    identity["variant_definition"] = json!(definition);
                }
            }
            self.definitions.insert(id.clone(), record);
            self.constant_value(def_id, &id);
            self.relations(def_id, &id);
            if matches!(self.tcx.def_kind(def_id), DefKind::Fn | DefKind::AssocFn) {
                let callable = self.callable(def_id);
                self.definitions.get_mut(&id).unwrap()["callable"] = callable;
            }
        }
        id
    }

    /// A local type's declaration as written, unless a macro generated it.
    /// The enum a variant or variant constructor belongs to, and the variant's name.
    /// A constructor's variant definition is recorded too.
    fn variant_of(&mut self, def_id: DefId) -> Option<(String, String, Option<String>)> {
        let tcx = self.tcx;
        let (variant, constructed) = match tcx.def_kind(def_id) {
            DefKind::Ctor(rustc_hir::def::CtorOf::Variant, _) => (tcx.parent(def_id), true),
            DefKind::Variant => (def_id, false),
            _ => return None,
        };
        let owner = self.definition(tcx.parent(variant));
        let definition = constructed.then(|| self.definition(variant));

        Some((owner, tcx.item_name(variant).to_string(), definition))
    }

    /// The inherent implementation a local associated function belongs to.
    fn inherent_implementation(&mut self, def_id: DefId) -> Option<String> {
        if !def_id.is_local()
            || !matches!(
                self.tcx.def_kind(def_id),
                DefKind::AssocFn | DefKind::AssocConst
            )
        {
            return None;
        }
        let implementation = self.tcx.inherent_impl_of_assoc(def_id)?;
        Some(self.definition(implementation))
    }

    /// A local type's or implementation's declaration as written, unless a macro generated it.
    fn declaration(&mut self, def_id: DefId) -> Option<Value> {
        let local = def_id.as_local()?;
        if !matches!(
            self.tcx.def_kind(def_id),
            DefKind::Fn
                | DefKind::Struct
                | DefKind::Enum
                | DefKind::Union
                | DefKind::Impl { .. }
                | DefKind::Const
                | DefKind::AssocConst
        ) {
            return None;
        }
        let span = match self.tcx.hir_node_by_def_id(local) {
            rustc_hir::Node::Item(item) => item.span,
            rustc_hir::Node::ImplItem(item) => item.span,
            _ => return None,
        };
        (!span.from_expansion()).then(|| self.span(span)).flatten()
    }

    /// What a local constant's body calls, when it is only a call with no arguments:
    /// the callee and the type arguments the call instantiates it with.
    fn initializer(&mut self, def_id: DefId) -> Option<Value> {
        use rustc_hir::{ExprKind, def::Res};
        let local = def_id.as_local()?;
        let body = self.tcx.hir_maybe_body_owned_by(local)?;
        let ExprKind::Call(callee, []) = body.value.kind else {
            return None;
        };
        let ExprKind::Path(ref path) = callee.kind else {
            return None;
        };
        let typeck = self.tcx.typeck(local);
        let Res::Def(_, function) = typeck.qpath_res(path, callee.hir_id) else {
            return None;
        };
        let arguments: Vec<String> = typeck
            .node_args(callee.hir_id)
            .types()
            .map(|ty| self.ty(ty))
            .collect();
        let generated = body.value.span.from_expansion();

        Some(
            json!({"callee": self.definition(function), "arguments": arguments, "generated": generated}),
        )
    }

    /// A local type whose only `Copy` implementation is the standard derive: the trait,
    /// the derive macro, and where the derive is written.
    fn builtin_copy(&mut self, def_id: DefId) -> Option<Value> {
        use rustc_span::hygiene::{ExpnKind, MacroKind};
        let tcx = self.tcx;
        if !def_id.is_local()
            || !matches!(
                tcx.def_kind(def_id),
                DefKind::Struct | DefKind::Enum | DefKind::Union
            )
        {
            return None;
        }
        let copy = tcx.lang_items().copy_trait()?;
        let self_ty = tcx.type_of(def_id).instantiate_identity().skip_norm_wip();
        let mut implementations = tcx
            .non_blanket_impls_for_ty(copy, self_ty)
            .filter(|id| id.is_local());
        let implementation = implementations.next()?;
        if implementations.next().is_some() || !tcx.is_automatically_derived(implementation) {
            return None;
        }
        let expansion = tcx.def_span(implementation).ctxt().outer_expn_data();
        let derive = expansion.macro_def_id?;
        let standard = !derive.is_local()
            && tcx.crate_name(derive.krate).as_str() == "core"
            && tcx.item_name(derive).as_str() == "Copy"
            && matches!(expansion.kind, ExpnKind::Macro(MacroKind::Derive, name) if name.as_str() == "Copy");
        if !standard {
            return None;
        }
        let span = self.span(expansion.call_site.source_callsite())?;

        Some(
            json!({"trait": self.definition(copy), "derive": self.definition(derive), "derive_span": span}),
        )
    }

    /// A local struct a macro generates: its declaration and field types as the macro
    /// writes them, and the original local macro invocation it comes from, if unique.
    fn generated_declaration(&mut self, def_id: DefId) -> Option<Value> {
        let local = def_id.as_local()?;
        if self.tcx.def_kind(def_id) != DefKind::Struct {
            return None;
        }
        let rustc_hir::Node::Item(item) = self.tcx.hir_node_by_def_id(local) else {
            return None;
        };
        let rustc_hir::ItemKind::Struct(_, generics, data) = item.kind else {
            return None;
        };
        if !item.span.from_expansion() {
            return None;
        }
        let shape = match data {
            rustc_hir::VariantData::Struct { .. } => "record",
            rustc_hir::VariantData::Tuple(..) => "tuple",
            rustc_hir::VariantData::Unit(..) => "unit",
        };
        let fields: Option<Vec<Value>> = data
            .fields()
            .iter()
            .map(|field| self.span(field.ty.span))
            .collect();
        let mut record =
            json!({"template": self.span(item.span)?, "shape": shape, "field_types": fields?});
        if !generics.params.is_empty() || !generics.predicates.is_empty() {
            record["generic"] = json!(true);
        }
        // The hygiene marks of the declaration's span, outermost expansion first.
        let marks: Vec<Value> = item
            .span
            .ctxt()
            .marks()
            .into_iter()
            .map(|(expansion, transparency)| {
                json!({"expansion": format!("{:?}", expansion.expn_hash()),
                       "transparency": format!("{transparency:?}")})
            })
            .collect();
        record["marks"] = json!(marks);
        if let Some(root) = super::generated_root::generated_root(self.tcx, local) {
            record["root"] = json!(root);
        }

        Some(record)
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
            (
                signature.abi() != rustc_abi::ExternAbi::Rust,
                "rust:foreign-abi",
            ),
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
        // A crate's root, which has no path of its own, is named by its crate.
        let name = match def_id.is_crate_root() {
            true => tcx.crate_name(def_id.krate).to_string(),
            false => tcx.def_path_str(def_id),
        };
        let mut record = json!({"id": id, "kind": definition_kind(tcx, def_id, kind), "name": name,
               "stable_identity": stable, "scope": scope, "modifiers": modifiers(tcx, def_id, kind),
               "compilation": compilation});
        for (field, value) in compiler_facts(tcx, def_id, kind) {
            record[field] = value;
        }
        record["extensions"] = json!({"rust:compiler-identity": rust_identity(tcx, def_id, kind)});

        record
    }

    /// The item an associated item belongs to, and the interface member it implements.
    fn relations(&mut self, def_id: DefId, id: &str) {
        let Some(item) = self.tcx.opt_associated_item(def_id) else {
            return;
        };

        let owner = self.definition(self.tcx.parent(def_id));
        let implements = item
            .trait_item_def_id()
            .map(|member| self.definition(member));
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
        let id = type_id(self.tcx, &spelling, ty);
        if !self.types.contains_key(&id) {
            self.types.insert(id.clone(), json!({}));
            let mut record = self.type_record(ty);
            record["id"] = json!(id);
            record["spelling"] = json!(spelling);
            if let Some(key) = super::sysroot::type_key(self.tcx, ty) {
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
            ty::FnPtr(..) | ty::FnDef(..) => self.function_type(ty),
            ty::Closure(def_id, args) => {
                let closure = args.as_closure();
                let signature = closure.sig().skip_binder();
                let parameters = self.types_of(signature.inputs().iter().copied());
                // The closure's environment, as the compiler stores its captures.
                let environment = self.ty(closure.tupled_upvars_ty());
                json!({"kind": "closure", "parameters": parameters, "result": self.ty(signature.output()),
                       "definition": self.definition(def_id), "environment": environment})
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
            ty::Alias(..) => self.alias(ty),
            ty::Param(param) => json!({"kind": "parameter", "index": param.index}),
            _ => self.scalar(ty),
        }
    }

    // rstyle-allow(mutation_per_target): the unit interns referenced records as it builds them
    /// A function item's or function pointer's type: its signature, and for an item the
    /// function it names.
    fn function_type(&mut self, ty: Ty<'tcx>) -> Value {
        let signature = ty.fn_sig(self.tcx).skip_binder();
        let parameters = self.types_of(signature.inputs().iter().copied());
        let mut record = json!({"kind": "function", "parameters": parameters,
                                "result": self.ty(signature.output())});
        if let ty::FnDef(def_id, _) = *ty.kind() {
            record["definition"] = json!(self.definition(def_id));
        }
        let modifiers: Vec<&str> = [
            (signature.safety().is_unsafe(), "unsafe"),
            (
                signature.abi() != rustc_abi::ExternAbi::Rust,
                "rust:foreign-abi",
            ),
        ]
        .into_iter()
        .filter_map(|(present, modifier)| present.then_some(modifier))
        .collect();
        if !modifiers.is_empty() {
            record["modifiers"] = json!(modifiers);
        }

        record
    }

    // rstyle-allow(mutation_per_target): the unit interns referenced records as it builds them
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
                let mut record =
                    json!({"kind": kind, "definition": definition, "arguments": arguments});
                // A local record's members, which its declaration in this workspace gives,
                // and a standard range's, whose values the frontend observes.
                if (adt.did().is_local() || self.range(adt)) && !adt.is_enum() {
                    record["fields"] = json!(self.fields(adt.non_enum_variant(), args));
                }
                let mut modifiers = self.value_traits(ty);
                // Type arguments are recorded; lifetime arguments only by their presence.
                if args.regions().next().is_some() {
                    modifiers.push("rust:lifetime-arguments");
                }
                // A library sum's variant shapes, which choose how its values are observed.
                if !adt.did().is_local() && adt.is_enum() {
                    modifiers.extend(variant_shapes(adt));
                }
                if adt.did().is_local() && adt.is_enum() {
                    record["variants"] = json!(self.variants(adt, args));
                    if exact_u8(adt) {
                        modifiers.push("rust:repr-u8");
                    }
                }
                if !modifiers.is_empty() {
                    record["modifiers"] = json!(modifiers);
                }
                record
            }
        }
    }

    fn range(&self, adt: ty::AdtDef<'tcx>) -> bool {
        let items = self.tcx.lang_items();
        [
            items.range_struct(),
            items.range_from_struct(),
            items.range_to_struct(),
        ]
        .contains(&Some(adt.did()))
    }

    /// Whether a concrete value type is `Copy` and whether dropping it runs code.
    fn value_traits(&self, ty: Ty<'tcx>) -> Vec<&'static str> {
        // Only a closed concrete type has these judgments of its own.
        if ty.has_param() || ty.has_escaping_bound_vars() || ty.has_infer() {
            return Vec::new();
        }
        let environment = ty::TypingEnv::fully_monomorphized();
        let mut traits = Vec::new();
        if self.tcx.type_is_copy_modulo_regions(environment, ty) {
            traits.push("rust:copy");
        }
        if ty.needs_drop(self.tcx, environment) {
            traits.push("rust:needs-drop");
        }

        traits
    }

    /// A sum's variants in declaration order: shape, discriminant and fields.
    fn variants(&mut self, adt: ty::AdtDef<'tcx>, args: ty::GenericArgsRef<'tcx>) -> Vec<Value> {
        let discriminants: Vec<i64> = adt
            .discriminants(self.tcx)
            .map(|(_, discriminant)| discriminant.val as i64)
            .collect();
        adt.variants()
            .iter()
            .zip(discriminants)
            .map(|(variant, discriminant)| {
                let shape = match variant.ctor_kind() {
                    Some(rustc_hir::def::CtorKind::Const) => "unit",
                    Some(rustc_hir::def::CtorKind::Fn) => "tuple",
                    None => "record",
                };
                json!({"name": variant.name.as_str(), "definition": self.definition(variant.def_id),
                       "shape": shape, "discriminant": discriminant, "fields": self.fields(variant, args)})
            })
            .collect()
    }

    /// A variant's or record's fields in declaration order, with their types instantiated.
    fn fields(&mut self, variant: &ty::VariantDef, args: ty::GenericArgsRef<'tcx>) -> Vec<Value> {
        let tcx = self.tcx;
        variant
            .fields
            .iter()
            .map(|field| {
                let declared = tcx.type_of(field.did).instantiate(tcx, args);
                let written = declared.skip_norm_wip();
                // With parameters left, normalizing needs the bounds of an owner this type
                // has none of here, and the solver panics looking up a const parameter's.
                let ty = match written.has_non_region_param() {
                    true => written,
                    false => tcx
                        .try_normalize_erasing_regions(
                            ty::TypingEnv::fully_monomorphized(),
                            declared,
                        )
                        .unwrap_or(written),
                };
                json!({"name": field.name.as_str(), "type": self.ty(ty)})
            })
            .collect()
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

    // rstyle-allow(mutation_per_target): the unit interns referenced records as it builds them
    /// An associated or free alias type, with the type it normalizes to where that
    /// needs no bounds of an owner.
    fn alias(&mut self, ty: Ty<'tcx>) -> Value {
        let mut record = json!({"kind": "opaque", "modifiers": ["rust:alias"]});
        let concrete =
            !ty.has_non_region_param() && !ty.has_infer() && !ty.has_escaping_bound_vars();
        let environment = ty::TypingEnv::fully_monomorphized();
        let normalized = concrete
            .then(|| {
                self.tcx
                    .try_normalize_erasing_regions(environment, ty::Unnormalized::new_wip(ty))
                    .ok()
            })
            .flatten()
            .filter(|normalized| *normalized != ty);
        if let Some(normalized) = normalized {
            record["normalized"] = json!(self.ty(normalized));
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

    pub fn tables(self) -> (Vec<Value>, Vec<Value>, Vec<Value>, Vec<Value>) {
        (
            self.files.into_values().collect(),
            self.definitions.into_values().collect(),
            self.types.into_values().collect(),
            self.values.into_values().collect(),
        )
    }

    // rstyle-allow(mutation_per_target): the unit interns referenced records as it builds them
    /// A local, nongeneric constant's evaluated scalar value.
    fn constant_value(&mut self, def_id: DefId, id: &str) {
        let tcx = self.tcx;
        if !(def_id.is_local() || primitive_constant(tcx, def_id))
            || !matches!(tcx.def_kind(def_id), DefKind::Const | DefKind::AssocConst)
            || tcx.generics_of(def_id).count() != 0
        {
            return;
        }
        if let Some(initializer) = self.initializer(def_id) {
            let definition = self.definitions.get_mut(id).unwrap();
            definition["extensions"]["rust:constant-initializer"] = initializer;
        }
        let environment = ty::TypingEnv::post_analysis(tcx, def_id);
        let declared = tcx.type_of(def_id).instantiate_identity();
        let Ok(ty) = tcx.try_normalize_erasing_regions(environment, declared) else {
            return;
        };
        let Ok(evaluated) = tcx.const_eval_poly(def_id) else {
            return;
        };
        let literal = literal(tcx, def_id, ty, evaluated);

        if let Some(literal) = literal {
            let record = json!({"definition": id, "type": self.ty(ty), "value": literal});
            self.values.insert(id.to_owned(), record);
        }
    }
}

/// A library constant of a primitive's own implementation, as `u32::MAX` is.
fn primitive_constant(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.def_kind(def_id) == DefKind::AssocConst
        && matches!(
            tcx.def_kind(tcx.parent(def_id)),
            DefKind::Impl { of_trait: false }
        )
        && tcx
            .type_of(tcx.parent(def_id))
            .instantiate_identity()
            .skip_norm_wip()
            .is_primitive()
}

/// A constant's evaluated value as an exact literal, for the types the unit records.
fn literal<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    ty: Ty<'tcx>,
    evaluated: rustc_middle::mir::ConstValue,
) -> Option<Value> {
    match ty.kind() {
        ty::Bool => evaluated
            .try_to_bool()
            .map(|value| json!({"kind": "bool", "value": value.to_string()})),
        ty::Tuple(items) if items.is_empty() => Some(json!({"kind": "unit", "value": "()"})),
        ty::Int(_) => evaluated.try_to_scalar_int().map(
            |value| json!({"kind": "integer", "value": value.to_int(value.size()).to_string()}),
        ),
        ty::Uint(_) => evaluated.try_to_scalar_int().map(
            |value| json!({"kind": "integer", "value": value.to_bits(value.size()).to_string()}),
        ),
        ty::Array(element, length) if element.is_bool() => {
            bool_array(tcx, def_id, ty, length.try_to_target_usize(tcx))
        }
        // A float's exact value is its IEEE bits, in hexadecimal at the float's width.
        ty::Float(_) => evaluated.try_to_scalar_int().map(|value| {
            let bits = value.to_bits(value.size());
            let digits = value.size().bytes_usize() * 2;
            json!({"kind": "float", "value": format!("0x{bits:0digits$x}")})
        }),
        _ => None,
    }
}

/// A bool array constant's elements, read from its evaluated allocation.
fn bool_array<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    ty: Ty<'tcx>,
    length: Option<u64>,
) -> Option<Value> {
    use rustc_middle::mir::{
        ConstValue,
        interpret::{GlobalAlloc, alloc_range},
    };
    let length = length?;
    let environment = ty::TypingEnv::post_analysis(tcx, def_id);
    let layout = tcx.layout_of(environment.as_query_input(ty)).ok()?;
    if layout.size.bytes() != length || layout.align.abi.bytes() != 1 {
        return None;
    }
    let ConstValue::Indirect { alloc_id, offset } = tcx.const_eval_poly(def_id).ok()? else {
        return None;
    };
    let GlobalAlloc::Memory(allocation) = tcx.global_alloc(alloc_id) else {
        return None;
    };
    let allocation = allocation.inner();
    if offset.bytes().checked_add(length)? > allocation.size().bytes() {
        return None;
    }
    let range = alloc_range(offset, rustc_abi::Size::from_bytes(length));
    let bytes = allocation.get_bytes_strip_provenance(&tcx, range).ok()?;
    let items: Option<Vec<Value>> = bytes
        .iter()
        .map(|byte| match byte {
            0 => Some(json!({"kind": "bool", "value": "false"})),
            1 => Some(json!({"kind": "bool", "value": "true"})),
            _ => None,
        })
        .collect();

    Some(json!({"kind": "array", "items": items?}))
}

/// A type's ID: its spelling, and for a type mentioning type parameters their indices,
/// since the same spelling names different parameters in different functions.
fn type_id(tcx: TyCtxt<'_>, spelling: &str, ty: Ty<'_>) -> String {
    let parameters: BTreeSet<String> = ty
        .walk()
        .filter_map(|arg| arg.as_type())
        .filter_map(|ty| match ty.kind() {
            ty::Param(param) => Some(format!("{}={}", param.name, param.index)),
            // A nominal type spelled as a primitive is, as `struct u32` is, by its identity.
            ty::Adt(adt, _) if primitive_name(&tcx.def_path_str(adt.did())) => {
                Some(format!("{:?}", tcx.def_path_hash(adt.did())))
            }
            _ => None,
        })
        .collect();
    if parameters.is_empty() {
        return format!("type:{spelling}");
    }
    let parameters: Vec<String> = parameters.into_iter().collect();
    format!("type:{spelling}#{}", parameters.join(","))
}

fn primitive_name(name: &str) -> bool {
    matches!(
        name,
        "bool"
            | "char"
            | "str"
            | "f16"
            | "f32"
            | "f64"
            | "f128"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
    )
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
        DefKind::Fn | DefKind::AssocFn => {
            let unsafe_call = tcx
                .fn_sig(def_id)
                .skip_binder()
                .skip_binder()
                .safety()
                .is_unsafe();
            let without_body = kind == DefKind::AssocFn
                && tcx.trait_of_assoc(def_id).is_some()
                && !tcx.associated_item(def_id).defaultness(tcx).has_value();
            [(unsafe_call, "unsafe"), (without_body, "abstract")]
                .into_iter()
                .filter_map(|(holds, modifier)| holds.then_some(modifier))
                .collect()
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

/// The optional compiler facts of a definition: whether it is standard, the
/// operation a standard callable declares, and whether it iterates or has a destructor.
fn compiler_facts(tcx: TyCtxt<'_>, def_id: DefId, kind: DefKind) -> Vec<(&'static str, Value)> {
    let standard = super::compiler::standard(tcx, def_id);
    let callable = matches!(kind, DefKind::Fn | DefKind::AssocFn);
    let adt = matches!(kind, DefKind::Struct | DefKind::Enum | DefKind::Union);
    let operation = (standard && callable)
        .then(|| super::compiler::declared_operation(tcx, def_id))
        .flatten();

    [
        ("standard", standard.then_some(Value::Bool(true))),
        ("operation", operation.map(|operation| json!(operation))),
        (
            "iteration",
            (standard && super::compiler::iterator_member(tcx, def_id))
                .then_some(Value::Bool(true)),
        ),
        (
            "destructor",
            (adt && tcx.adt_destructor(def_id).is_some()).then_some(Value::Bool(true)),
        ),
    ]
    .into_iter()
    .filter_map(|(field, value)| Some((field, value?)))
    .collect()
}

// rstyle-allow(mutation_per_target): builds one identity record key by key
/// The compiler's own identity of a definition, as the frontend's facts print it:
/// its `DefId`, `DefKind`, `DefPathHash`, crate name and, for another crate, its hash.
fn rust_identity(tcx: TyCtxt<'_>, def_id: DefId, kind: DefKind) -> serde_json::Map<String, Value> {
    let mut identity = serde_json::Map::new();
    identity.insert("def_id".into(), json!(format!("{def_id:?}")));
    identity.insert("def_kind".into(), json!(format!("{kind:?}")));
    if let Some(name) = tcx.get_diagnostic_name(def_id) {
        identity.insert("diagnostic".into(), json!(name.as_str()));
    }
    identity.insert(
        "def_path_hash".into(),
        json!(format!("{:?}", tcx.def_path_hash(def_id))),
    );
    identity.insert(
        "crate_name".into(),
        json!(tcx.crate_name(def_id.krate).to_string()),
    );
    identity.insert(
        "stable_crate_id".into(),
        json!(format!("{:?}", tcx.stable_crate_id(def_id.krate))),
    );
    if def_id.is_local()
        && let Some(parent) = tcx.opt_parent(def_id)
    {
        identity.insert("parent".into(), json!(format!("{parent:?}")));
    }
    if !def_id.is_local() {
        identity.insert(
            "crate_hash".into(),
            json!(tcx.crate_hash(def_id.krate).to_string()),
        );
        let rmeta = tcx.used_crate_source(def_id.krate).rmeta.clone();
        if let Some(rmeta) = rmeta {
            identity.insert("rmeta".into(), json!(rmeta));
        }
    }
    if let Some(generics) = generics(tcx, def_id, kind) {
        identity.insert("generics".into(), generics);
    }

    identity
}

/// The count of a generic item's parameters, its parents' included, and its own
/// parameters' kinds, as the compiler's generics record them.
fn generics(tcx: TyCtxt<'_>, def_id: DefId, kind: DefKind) -> Option<Value> {
    if !matches!(
        kind,
        DefKind::Fn
            | DefKind::AssocFn
            | DefKind::Ctor(..)
            | DefKind::Struct
            | DefKind::Enum
            | DefKind::Union
            | DefKind::Impl { .. }
            | DefKind::Trait
    ) {
        return None;
    }
    let generics = tcx.generics_of(def_id);
    let own: Vec<&str> = generics
        .own_params
        .iter()
        .map(|parameter| match parameter.kind {
            ty::GenericParamDefKind::Lifetime => "lifetime",
            ty::GenericParamDefKind::Type { .. } => "type",
            ty::GenericParamDefKind::Const { .. } => "const",
        })
        .collect();
    Some(json!({"count": generics.count(), "own": own, "parented": generics.parent.is_some()}))
}

/// Whether a sum is represented exactly as `u8`, with no other representation hint.
fn exact_u8(adt: ty::AdtDef<'_>) -> bool {
    let representation = adt.repr();
    representation.int == Some(rustc_abi::IntegerType::Fixed(rustc_abi::Integer::I8, false))
        && !representation.c()
        && !representation.transparent()
        && !representation.simd()
        && !representation.scalable()
        && representation.align.is_none()
        && representation.pack.is_none()
}

/// Whether every variant of a sum is fieldless, and whether any is unit or record-like.
fn variant_shapes(adt: ty::AdtDef<'_>) -> Vec<&'static str> {
    let mut shapes = Vec::new();
    if adt
        .variants()
        .iter()
        .all(|variant| variant.fields.is_empty())
    {
        shapes.push("rust:fieldless-variants");
    }
    if adt
        .variants()
        .iter()
        .any(|variant| variant.fields.is_empty() || variant.ctor_kind().is_none())
    {
        shapes.push("rust:unit-or-record-variant");
    }
    shapes
}
