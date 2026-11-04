//! Callback closure environments: each callback closure's captures and how its
//! environment stores them, observed with the concrete type grammar.

use super::Capture;
use crate::concrete_types::{MAX_FIELDS, Observation};
use crate::frontend::{HirNode, HirRef, PathRes};
use crate::id_error_maps::capture as reader_records;
use crate::option_callbacks::{self as records, CaptureMode};
use rstyle_core::cpg::model::{Capture as Captured, Node, Symbol, Type, TypeKind};

const USIZE_READER_KIND: &str = "compiler_usize_reader_callback_environment";
const USIZE_READER_IDENTITY: &str = "rstyle.rust.usize-reader-callback-environment/1";

impl<'a> Capture<'_, '_, 'a> {
    /// Each callback closure's environment, as the frontend resolves the closure.
    pub(super) fn callback_environments(&mut self, node: &mut HirNode) -> Result<(), String> {
        let profile = self.context.reader.profile;
        if profile.supports("rust.immediate-option-callbacks") {
            self.option_environments(node, None)?;
        }
        if profile.supports(crate::id_error_maps::CAPABILITY) {
            self.reader_environments(
                node,
                &|context, closure| {
                    context.immediate_core_callback(
                        closure,
                        &["core::result::Result::<T, E>::map_err"],
                    )
                },
                (reader_records::KIND, reader_records::IDENTITY),
            )?;
        }
        if profile.supports(super::super::callbacks::USIZE_READERS) {
            self.reader_environments(
                node,
                &|context, closure| context.usize_reader_callback(closure).unwrap_or(false),
                (USIZE_READER_KIND, USIZE_READER_IDENTITY),
            )?;
        }
        Ok(())
    }

    fn closure(&self, node: &HirNode) -> Result<&'a Node, String> {
        self.context
            .body
            .node(&format!("f{}.{}", node.hir.owner, node.hir.local))
    }

    fn option_environments(
        &mut self,
        node: &mut HirNode,
        owner: Option<&HirRef>,
    ) -> Result<(), String> {
        if node.kind == "expr_closure" && !node.children.is_empty() {
            node.res = Some(self.option_environment(node, owner == Some(&node.hir))?);
        }
        let callback = immediate_map_or_callback(node);
        for child in &mut node.children {
            self.option_environments(child, callback.as_ref())?;
        }
        Ok(())
    }

    fn reader_environments(
        &mut self,
        node: &mut HirNode,
        selected: &dyn Fn(&super::super::Context<'_, 'a>, &'a Node) -> bool,
        (kind, identity): (&str, &str),
    ) -> Result<(), String> {
        if node.kind == "expr_closure" && !node.children.is_empty() {
            let closure = self.closure(node)?;
            if selected(self.context, closure) {
                let environment = self.reader_environment(closure, identity)?;
                node.res = Some(PathRes {
                    kind: kind.into(),
                    target: None,
                    description: serde_json::to_string(&environment)
                        .map_err(|error| error.to_string())?,
                });
            }
        }
        for child in &mut node.children {
            self.reader_environments(child, selected, (kind, identity))?;
        }
        Ok(())
    }

    /// The closure's type, environment and captures, within the field budget.
    fn environment_parts(
        &self,
        closure: &'a Node,
    ) -> Result<(&'a Type, &'a str, &'a [Captured]), String> {
        let ty = self.context.reader.ty(closure
            .ty
            .as_deref()
            .ok_or("Callback has no compiler closure type")?)?;
        if ty.kind != TypeKind::Closure {
            return Err("Callback has no compiler closure type".into());
        }
        let environment = ty
            .environment
            .as_deref()
            .ok_or("Callback has no compiler environment")?;
        let captures = closure.captures.as_deref().unwrap_or_default();
        if captures.len() > MAX_FIELDS {
            return Err("Callback environment exceeds concrete field budget".into());
        }
        Ok((ty, environment, captures))
    }

    fn option_environment(
        &mut self,
        node: &HirNode,
        shared_owner: bool,
    ) -> Result<PathRes, String> {
        let closure = self.closure(node)?;
        let (_, environment, captures) = self.environment_parts(closure)?;
        let shared_place = captures
            .first()
            .filter(|_| shared_owner && captures.len() == 1)
            .filter(|place| self.shared_formal_place(place, environment));
        let shared_formal = self.shared_formal_capture(node, shared_place)?;
        let stored_type = self.callback_storage(environment, shared_formal.is_some())?;
        let environment = records::Environment {
            captures: captures
                .iter()
                .map(|place| records::Capture {
                    root: super::super::hir_ref(&place.symbol),
                    mode: mode(place),
                    projections: place.path.len(),
                })
                .collect(),
            stored_type,
            shared_formal,
        };
        Ok(PathRes {
            kind: records::ENVIRONMENT_KIND.into(),
            target: None,
            description: serde_json::to_string(&environment).map_err(|error| error.to_string())?,
        })
    }

    fn callback_storage(
        &mut self,
        environment: &'a str,
        shared_formal: bool,
    ) -> Result<Observation, String> {
        let reader = self.context.reader;
        let byte = |id: &str| {
            reader.ty(id).is_ok_and(|ty| {
                ty.kind == TypeKind::Reference
                    && ty.mode.as_deref() == Some("shared")
                    && ty
                        .target
                        .as_deref()
                        .and_then(|target| reader.ty(target).ok())
                        .is_some_and(|target| {
                            target.kind == TypeKind::Integer && target.spelling == "u8"
                        })
            })
        };
        let supported = reader.ty(environment).is_ok_and(|ty| {
            ty.kind == TypeKind::Tuple && matches!(ty.items.as_deref(), Some([item]) if byte(item))
        });
        if supported || shared_formal {
            return Ok(self.observe(environment)?.0);
        }
        Ok(Observation::Unsupported {
            reason: "Callback environment is not exactly one shared primitive byte".into(),
        })
    }

    /// A shared capture, through one dereference, of an immutable formal that is a
    /// shared reference to a struct, stored as that reference.
    fn shared_formal_place(&self, place: &Captured, environment: &str) -> bool {
        let reader = self.context.reader;
        let Ok(symbol) = self.context.body.symbol(&place.symbol) else {
            return false;
        };
        let Some(root) = symbol.ty.as_deref() else {
            return false;
        };
        let Ok(root_type) = reader.ty(root) else {
            return false;
        };
        let Some(referent) = root_type.target.as_deref() else {
            return false;
        };
        let structure = reader
            .ty(referent)
            .ok()
            .filter(|referent| referent.kind == TypeKind::Record)
            .and_then(|referent| referent.definition.as_deref())
            .and_then(|definition| reader.tables.definition(definition).ok())
            .and_then(|definition| {
                definition
                    .extensions
                    .get("rust:compiler-identity")?
                    .get("def_kind")?
                    .as_str()
                    .map(|kind| kind == "Struct")
            })
            .unwrap_or(false);
        let stored = reader.ty(environment).is_ok_and(|ty| {
            ty.kind == TypeKind::Tuple
                && matches!(ty.items.as_deref(), Some([item]) if item == root)
        });
        place.mode == "shared"
            && self.immutable_formal(symbol)
            && root_type.kind == TypeKind::Reference
            && root_type.mode.as_deref() == Some("shared")
            && structure
            && matches!(place.path.as_slice(), [step] if step.kind == "deref" && step.ty == referent)
            && stored
    }

    /// A by-value, immutable binding that is a whole parameter of the function.
    fn immutable_formal(&self, symbol: &Symbol) -> bool {
        let signature = &self.context.body.function.signature;
        let plain = signature.parameters.iter().any(|parameter| {
            parameter.symbol == symbol.id
                && parameter.original_kind.as_deref() == Some("pat_binding")
        }) || signature
            .receiver
            .as_ref()
            .is_some_and(|receiver| receiver.symbol == symbol.id);
        plain && !symbol.mutable
    }

    fn shared_formal_capture(
        &mut self,
        node: &HirNode,
        place: Option<&'a Captured>,
    ) -> Result<Option<records::SharedCapture>, String> {
        if !self.supports("rust.shared-helper-option-callbacks") {
            return Ok(None);
        }
        let Some(place) = place else {
            return Ok(None);
        };
        let root = super::super::hir_ref(&place.symbol);
        if !shared_helper_body(node, &root) {
            return Ok(None);
        }
        let symbol = self.context.body.symbol(&place.symbol)?;
        let base = symbol
            .ty
            .as_deref()
            .ok_or("Missing shared capture root type")?;
        let root_type = self.observe(base)?.0;
        let projected = records::SharedProjection::SharedDeref {
            before: root_type.clone(),
            after: self.observe(&place.path[0].ty)?.0,
        };
        let mut paths = Vec::new();
        crate::frontend::collect_kind(node, "expr_path", &mut paths);
        let uses = paths
            .into_iter()
            .filter(|path| {
                path.res.as_ref().is_some_and(|resolution| {
                    resolution.kind == "local" && resolution.target.as_ref() == Some(&root)
                })
            })
            .map(|path| {
                let ty = self
                    .hir_type(&path.hir)
                    .ok_or("Missing shared capture use type")?;
                Ok(records::SharedUse {
                    hir: path.hir.clone(),
                    ty: self.observe(ty)?.0,
                })
            })
            .collect::<Result<_, String>>()?;
        Ok(Some(records::SharedCapture {
            closure: node.hir.clone(),
            root_type,
            projections: vec![projected],
            uses,
        }))
    }

    /// A `Result` callback's environment, captures and signature, as the reader
    /// contracts record them.
    fn reader_environment(
        &mut self,
        closure: &'a Node,
        identity: &str,
    ) -> Result<reader_records::Environment, String> {
        let (ty, environment, captures) = self.environment_parts(closure)?;
        let definition = self.context.reader.tables.definition(
            ty.definition
                .as_deref()
                .ok_or("Missing closure definition")?,
        )?;
        let definition_identity = definition
            .extensions
            .get("rust:compiler-identity")
            .and_then(|identity| identity.get("def_path_hash")?.as_str())
            .ok_or("Missing closure definition identity")?
            .to_owned();
        let body = self
            .context
            .body
            .children(closure)?
            .last()
            .map(|body| super::super::hir_ref(&body.id))
            .ok_or("Missing closure body")?;
        let captures = captures
            .iter()
            .map(|place| self.reader_capture(place))
            .collect::<Result<_, String>>()?;
        let stored_type = self.observe(environment)?.0;
        let signature_inputs = ty
            .parameters
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|parameter| Ok(self.observe(parameter)?.0))
            .collect::<Result<_, String>>()?;
        let signature_output = self
            .observe(ty.result.as_deref().ok_or("Missing closure result")?)?
            .0;
        Ok(reader_records::Environment {
            identity: identity.into(),
            closure: super::super::hir_ref(&closure.id),
            body,
            definition_identity,
            captures,
            stored_type,
            // A closure's signature is always called through the compiler's tupled ABI.
            signature_abi: "RustCall".into(),
            signature_inputs,
            signature_output,
        })
    }

    fn reader_capture(&mut self, place: &'a Captured) -> Result<reader_records::Capture, String> {
        let symbol = self.context.body.symbol(&place.symbol)?;
        let base = symbol.ty.as_deref().ok_or("Missing capture root type")?;
        let mut before = base;
        let mut projections = Vec::new();
        for step in &place.path {
            projections.push(reader_records::Projection {
                kind: if step.kind == "deref" {
                    "deref"
                } else {
                    "unsupported"
                }
                .into(),
                before: self.observe(before)?.0,
                after: self.observe(&step.ty)?.0,
            });
            before = &step.ty;
        }
        Ok(reader_records::Capture {
            root: super::super::hir_ref(&place.symbol),
            mode: mode(place),
            root_type: self.observe(base)?.0,
            projections,
        })
    }
}

fn mode(place: &Captured) -> CaptureMode {
    match place.mode.as_str() {
        "shared" => CaptureMode::Shared,
        "unique" => CaptureMode::Unique,
        "exclusive" => CaptureMode::Mutable,
        "use" => CaptureMode::Use,
        _ => CaptureMode::Value,
    }
}

fn immediate_map_or_callback(node: &HirNode) -> Option<HirRef> {
    if node.kind != "expr_method_call" || node.children.len() != 3 {
        return None;
    }
    let resolution = node.res.as_ref()?;
    let callee: crate::data_library::Callee = serde_json::from_str(&resolution.description).ok()?;
    if callee.crate_name != "core"
        || callee.implementation.is_some()
        || !matches!(
            callee.path.as_str(),
            "core::option::Option::<T>::map_or" | "std::option::Option::<T>::map_or"
        )
    {
        return None;
    }
    node.children.last().map(|callback| callback.hir.clone())
}

fn shared_helper_body(node: &HirNode, root: &HirRef) -> bool {
    let [pattern, body] = node.children.as_slice() else {
        return false;
    };
    let binding = if pattern.kind == "pat_shared_copy" {
        let [binding] = pattern.children.as_slice() else {
            return false;
        };
        binding
    } else {
        pattern
    };
    if node.expansion.is_some()
        || body.expansion.is_some()
        || body.kind != "expr_method_call"
        || body.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("bool")
        || !body
            .res
            .as_ref()
            .is_some_and(|res| res.kind == "resolved_inherent_callee")
        || binding.kind != "pat_binding"
        || binding.binding_mut != Some(false)
        || binding.param_index.is_some()
        || binding.binding.is_none()
        || !binding.children.is_empty()
        || binding.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) != Some("u8")
    {
        return false;
    }
    let [receiver, payload] = body.children.as_slice() else {
        return false;
    };
    capture_path(receiver, root) && capture_path(payload, &binding.hir)
}

fn capture_path(node: &HirNode, target: &HirRef) -> bool {
    node.kind == "expr_path"
        && node.expansion.is_none()
        && node.children.is_empty()
        && node.res.as_ref().is_some_and(|resolution| {
            resolution.kind == "local" && resolution.target.as_ref() == Some(target)
        })
}
