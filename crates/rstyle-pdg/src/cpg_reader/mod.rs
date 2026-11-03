//! Frontend facts read from `compat.cpg` documents: the [`HirFnFacts`] the analyses
//! consume, rebuilt from what a Rust document records instead of from the compiler.
//!
//! A channel the document does not yet carry is reported by [`compare`] against the
//! driver's own capture, never filled with a default that hides the difference.

mod body;
pub mod compare;

use crate::extraction_profile::ExtractionProfile;
use crate::frontend::{
    BindingFact, ByteSpan, GenericFact, HirFnFacts, HirNode, HirRef, ParamFact, ReceiverFact,
    ResolvedTy, SignatureFacts, SourceFacts, SourceSnapshot, SourceText,
};
use rstyle_core::cpg::model::{Document, File, FileSpan, Function, Type, TypeKind, Unit};
use rstyle_core::cpg::tree::{Body, Tables};
use std::collections::BTreeMap;

/// One function read from a document, keyed as the driver's capture keys it.
pub struct Read {
    pub name: String,
    pub fn_span: ByteSpan,
    pub facts: Result<HirFnFacts, String>,
}

/// Every function of a bundled document, read under `profile`.
pub fn read(document: &Document, profile: ExtractionProfile) -> Result<Vec<Read>, String> {
    let unit = document.unit.as_ref().ok_or("The document has no unit")?;
    let reader = Reader::new((unit, &document.producer), profile)?;
    let functions = unit.functions.as_deref().unwrap_or_default();

    Ok(functions
        .iter()
        .map(|function| reader.read(function))
        .collect())
}

struct Reader<'a> {
    unit: &'a Unit,
    producer: &'a rstyle_core::cpg::model::Producer,
    tables: Tables<'a>,
    files: BTreeMap<&'a str, &'a File>,
    profile: ExtractionProfile,
}

impl<'a> Reader<'a> {
    fn new(
        (unit, producer): (&'a Unit, &'a rstyle_core::cpg::model::Producer),
        profile: ExtractionProfile,
    ) -> Result<Self, String> {
        let files = unit
            .files
            .iter()
            .map(|file| (file.id.as_str(), file))
            .collect();
        Ok(Self {
            unit,
            producer,
            tables: Tables::new(unit)?,
            files,
            profile,
        })
    }

    fn read(&self, function: &'a Function) -> Read {
        let fn_span = ByteSpan {
            start: function.span.start as u32,
            end: function.span.end as u32,
        };
        Read {
            name: function.name.clone(),
            fn_span,
            facts: self.facts(function, fn_span),
        }
    }

    fn facts(&self, function: &'a Function, fn_span: ByteSpan) -> Result<HirFnFacts, String> {
        let body = Body::new(function)?;
        let source = self.source(&function.span)?;
        let context = Context {
            reader: self,
            body: &body,
            origin: function.span.start,
            source: source.as_str(),
        };
        let params = context.parameters()?;
        let tree = context.tree(body.root()?)?;
        let scope = format!("scope:source:fn:0:{}", source.len());
        let bindings: Vec<BindingFact> = params
            .iter()
            .flat_map(|param| crate::frontend::bindings(&param.pat, &scope))
            .chain(crate::frontend::bindings(&tree, &scope))
            .collect();
        let signature = context.signature(params)?;

        Ok(HirFnFacts {
            name: function.name.clone(),
            fn_span,
            source: SourceFacts {
                digest: self.digest(&source),
                bytes: source,
            },
            signature,
            bindings,
            body: tree,
            build: self.build(),
            source_diagnostics: Vec::new(),
            source_scan_complete: false,
        })
    }

    fn source(&self, span: &FileSpan) -> Result<SourceText, String> {
        let file = self.file(&span.file)?;
        let content = file
            .content
            .as_deref()
            .ok_or_else(|| format!("File {} has no content", file.id))?;
        let range = ByteSpan {
            start: span.start as u32,
            end: span.end as u32,
        };
        SourceText::from_snapshot(SourceSnapshot::from(content), range)
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

    fn build(&self) -> crate::frontend::BuildFacts {
        build::facts(self.unit, self.producer)
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

/// One function's reading context.
struct Context<'r, 'a> {
    reader: &'r Reader<'a>,
    body: &'r Body<'a>,
    /// File offset the function's relative spans count from.
    origin: usize,
    source: &'r str,
}

impl<'a> Context<'_, 'a> {
    /// A file span relative to the function's bytes.
    fn span(&self, span: &FileSpan) -> ByteSpan {
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
        let receiver = signature
            .receiver
            .iter()
            .map(|receiver| (&receiver.symbol, &receiver.ty, receiver.annotation.as_ref()));
        let parameters = signature.parameters.iter().map(|parameter| {
            (
                &parameter.symbol,
                &parameter.ty,
                parameter.annotation.as_ref(),
            )
        });

        receiver
            .chain(parameters)
            .enumerate()
            .map(|(position, (symbol, ty, annotation))| {
                self.parameter(position as u32, (symbol, ty), annotation)
            })
            .collect()
    }

    fn parameter(
        &self,
        position: u32,
        (symbol, ty): (&str, &str),
        annotation: Option<&FileSpan>,
    ) -> Result<ParamFact, String> {
        let record = self.body.symbol(symbol)?;
        let name_span = record
            .declared
            .as_ref()
            .map(|span| self.span(span))
            .unwrap_or(ByteSpan { start: 0, end: 0 });
        let ty_span = annotation
            .map(|span| self.span(span))
            .unwrap_or(ByteSpan { start: 0, end: 0 });
        let resolved = self.reader.resolved(ty, self.snippet(ty_span))?;
        let pat = HirNode {
            hir: hir_ref(symbol),
            kind: "pat_binding".into(),
            span: name_span,
            snippet: self.snippet(name_span),
            ty: Some(ResolvedTy {
                spelling: String::new(),
                ..resolved.clone()
            }),
            res: None,
            dispatch: None,
            expansion: None,
            binding: Some(symbol_name(symbol)),
            binding_mut: Some(record.mutable),
            lit: None,
            param_index: Some(position),
            is_unsafe_block: false,
            children: Vec::new(),
        };

        Ok(ParamFact {
            symbol: symbol_name(symbol),
            position,
            name_span,
            ty_span,
            ty: resolved,
            pat,
        })
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
            return_ty: self
                .reader
                .resolved(&signature.result, self.result_spelling(result_span))?,
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
    fn result_spelling(&self, span: ByteSpan) -> String {
        match span.start == span.end {
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
        let mut declared: Vec<(u32, GenericFact)> = lifetimes
            .chain(types)
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
