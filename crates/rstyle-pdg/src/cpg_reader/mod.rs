//! Frontend facts read from `compat.cpg` documents: the [`HirFnFacts`] the analyses
//! consume, rebuilt from what a Rust document records instead of from the compiler.
//!
//! A channel the document does not yet carry is reported by [`compare`] against the
//! driver's own capture, never filled with a default that hides the difference.

mod advisory;
mod assertions;
mod body;
mod callbacks;
mod callees;
mod concrete;
pub use concrete::Passes;
pub mod compare;
mod findings;
mod for_loops;
mod inherent;
mod items;
mod locals;
mod patterns;
mod propagation;
mod resolution;
mod scan;
mod slices;
mod variants;

use crate::extraction_profile::ExtractionProfile;
use crate::frontend::{
    BindingFact, ByteSpan, GenericFact, HirFnFacts, HirNode, HirRef, ParamFact, ReceiverFact,
    ResolvedTy, SignatureFacts, SourceFacts, SourceSnapshot, SourceText,
};
use rstyle_core::cpg::model::{Document, File, FileSpan, Function, Node, Type, TypeKind, Unit};
use rstyle_core::cpg::tree::{Body, Tables};
use std::collections::BTreeMap;

/// One function read from a document, keyed as the driver's capture keys it.
pub struct Read {
    pub name: String,
    pub fn_span: ByteSpan,
    /// The compiler owner of the function's body, which tells apart functions of
    /// one name and source.
    pub owner: u32,
    pub facts: Result<HirFnFacts, String>,
}

/// Every function of a bundled document, read under `profile` through `passes`.
pub fn read(
    document: &Document,
    profile: ExtractionProfile,
    passes: Passes,
) -> Result<Vec<Read>, String> {
    let unit = document.unit.as_ref().ok_or("The document has no unit")?;
    let mut reader = Reader::new((unit, &document.producer), (profile, passes))?;
    reader.advisory = advisory::derive(document)?;
    let functions = unit.functions.as_deref().unwrap_or_default();

    Ok(functions
        .iter()
        .map(|function| reader.read(function))
        .collect())
}

struct Reader<'a> {
    unit: &'a Unit,
    passes: Passes,
    /// Each scanned file's functions, by file ID.
    scans: std::cell::RefCell<BTreeMap<&'a str, std::rc::Rc<Vec<scan::Function>>>>,
    producer: &'a rstyle_core::cpg::model::Producer,
    tables: Tables<'a>,
    files: BTreeMap<&'a str, &'a File>,
    profile: ExtractionProfile,
    /// Each file's shared snapshot, by file ID.
    snapshots: std::cell::RefCell<BTreeMap<&'a str, SourceSnapshot>>,
    /// Each function's core facts, when the advisory run records them.
    advisory: Option<Vec<rstyle_core::facts::FnFacts>>,
}

impl<'a> Reader<'a> {
    fn new(
        (unit, producer): (&'a Unit, &'a rstyle_core::cpg::model::Producer),
        (profile, passes): (ExtractionProfile, Passes),
    ) -> Result<Self, String> {
        let files = unit
            .files
            .iter()
            .map(|file| (file.id.as_str(), file))
            .collect();
        Ok(Self {
            unit,
            scans: Default::default(),
            producer,
            advisory: None,
            snapshots: Default::default(),
            tables: Tables::new(unit)?,
            files,
            profile,
            passes,
        })
    }

    fn read(&self, function: &'a Function) -> Read {
        let located = located(function);
        let fn_span = ByteSpan {
            start: located.start as u32,
            end: located.end as u32,
        };
        Read {
            name: function.name.clone(),
            fn_span,
            owner: hir_ref(&function.root).owner,
            facts: self.facts(function, fn_span),
        }
    }

    fn facts(&self, function: &'a Function, fn_span: ByteSpan) -> Result<HirFnFacts, String> {
        let body = Body::new(function)?;
        let located = located(function);
        let source = self.source(located)?;
        let context = Context {
            reader: self,
            body: &body,
            exits: resolution::loop_exits(&body)?,
            parents: function
                .nodes
                .iter()
                .flat_map(|node| {
                    node.children
                        .iter()
                        .map(move |child| (child.as_str(), node))
                })
                .collect(),
            origin: located.start,
            source: source.as_str(),
        };

        let facts = self.walked(&context, (function, fn_span), &source)?;
        context.after_walk(facts, self.passes)
    }

    /// The facts the walk itself produces: the signature, body and bindings.
    fn walked(
        &self,
        context: &Context<'_, 'a>,
        (function, fn_span): (&'a Function, ByteSpan),
        source: &SourceText,
    ) -> Result<HirFnFacts, String> {
        let body = context.body;
        let params = context.parameters()?;
        let tree = context.tree(body.root()?)?;
        let scope = format!("scope:source:fn:0:{}", source.len());
        let bindings: Vec<BindingFact> = params
            .iter()
            .flat_map(|param| crate::frontend::bindings(&param.pat, &scope))
            .chain(crate::frontend::bindings(&tree, &scope))
            .collect();
        let signature = context.signature(params)?;
        let (source_diagnostics, source_scan_complete) = context.source_scan(function)?;

        Ok(HirFnFacts {
            name: function.name.clone(),
            fn_span,
            source: SourceFacts {
                digest: self.digest(source),
                bytes: source.clone(),
            },
            signature,
            bindings,
            body: tree,
            build: self.function_build(function),
            source_diagnostics,
            source_scan_complete,
        })
    }

    fn source(&self, span: &FileSpan) -> Result<SourceText, String> {
        let range = ByteSpan {
            start: span.start as u32,
            end: span.end as u32,
        };
        SourceText::from_snapshot(self.snapshot(&span.file)?, range)
    }

    /// A file's bytes, one snapshot per file, as the frontend shares them.
    fn snapshot(&self, id: &str) -> Result<SourceSnapshot, String> {
        let file = self.file(id)?;
        if let Some(snapshot) = self.snapshots.borrow().get(file.id.as_str()) {
            return Ok(snapshot.clone());
        }
        let content = file
            .content
            .as_deref()
            .ok_or_else(|| format!("File {} has no content", file.id))?;
        let snapshot = SourceSnapshot::from(content);
        self.snapshots
            .borrow_mut()
            .insert(file.id.as_str(), snapshot.clone());

        Ok(snapshot)
    }

    /// The type arguments of a type the compiler names by diagnostic item `name`.
    fn diagnostic_arguments(&self, ty: &Type, name: &str) -> Option<&'a [String]> {
        let definition = self.tables.definition(ty.definition.as_deref()?).ok()?;
        (identity(definition, "diagnostic") == Some(name)).then_some(())?;
        // The type table outlives the type a caller holds.
        let own = self.tables.ty(&ty.id).ok()?;
        own.arguments.as_deref()
    }

    /// A file's absolute path, as the capture session records it.
    fn session_file(&self, file: &str) -> Option<&'a str> {
        self.unit
            .extensions
            .as_ref()?
            .get("rust:compiler-session")?
            .get("files")?
            .get(file)?
            .as_str()
    }

    /// Whether a type is the integer type `spelling` names.
    fn integer(&self, ty: Option<&str>, spelling: &str) -> bool {
        ty.and_then(|ty| self.ty(ty).ok())
            .is_some_and(|ty| ty.kind == TypeKind::Integer && ty.spelling == spelling)
    }

    fn file(&self, id: &str) -> Result<&'a File, String> {
        self.files
            .get(id)
            .copied()
            .ok_or_else(|| format!("Unknown file {id}"))
    }

    /// Function-source identity as the profile records it.
    fn digest(&self, source: &SourceText) -> String {
        if self.profile == ExtractionProfile::DataV3 {
            source.checksum().to_owned()
        } else {
            source.identity().to_owned()
        }
    }

    /// The unit's build facts, with the options the frontend records for a function.
    fn function_build(&self, function: &Function) -> crate::frontend::BuildFacts {
        let mut build = build::facts(self.unit, self.producer);
        let session = self
            .unit
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.get("rust:compiler-session"));
        build::session_options(self.unit, session, &mut build.frontend_options);
        let definition = self.tables.definition(&function.definition).ok();
        let options = &mut build.frontend_options;
        let file = session
            .and_then(|session| session.get("files")?.get(&located(function).file)?.as_str());
        if let Some(file) = file {
            options.insert("source_file".into(), serde_json::json!(file));
        }
        let name = definition
            .and_then(|definition| {
                definition
                    .extensions
                    .get("rust:compiler-identity")?
                    .get("ident_span")
                    .cloned()
            })
            .and_then(|span| serde_json::from_value::<FileSpan>(span).ok())
            .and_then(|span| self.source(&span).ok())
            .map(|name| {
                name.as_str()
                    .strip_prefix("r#")
                    .unwrap_or(name.as_str())
                    .to_owned()
            });
        options.insert("source_identifier".into(), serde_json::json!(name));
        if let Some(definition) = definition {
            build::identity_options(self.profile, definition, options);
        }

        build
    }

    /// The unit type as the driver records it.
    fn unit_type(&self) -> Result<ResolvedTy, String> {
        Ok(ResolvedTy {
            normalized: Some("unit".into()),
            observed: "()".into(),
            spelling: String::new(),
        })
    }

    fn ty(&self, id: &str) -> Result<&'a Type, String> {
        self.tables.ty(id)
    }

    /// A type as the driver records it: the scalar token, its observed spelling and
    /// the source spelling of an annotation.
    fn resolved(&self, id: &str, spelling: String) -> Result<ResolvedTy, String> {
        let ty = self.ty(id)?;
        let normalized = match ty.kind {
            TypeKind::Integer | TypeKind::Bool | TypeKind::Char => Some(ty.spelling.clone()),
            TypeKind::Float if matches!(ty.bits, Some(32 | 64)) => Some(ty.spelling.clone()),
            TypeKind::Unit => Some("unit".to_owned()),
            _ => None,
        };
        Ok(ResolvedTy {
            normalized,
            observed: ty.spelling.clone(),
            spelling,
        })
    }
}

mod build;

/// A signature parameter's records, the receiver included.
struct Parameter<'p> {
    symbol: &'p str,
    ty: &'p str,
    annotation: Option<&'p FileSpan>,
    /// A parameter pattern's ID, tag and span.
    pattern: Option<(&'p str, &'p str, &'p FileSpan)>,
}

/// One function's reading context.
struct Context<'r, 'a> {
    reader: &'r Reader<'a>,
    body: &'r Body<'a>,
    /// Each loop exit's labelled flag and destination loop.
    exits: std::collections::BTreeMap<String, (bool, HirRef)>,
    /// Each node's parent.
    parents: std::collections::BTreeMap<&'a str, &'a Node>,
    /// File offset the function's relative spans count from.
    origin: usize,
    source: &'r str,
}

impl<'a> Context<'_, 'a> {
    // rstyle-allow(mutation_per_target): each pass installs its own records on the facts in turn
    /// The frontend's passes after its walk, in its order.
    fn after_walk(&self, mut facts: HirFnFacts, passes: Passes) -> Result<HirFnFacts, String> {
        self.concrete_types(&mut facts, passes)?;
        self.reader.advisory(self.body.function, &mut facts)?;
        if passes == Passes::Complete {
            self.inherent_definition(&mut facts)?;
            self.trait_members(&mut facts)?;
            crate::field_locations::capture(&mut facts)?;
            if self.reader.profile == ExtractionProfile::DataV3 {
                self.local_observations(&mut facts)?;
            }
            self.call_instances(&mut facts)?;
        }
        // An advisory consumer of mutations reads the body's local mutation types too.
        if passes == Passes::AdvisoryMutations && self.reader.profile == ExtractionProfile::DataV3 {
            self.local_observations(&mut facts)?;
        }

        Ok(facts)
    }

    /// A frontend observation the function's document records, or the error the
    /// frontend met observing it.
    fn function_extension(&self, key: &str) -> Result<Option<&'a serde_json::Value>, String> {
        let extensions = self.body.function.extensions.as_ref();
        let Some(value) = extensions.and_then(|extensions| extensions.get(key)) else {
            return Ok(None);
        };
        match value.get("error").and_then(serde_json::Value::as_str) {
            Some(error) => Err(error.to_owned()),
            None => Ok(Some(value)),
        }
    }

    /// A Rust-only contract the frontend observes at a node, as it records it.
    pub(super) fn node_observation<T: serde::de::DeserializeOwned>(
        &self,
        node: &Node,
        key: &str,
    ) -> Option<T> {
        let observations = self
            .body
            .function
            .extensions
            .as_ref()?
            .get("rust:node-observations")?;
        serde_json::from_value(observations.get(&node.id)?.get(key)?.clone()).ok()
    }

    /// The generic and trait call instances the frontend observes, as it records them.
    fn call_instances(&self, facts: &mut HirFnFacts) -> Result<(), String> {
        if let Some(inventory) = self.function_extension("rust:call-instances")? {
            let options = &mut facts.build.frontend_options;
            options.insert("rstyle.rust.call-instances".into(), inventory.clone());
        }
        Ok(())
    }

    /// A trait member's own selections, as the frontend observes them.
    fn trait_members(&self, facts: &mut HirFnFacts) -> Result<(), String> {
        let members = self.function_extension("rust:trait-members")?;
        for (key, value) in members
            .and_then(serde_json::Value::as_object)
            .into_iter()
            .flatten()
        {
            facts
                .build
                .frontend_options
                .insert(key.clone(), value.clone());
        }
        Ok(())
    }

    /// The definition a node's reference targets, when the target is of `kind`.
    fn target_definition(
        &self,
        node: &Node,
        kind: rstyle_core::cpg::model::TargetKind,
    ) -> Option<&'a rstyle_core::cpg::model::Definition> {
        let target = self.body.reference(&node.id)?.target.as_ref()?;
        if target.kind != kind {
            return None;
        }
        self.reader.tables.definition(target.id.as_deref()?).ok()
    }

    /// A file span relative to the function's bytes.
    /// A macro-generated function locates what its macro writes at the call site.
    fn span(&self, span: &FileSpan) -> ByteSpan {
        let located = located(self.body.function);
        let outside =
            span.file != located.file || span.start < located.start || span.end > located.end;
        let span = if outside && !std::ptr::eq(located, &self.body.function.span) {
            located
        } else {
            span
        };
        ByteSpan {
            start: span.start.saturating_sub(self.origin) as u32,
            end: span.end.saturating_sub(self.origin) as u32,
        }
    }

    fn snippet(&self, span: ByteSpan) -> String {
        self.source
            .get(span.start as usize..span.end as usize)
            .unwrap_or_default()
            .to_owned()
    }

    fn parameters(&self) -> Result<Vec<ParamFact>, String> {
        let signature = &self.body.function.signature;
        let receiver = signature.receiver.iter().map(|receiver| Parameter {
            symbol: &receiver.symbol,
            ty: &receiver.ty,
            annotation: receiver.annotation.as_ref(),
            pattern: None,
        });
        let parameters = signature.parameters.iter().map(|parameter| Parameter {
            symbol: &parameter.symbol,
            ty: &parameter.ty,
            annotation: parameter.annotation.as_ref(),
            pattern: parameter
                .pattern
                .as_deref()
                .zip(parameter.original_kind.as_deref())
                .zip(parameter.span.as_ref())
                .map(|((id, tag), span)| (id, tag, span)),
        });

        receiver
            .chain(parameters)
            .enumerate()
            .map(|(position, parameter)| self.parameter(position as u32, parameter))
            .collect()
    }

    fn parameter(&self, position: u32, parameter: Parameter<'_>) -> Result<ParamFact, String> {
        let ty_span = parameter
            .annotation
            .map_or(ByteSpan { start: 0, end: 0 }, |span| self.span(span));
        // A binding's type is the one typeck records, with its regions erased.
        let ty = self
            .body
            .symbol(parameter.symbol)
            .ok()
            .and_then(|symbol| symbol.ty.as_deref())
            .unwrap_or(parameter.ty);
        let resolved = self.reader.resolved(ty, self.snippet(ty_span))?;
        let pat = match parameter.pattern {
            Some((id, tag, span)) if tag != "pat_binding" => {
                self.parameter_pattern((id, tag, span), parameter.symbol, &resolved)?
            }
            _ => self.binding_parameter(parameter.symbol, &resolved)?,
        };
        let pat = HirNode {
            param_index: Some(position),
            ..pat
        };

        Ok(ParamFact {
            symbol: pat.binding.clone().unwrap_or_default(),
            position,
            name_span: pat.span,
            ty_span,
            ty: resolved,
            pat,
        })
    }

    /// A parameter that is a plain binding, located at its identifier.
    fn binding_parameter(&self, symbol: &str, resolved: &ResolvedTy) -> Result<HirNode, String> {
        let record = self.body.symbol(symbol)?;
        let span = record
            .declared
            .as_ref()
            .map_or(ByteSpan { start: 0, end: 0 }, |span| self.span(span));
        Ok(HirNode {
            binding: Some(symbol_name(symbol)),
            binding_mut: Some(record.mutable),
            ..self.parameter_node(hir_ref(symbol), ("pat_binding", span), resolved)
        })
    }

    /// Any other parameter pattern, as the frontend's pattern records tag it.
    fn parameter_pattern(
        &self,
        (id, tag, span): (&str, &str, &FileSpan),
        symbol: &str,
        resolved: &ResolvedTy,
    ) -> Result<HirNode, String> {
        let tag = match tag {
            "pat_wild" | "pat_binding_pattern" => tag,
            _ => "pat_unsupported",
        };
        let node = self.parameter_node(hir_ref(id), (tag, self.span(span)), resolved);
        if tag != "pat_binding_pattern" {
            return Ok(node);
        }
        let record = self.body.symbol(symbol)?;

        Ok(HirNode {
            binding: Some(symbol_name(symbol)),
            binding_mut: Some(record.mutable),
            ..node
        })
    }

    fn parameter_node(
        &self,
        hir: HirRef,
        (tag, span): (&str, ByteSpan),
        resolved: &ResolvedTy,
    ) -> HirNode {
        HirNode {
            hir,
            kind: tag.into(),
            span,
            snippet: self.snippet(span),
            ty: Some(ResolvedTy {
                spelling: String::new(),
                ..resolved.clone()
            }),
            res: None,
            dispatch: None,
            expansion: None,
            binding: None,
            binding_mut: None,
            lit: None,
            param_index: None,
            is_unsafe_block: false,
            children: Vec::new(),
        }
    }

    fn signature(&self, params: Vec<ParamFact>) -> Result<SignatureFacts, String> {
        let signature = &self.body.function.signature;
        let result_span = signature
            .result_span
            .as_ref()
            .map_or(ByteSpan { start: 0, end: 0 }, |span| self.span(span));
        let modifiers = &signature.modifiers;
        let has = |modifier: &str| modifiers.iter().any(|m| m == modifier);
        let receiver = signature
            .receiver
            .as_ref()
            .and(params.first())
            .map(|first| ReceiverFact {
                span: first.name_span,
                snippet: first.pat.snippet.clone(),
            });

        Ok(SignatureFacts {
            sig_span: self.span(&signature.span),
            receiver,
            return_ty: self.reader.resolved(
                &signature.result,
                self.result_spelling(result_span, signature.implicit_result),
            )?,
            return_ty_span: result_span,
            has_self: signature.receiver.is_some(),
            abi: crate::model::StringFact {
                status: crate::model::FactStatus::Known,
                value: Some(if has("rust:foreign-abi") { "C" } else { "Rust" }.into()),
                reason: None,
            },
            generics: self.generics(),
            is_variadic: has("variadic"),
            is_async: has("async"),
            is_unsafe: has("unsafe"),
            is_const: has("const"),
            params,
        })
    }
}

impl Context<'_, '_> {
    /// A written result type as written; an implicit one as the unit it is.
    fn result_spelling(&self, span: ByteSpan, implicit: bool) -> String {
        match implicit {
            true => "()".into(),
            false => self.snippet(span),
        }
    }

    /// The function's own generic parameters in declaration order, lifetimes included.
    fn generics(&self) -> Vec<GenericFact> {
        let signature = &self.body.function.signature;
        let lifetimes = signature
            .lifetimes
            .iter()
            .map(|lifetime| (lifetime.position, &lifetime.name, lifetime.span.as_ref()));
        let types = signature
            .generics
            .iter()
            .map(|generic| (generic.position, &generic.name, generic.span.as_ref()));
        let constants = signature
            .constants
            .iter()
            .map(|constant| (constant.position, &constant.name, constant.span.as_ref()));
        let mut declared: Vec<(u32, GenericFact)> = lifetimes
            .chain(types)
            .chain(constants)
            .filter_map(|(position, name, span)| {
                let span = span.map_or(ByteSpan { start: 0, end: 0 }, |span| self.span(span));
                Some((
                    position?,
                    GenericFact {
                        name: name.clone(),
                        span,
                    },
                ))
            })
            .collect();
        declared.sort_by_key(|(position, _)| *position);

        declared.into_iter().map(|(_, generic)| generic).collect()
    }
}

/// The HIR identity a node or symbol ID carries: `f<owner>.<local>` or `sf<owner>.<local>`.
/// A key of a definition's `rust:compiler-identity` extension.
fn identity<'d>(definition: &'d rstyle_core::cpg::model::Definition, key: &str) -> Option<&'d str> {
    definition
        .extensions
        .get("rust:compiler-identity")?
        .get(key)?
        .as_str()
}

/// Where the frontend locates a function: where it is written, or for a function a
/// macro generates, the macro's outermost call site.
fn located(function: &Function) -> &FileSpan {
    function
        .nodes
        .iter()
        .find(|node| node.id == function.root)
        .filter(|root| root.provenance.origin == rstyle_core::cpg::model::Origin::MacroExpansion)
        .and_then(|root| root.provenance.call_site.as_ref())
        .unwrap_or(&function.span)
}

fn hir_ref(id: &str) -> HirRef {
    let id = id.strip_prefix('s').unwrap_or(id);
    let id = id.strip_prefix('f').unwrap_or(id);
    let (owner, local) = id.split_once('.').unwrap_or_default();
    HirRef {
        krate: 0,
        owner: owner.parse().unwrap_or(u32::MAX),
        local: local.parse().unwrap_or(u32::MAX),
    }
}

/// The driver's symbol name for a document symbol: `s:<owner>:<local>`.
fn symbol_name(id: &str) -> String {
    let reference = hir_ref(id);
    format!("s:{}:{}", reference.owner, reference.local)
}
