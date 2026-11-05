//! Each method call's receiver, as the data profile observes it: the receiver's type
//! before and after its implicit adjustments, and the callee's instantiated signature.

use super::Capture;
use crate::concrete_types::{Observation, TypeTree};
use crate::data_receivers::{self as records, Adjustment, Kind, Method};
use crate::frontend::{HirFnFacts, HirNode};
use rstyle_core::cpg::model::{Adjustment as Adjusted, Node, TypeKind};

impl<'a> Capture<'_, '_, 'a> {
    pub(super) fn install_receivers(&mut self, facts: &mut HirFnFacts) -> Result<(), String> {
        let mut methods = Vec::new();
        self.receiver_records(&facts.body, &mut methods)?;
        let capture = records::Capture {
            identity: records::IDENTITY.into(),
            methods,
        };
        facts.build.frontend_options.insert_data(capture);

        Ok(())
    }

    fn receiver_records(
        &mut self,
        node: &HirNode,
        methods: &mut Vec<Method>,
    ) -> Result<(), String> {
        if node.kind == "expr_method_call" {
            if methods.len() >= records::MAX_RECORDS {
                return Err("Method receiver capture exceeds record budget".into());
            }
            methods.push(self.receiver_record(node)?);
        }
        for child in &node.children {
            self.receiver_records(child, methods)?;
        }
        Ok(())
    }

    // rstyle-allow(mutation_per_target): each observed type shares one budgeted observer
    fn receiver_record(&mut self, node: &HirNode) -> Result<Method, String> {
        let call = self.cpg_node(node)?;
        let receiver = self.context.body.child(call, 0)?;
        let signature = self
            .context
            .body
            .reference(&call.id)
            .and_then(|reference| reference.signature.as_ref())
            .ok_or("Missing resolved method definition")?;
        let ascii = self.character_ascii(call, receiver);
        let adjustments = receiver.adjustments.as_deref().unwrap_or_default();
        if adjustments.len() > records::MAX_ADJUSTMENTS {
            return Err("Method receiver capture exceeds adjustment budget".into());
        }
        let adjustments = adjustments
            .iter()
            .map(|adjustment| {
                let target = adjustment
                    .target
                    .as_deref()
                    .ok_or("Missing adjustment target")?;
                Ok(Adjustment {
                    kind: kind(adjustment),
                    target: self.method_observation(target, ascii)?,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let source = receiver.ty.as_deref().ok_or("Missing receiver type")?;
        let adjusted = receiver
            .adjustments
            .as_deref()
            .and_then(|adjustments| adjustments.last()?.target.as_deref())
            .unwrap_or(source);

        Ok(Method {
            call: node.hir.clone(),
            source_span: node.span,
            receiver: super::super::hir_ref(&receiver.id),
            receiver_type: self.observe(source)?.0,
            adjusted_type: self.method_observation(adjusted, ascii)?,
            adjustments,
            signature_inputs: signature
                .parameters
                .iter()
                .map(|ty| self.signature_type(ty.as_deref(), ascii))
                .collect::<Result<_, _>>()?,
            signature_result: self.signature_type(signature.result.as_deref(), ascii)?,
        })
    }

    /// `char::is_ascii`, which the profile observes through a shared character.
    fn character_ascii(&self, call: &Node, receiver: &Node) -> bool {
        let reader = self.context.reader;
        let Some(reference) = self.context.body.reference(&call.id) else {
            return false;
        };
        let callee = reference
            .target
            .as_ref()
            .and_then(|target| target.id.as_deref())
            .or(reference.via.as_deref())
            .and_then(|id| reader.tables.definition(id).ok());
        let diagnostic = callee.and_then(|callee| super::super::identity(callee, "diagnostic"));
        let character = |id: Option<&str>| {
            id.and_then(|id| reader.ty(id).ok())
                .is_some_and(|ty| ty.kind == TypeKind::Char)
        };
        let Some(signature) = reference.signature.as_ref() else {
            return false;
        };
        let boolean = signature
            .result
            .as_deref()
            .and_then(|id| reader.ty(id).ok())
            .is_some_and(|ty| ty.kind == TypeKind::Bool);
        let shared = matches!(signature.parameters.as_slice(), [Some(parameter)] if self.shared_character(parameter));

        character(receiver.ty.as_deref())
            && self.supports("rust.character-ascii-predicate")
            && diagnostic == Some("char_is_ascii")
            && boolean
            && shared
    }

    fn shared_character(&self, id: &str) -> bool {
        let reader = self.context.reader;
        reader.ty(id).is_ok_and(|ty| {
            ty.kind == TypeKind::Reference
                && ty.mode.as_deref() == Some("shared")
                && ty
                    .target
                    .as_deref()
                    .and_then(|target| reader.ty(target).ok())
                    .is_some_and(|target| target.kind == TypeKind::Char)
        })
    }

    fn signature_type(&mut self, ty: Option<&'a str>, ascii: bool) -> Result<Observation, String> {
        match ty {
            Some(ty) => self.method_observation(ty, ascii),
            None => Ok(Observation::Unsupported {
                reason: "Method signature type cannot be normalized".into(),
            }),
        }
    }

    fn method_observation(&mut self, ty: &'a str, ascii: bool) -> Result<Observation, String> {
        if ascii && self.shared_character(ty) {
            return Ok(Observation::Captured {
                ty: TypeTree::Shared(Box::new(TypeTree::Scalar("char".into()))),
            });
        }
        Ok(self.observe(ty)?.0)
    }
}

/// An adjustment as the receiver record names it.
fn kind(adjustment: &Adjusted) -> Kind {
    let unsupported = |name: &str| Kind::Unsupported(name.into());
    match (adjustment.kind.as_str(), adjustment.mode.as_deref()) {
        ("auto_deref", _) => Kind::BuiltinDereference,
        ("auto_ref", Some("shared")) => Kind::SharedBorrow,
        ("auto_ref", Some("exclusive")) => Kind::MutableBorrow,
        ("auto_ref", _) => unsupported("raw_borrow"),
        ("rust:never-to-any", _) => unsupported("never_coercion"),
        ("overloaded_deref", _) => unsupported("overloaded_dereference"),
        ("rust:pin-deref", _) => unsupported("pin_dereference"),
        ("pin", _) => unsupported("pin_borrow"),
        ("reborrow", _) => unsupported("generic_reborrow"),
        _ => unsupported("pointer_coercion"),
    }
}
