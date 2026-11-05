//! The data profiles' concrete type observations: the frontend's type grammar for
//! a function's interface, and the type tokens it installs on the signature, body
//! and bindings, rebuilt from the document's type table.

mod callbacks;
mod enums;
mod generated;
mod origins;
mod products;
mod receivers;
mod storage;

use super::Context;
use crate::concrete_types::{
    Field, IDENTITY, InterfaceObservations, MAX_DEPTH, MAX_ENTRIES, MAX_FIELDS, MAX_SOURCE_BYTES,
    Nominal, Observation, TypeTree,
};
use crate::extraction_profile::ExtractionProfile;
use crate::frontend::{BindingFact, HirFnFacts, HirNode, HirRef};
use rstyle_core::cpg::model::{Definition, Type, TypeKind};
use std::collections::BTreeMap;

/// Which of the frontend's capture passes the facts went through.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Passes {
    /// Advisory consumers: the interface's types only.
    Advisory,
    /// Advisory consumers that also read the body's local mutation types.
    AdvisoryMutations,
    /// The complete data capture: the body and bindings too.
    Complete,
}

enum Failure {
    Unsupported(String),
    Budget(String),
}

fn unsupported(reason: &str) -> Failure {
    Failure::Unsupported(reason.into())
}

impl<'a> Context<'_, 'a> {
    // rstyle-allow(mutation_per_target): one observer installs the interface, body and binding tokens in turn
    /// Installs the concrete type observations the frontend installs on `facts`.
    pub(super) fn concrete_types(
        &self,
        facts: &mut HirFnFacts,
        passes: Passes,
    ) -> Result<(), String> {
        if !has_data_interface(facts, self.reader.profile) {
            return Ok(());
        }
        let mut capture = Capture {
            context: self,
            types: self.hir_types(),
            entries: 0,
            visiting: Vec::new(),
            completed: BTreeMap::new(),
            origins: BTreeMap::new(),
            generated_origins: BTreeMap::new(),
            product_selections: Vec::new(),
        };
        let observations = capture.interface(facts)?;
        let data = self.reader.profile.supports("rust.concrete-read-only-data");
        if data {
            interface_tokens(facts, &observations)?;
            match passes {
                Passes::Complete => {
                    capture.node_tokens(&mut facts.body, false)?;
                    capture.binding_tokens(&mut facts.bindings, false)?;
                    capture.install_receivers(facts)?;
                    capture.callback_environments(&mut facts.body)?;
                }
                Passes::Advisory | Passes::AdvisoryMutations => {
                    capture.binding_tokens(&mut facts.bindings, true)?;
                }
            }
        }
        // The complete capture records where the observed declarations are, whatever
        // the profile installs.
        if passes == Passes::Complete {
            capture.install_product_selections(facts)?;
            capture.install_origins(&observations, facts)?;
        }
        facts.build.frontend_options.insert_data(observations);

        Ok(())
    }

    /// Each HIR node's and binding's value type, as the compiler's typeck records it.
    pub(super) fn hir_types(&self) -> BTreeMap<(u32, u32), &'a str> {
        let function = self.body.function;
        let nodes = function
            .nodes
            .iter()
            .filter_map(|node| Some((&node.id, node.ty.as_deref()?)));
        let symbols = function
            .symbols
            .iter()
            .filter_map(|symbol| Some((&symbol.id, symbol.ty.as_deref()?)));
        let patterns = function
            .signature
            .parameters
            .iter()
            .filter_map(|parameter| Some((parameter.pattern.as_ref()?, parameter.ty.as_str())));

        // A binding's own type, as typeck records it, wins over its parameter's.
        patterns
            .chain(nodes)
            .chain(symbols)
            .map(|(id, ty)| {
                let reference = super::hir_ref(id);
                ((reference.owner, reference.local), ty)
            })
            .collect()
    }
}

fn has_data_interface(facts: &HirFnFacts, profile: ExtractionProfile) -> bool {
    profile.supports("rust.concrete-read-only-data")
        || profile.supports("rust.verified-helper-calls")
            && (facts.signature.return_ty.normalized.is_none()
                || facts
                    .signature
                    .params
                    .iter()
                    .any(|parameter| parameter.ty.normalized.is_none()))
}

// rstyle-allow(side_effect_density): installs each observed interface token in place
fn interface_tokens(
    facts: &mut HirFnFacts,
    interface: &InterfaceObservations,
) -> Result<(), String> {
    let signature = &mut facts.signature;
    for (parameter, observation) in signature.params.iter_mut().zip(&interface.parameters) {
        if let Observation::Captured { ty } = observation {
            parameter.ty.normalized = Some(crate::data_types::token(ty)?);
            parameter.pat.ty = Some(parameter.ty.clone());
        }
    }
    if let Observation::Captured { ty } = &interface.result {
        signature.return_ty.normalized = Some(crate::data_types::token(ty)?);
    }
    Ok(())
}

struct Capture<'c, 'r, 'a> {
    context: &'c Context<'r, 'a>,
    types: BTreeMap<(u32, u32), &'a str>,
    entries: usize,
    /// The structs being expanded, which a field of theirs may not expand again.
    visiting: Vec<&'a str>,
    /// Each root type's completed observation and token.
    completed: BTreeMap<&'a str, (Observation, Option<String>)>,
    /// Where each observed local declaration is written, by definition path.
    origins: BTreeMap<String, crate::declaration_origins::Origin>,
    /// The nominal product constants the body's paths select, in walk order.
    product_selections: Vec<crate::product_constant_values::SelectedUse>,
    /// Where each observed generated declaration's template is, by definition path.
    generated_origins: BTreeMap<String, crate::declaration_origins::GeneratedOrigin>,
}

impl<'a> Capture<'_, '_, 'a> {
    fn interface(&mut self, facts: &HirFnFacts) -> Result<InterfaceObservations, String> {
        let parameters = facts
            .signature
            .params
            .iter()
            .map(|parameter| {
                let ty = self
                    .hir_type(&parameter.pat.hir)
                    .ok_or("Missing concrete parameter type")?;
                Ok(self.observe(ty)?.0)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let result = self.context.body.function.signature.result.as_str();
        let observations = InterfaceObservations {
            identity: IDENTITY.into(),
            parameters,
            result: self.observe(result)?.0,
        };
        observations.validate()?;

        Ok(observations)
    }

    fn hir_type(&self, reference: &HirRef) -> Option<&'a str> {
        self.types.get(&(reference.owner, reference.local)).copied()
    }

    fn observe(&mut self, ty: &'a str) -> Result<(Observation, Option<String>), String> {
        if let Some(completed) = self.completed.get(ty) {
            return Ok(completed.clone());
        }
        let observation = match self.tree(ty, 0) {
            Ok(tree) => Observation::Captured { ty: tree },
            Err(Failure::Unsupported(reason)) => Observation::Unsupported { reason },
            Err(Failure::Budget(reason)) => return Err(reason),
        };
        let token = match &observation {
            Observation::Captured { ty } => Some(crate::data_types::token(ty)?),
            Observation::Unsupported { .. } => None,
        };
        self.completed
            .insert(ty, (observation.clone(), token.clone()));

        Ok((observation, token))
    }

    fn token(&mut self, reference: &HirRef, returning: bool) -> Result<Option<String>, String> {
        let ty = match returning {
            true => Some(self.context.body.function.signature.result.as_str()),
            false => self.hir_type(reference),
        };
        match ty {
            Some(ty) => Ok(self.observe(ty)?.1),
            None => Ok(None),
        }
    }

    fn binding_tokens(
        &mut self,
        bindings: &mut [BindingFact],
        formals_only: bool,
    ) -> Result<(), String> {
        for binding in bindings {
            if (!formals_only || binding.is_param) && binding.ty.normalized.is_none() {
                binding.ty.normalized = self.token(&binding.hir, false)?;
            }
        }
        Ok(())
    }

    // rstyle-allow(mutation_per_target): installs each token on the node and its children in turn
    fn node_tokens(&mut self, node: &mut HirNode, index_bound: bool) -> Result<(), String> {
        self.product_constant(node)?;
        let range_syntax = index_bound
            && node.kind == "expr_struct"
            && node
                .res
                .as_ref()
                .is_some_and(|resolution| resolution.kind == "resolved_range_type");
        if !range_syntax && node.ty.as_ref().is_some_and(|ty| ty.normalized.is_none()) {
            let token = self.token(&node.hir, node.kind == "expr_ret")?;
            node.ty.as_mut().expect("present type").normalized = token;
        }
        let bound = node
            .children
            .get(1)
            .filter(|_| node.kind == "expr_index")
            .map(|child| child.hir.clone());
        for child in &mut node.children {
            let bounded = bound.as_ref() == Some(&child.hir);
            self.node_tokens(child, bounded)?;
        }
        let profile = self.context.reader.profile;
        if matches!(node.kind.as_str(), "expr_method_call" | "expr_call")
            || node.kind == "expr_cast" && profile.supports("rust.fieldless-enum-values")
        {
            call_tokens(node);
        }

        Ok(())
    }

    fn ty(&self, id: &str) -> Result<&'a Type, Failure> {
        self.context.reader.ty(id).map_err(Failure::Unsupported)
    }

    fn supports(&self, capability: &str) -> bool {
        self.context.reader.profile.supports(capability)
    }

    fn tree(&mut self, id: &'a str, depth: usize) -> Result<TypeTree, Failure> {
        self.entries += 1;
        if depth > MAX_DEPTH || self.entries > MAX_ENTRIES {
            return Err(Failure::Budget(format!(
                "Concrete type observation exceeds depth/entry budget (depth {depth}/{MAX_DEPTH}, entries {}/{MAX_ENTRIES})",
                self.entries
            )));
        }
        let ty = self.ty(id)?;
        match ty.kind {
            TypeKind::Bottom
                if self.supports("rust.u16-usize-reader-observations")
                    && self.context.usize_reader_result() =>
            {
                Ok(TypeTree::NeverError)
            }
            TypeKind::Integer | TypeKind::Bool => Ok(TypeTree::Scalar(ty.spelling.clone())),
            TypeKind::Char if depth == 0 && self.supports("rust.character-scalars") => {
                Ok(TypeTree::Scalar("char".into()))
            }
            TypeKind::Float
                if depth == 0
                    && matches!(ty.bits, Some(32 | 64))
                    && self.supports("rust.float-kernel-scalars") =>
            {
                Ok(TypeTree::Scalar(ty.spelling.clone()))
            }
            TypeKind::Unit => Ok(TypeTree::Scalar("unit".into())),
            TypeKind::String => Ok(TypeTree::Str),
            _ => self.composite(ty, depth),
        }
    }

    // rstyle-allow(mutation_per_target): recursive observation shares one budgeted observer
    fn composite(&mut self, ty: &'a Type, depth: usize) -> Result<TypeTree, Failure> {
        let mode = ty.mode.as_deref();
        match ty.kind {
            TypeKind::Reference
                if mode == Some("exclusive")
                    && depth == 0
                    && self.supports("rust.mutable-scalar-formals")
                    && self.context.reader.integer(ty.target.as_deref(), "usize") =>
            {
                Ok(TypeTree::MutableScalar("usize".into()))
            }
            TypeKind::Tuple => {
                let items = ty.items.as_deref().unwrap_or_default();
                let trees = items.iter().map(|item| self.tree(item, depth + 1));
                trees.collect::<Result<Vec<_>, _>>().map(TypeTree::Tuple)
            }
            TypeKind::Reference if mode == Some("shared") => {
                let target = ty
                    .target
                    .as_deref()
                    .ok_or_else(|| unsupported("Reference has no target"))?;
                Ok(TypeTree::Shared(Box::new(self.tree(target, depth + 1)?)))
            }
            TypeKind::Sequence if ty.fixed != Some(true) => {
                let element = ty
                    .element
                    .as_deref()
                    .ok_or_else(|| unsupported("Slice has no element"))?;
                Ok(TypeTree::Slice(Box::new(self.tree(element, depth + 1)?)))
            }
            _ => self.storage(ty, depth),
        }
    }

    // rstyle-allow(mutation_per_target): recursive observation shares one budgeted observer
    fn storage(&mut self, ty: &'a Type, depth: usize) -> Result<TypeTree, Failure> {
        if let Some(carrier) = self.carrier(ty, depth)? {
            return Ok(carrier);
        }
        if self.context.reader.profile == ExtractionProfile::DataV3
            && let Some(payload) = self
                .option_payload(ty)
                .or_else(|| self.nonnested_payload(ty))
        {
            let option = TypeTree::Option(Box::new(self.tree(payload, depth + 1)?));
            crate::concrete_types::validate(&option).map_err(Failure::Unsupported)?;
            return Ok(option);
        }
        if let Some(result) = self.observed_result(ty, depth)? {
            return Ok(result);
        }
        match ty.kind {
            TypeKind::Sequence => {
                let element = ty
                    .element
                    .as_deref()
                    .ok_or_else(|| unsupported("Array has no element"))?;
                let length = ty
                    .length
                    .ok_or_else(|| unsupported("Array length is not a resolved concrete value"))?;
                Ok(TypeTree::Array {
                    element: Box::new(self.tree(element, depth + 1)?),
                    length,
                })
            }
            TypeKind::Record => self.structure(ty, depth),
            TypeKind::Sum if !self.supports("rust.scalar-enum-values") => self.structure(ty, depth),
            TypeKind::Sum => self.enumeration(ty, depth),
            // An owned box is a library struct to the frontend.
            TypeKind::Pointer if ty.mode.as_deref() == Some("owned") && ty.definition.is_some() => {
                self.structure(ty, depth)
            }
            _ => Err(unsupported(
                "Outside the concrete drop-free/shared type observation grammar",
            )),
        }
    }

    // rstyle-allow(mutation_per_target): recursive observation shares one budgeted observer
    /// A local struct with its declaration as written and its fields' types.
    fn structure(&mut self, ty: &'a Type, depth: usize) -> Result<TypeTree, Failure> {
        if let Some(captured) = self.scalar_product_instance(ty, depth)? {
            return Ok(captured);
        }
        let (definition, fields) = self.checked_structure(ty)?;
        if let Some(generated) = definition.extensions.get("rust:generated-declaration") {
            return self.generated_structure((definition, fields), generated, depth);
        }
        let source = self.declaration(definition)?;
        if self.context.reader.profile.is_data() {
            crate::concrete_types::lifetimes::declaration_count(&source)
                .map_err(Failure::Unsupported)?;
        }
        let declarations =
            crate::concrete_types::field_declarations(&source).map_err(Failure::Unsupported)?;
        if declarations.len() != fields.len() {
            return Err(unsupported("Concrete field/source inventory disagrees"));
        }

        self.visiting.push(definition.id.as_str());
        let captured = self.fields(fields, declarations, depth);
        self.visiting.pop();
        Ok(TypeTree::Struct {
            nominal: self.origin_nominal(definition, &source)?,
            source,
            fields: captured?,
        })
    }

    /// A struct the grammar admits: local, concrete, destructor-free and within budget.
    fn checked_structure(
        &self,
        ty: &'a Type,
    ) -> Result<(&'a Definition, &'a [rstyle_core::cpg::model::TypeField]), Failure> {
        let definition = self.definition_of(ty)?;
        // A data profile admits a struct whose only arguments are lifetimes.
        let lifetimes = ty.modifiers.iter().any(|m| m == "rust:lifetime-arguments");
        let own = definition
            .extensions
            .get("rust:compiler-identity")
            .and_then(|identity| identity.get("generics")?.get("own")?.as_array().cloned())
            .unwrap_or_default();
        // A data profile budgets a struct's lifetime arguments, as it does its fields.
        let profile = self.context.reader.profile;
        if profile.is_data() && own.len() > MAX_FIELDS && own.iter().all(|kind| kind == "lifetime")
        {
            return Err(Failure::Budget(
                "Nominal lifetime arguments exceed count budget".into(),
            ));
        }
        // Type and const parameters both make a struct generic; only types are arguments.
        let parameters = own.iter().any(|kind| kind != "lifetime");
        let generic = parameters
            || ty
                .arguments
                .as_ref()
                .is_some_and(|arguments| !arguments.is_empty())
            || lifetimes && !profile.is_data();
        if ty.kind != TypeKind::Record
            || definition.scope != rstyle_core::cpg::model::Scope::Unit
            || generic
            || definition.destructor
        {
            return Err(unsupported(
                "Struct requires local concrete source, no generics/destructor/interior mutability",
            ));
        }
        if self.visiting.contains(&definition.id.as_str()) {
            return Err(unsupported(
                "Recursive concrete type expansion is unavailable",
            ));
        }
        let fields = ty.fields.as_deref().unwrap_or_default();
        if fields.len() > MAX_FIELDS {
            return Err(Failure::Budget(
                "Concrete struct exceeds field budget".into(),
            ));
        }

        Ok((definition, fields))
    }

    fn definition_of(&self, ty: &Type) -> Result<&'a Definition, Failure> {
        ty.definition
            .as_deref()
            .and_then(|id| self.context.reader.tables.definition(id).ok())
            .ok_or_else(|| unsupported("Missing nominal definition"))
    }

    /// A local type's declaration as written, within the source budget.
    fn declaration(&self, definition: &Definition) -> Result<crate::SourceText, Failure> {
        let span = definition
            .span
            .as_ref()
            .ok_or_else(|| unsupported("Macro-generated struct declaration is unavailable"))?;
        let source = self
            .context
            .reader
            .source(span)
            .map_err(|_| unsupported("Missing original concrete declaration bytes"))?;
        if source.len() > MAX_SOURCE_BYTES {
            return Err(Failure::Budget(
                "Concrete declaration exceeds source budget".into(),
            ));
        }
        Ok(source)
    }

    fn fields(
        &mut self,
        fields: &'a [rstyle_core::cpg::model::TypeField],
        declarations: Vec<(String, String)>,
        depth: usize,
    ) -> Result<Vec<Field>, Failure> {
        let disagreement = "Resolved field identity disagrees with original declaration";
        self.declared_fields((fields, declarations), depth, disagreement)
    }

    /// Fields observed in declaration order, each checked against its written name.
    fn declared_fields(
        &mut self,
        (fields, declarations): (
            &'a [rstyle_core::cpg::model::TypeField],
            Vec<(String, String)>,
        ),
        depth: usize,
        disagreement: &str,
    ) -> Result<Vec<Field>, Failure> {
        fields
            .iter()
            .zip(declarations)
            .enumerate()
            .map(|(position, (field, (name, declaration)))| {
                if field.name != name {
                    return Err(unsupported(disagreement));
                }
                Ok(Field {
                    position,
                    name,
                    declaration: declaration.into(),
                    ty: self.tree(&field.ty, depth + 1)?,
                })
            })
            .collect()
    }

    /// A struct's package and declaration identity, as the frontend names it.
    fn nominal(
        &self,
        definition_path: &str,
        source: &crate::SourceText,
    ) -> Result<Nominal, Failure> {
        nominal(self.context.reader, definition_path, source)
    }

    /// A standard library type's arguments, when `key` names the type.
    fn standard_arguments(&self, ty: &'a Type, key: &str) -> Option<&'a [String]> {
        let standard = ty
            .definition
            .as_deref()
            .and_then(|id| self.context.reader.tables.definition(id).ok())
            .is_some_and(|definition| definition.standard);
        (standard && ty.key.as_deref() == Some(key))
            .then_some(ty.arguments.as_deref())
            .flatten()
    }

    fn option_payload(&self, ty: &'a Type) -> Option<&'a str> {
        let [payload] = self.standard_arguments(ty, "rust:Option")? else {
            return None;
        };
        let payload_type = self.ty(payload).ok()?;
        let scalar = |ty: &Type| matches!(ty.kind, TypeKind::Bool | TypeKind::Integer);
        let shared_scalar = payload_type.kind == TypeKind::Reference
            && payload_type.mode.as_deref() == Some("shared")
            && payload_type
                .target
                .as_deref()
                .and_then(|target| self.ty(target).ok())
                .is_some_and(scalar);

        (scalar(payload_type) || shared_scalar).then_some(payload.as_str())
    }

    /// A product or borrowed byte slice `Option` payload, at the root of an observation.
    fn nonnested_payload(&self, ty: &'a Type) -> Option<&'a str> {
        if !self.visiting.is_empty() {
            return None;
        }
        let [payload] = self.standard_arguments(ty, "rust:Option")? else {
            return None;
        };
        let payload_type = self.ty(payload).ok()?;
        // A tuple or any nominal type, as the compiler's ADTs are, the library's
        // containers and `Option` itself included.
        let nominal = payload_type.definition.is_some()
            && !matches!(
                payload_type.kind,
                TypeKind::Function
                    | TypeKind::Closure
                    | TypeKind::Interface
                    | TypeKind::Opaque
                    | TypeKind::Parameter
            );
        let product = self.supports("rust.concrete-option-propagation")
            && (matches!(payload_type.kind, TypeKind::Tuple | TypeKind::Unit) || nominal);
        let borrowed_bytes = self.supports("rust.checked-borrowed-result-flow")
            && payload_type.kind == TypeKind::Reference
            && payload_type.mode.as_deref() == Some("shared")
            && payload_type
                .target
                .as_deref()
                .and_then(|target| self.ty(target).ok())
                .is_some_and(|slice| {
                    slice.kind == TypeKind::Sequence
                        && slice.fixed != Some(true)
                        && slice
                            .element
                            .as_deref()
                            .and_then(|element| self.ty(element).ok())
                            .is_some_and(|element| {
                                element.kind == TypeKind::Integer && element.spelling == "u8"
                            })
                });

        (product || borrowed_bytes).then_some(payload.as_str())
    }

    fn result_payloads(&self, ty: &'a Type) -> Option<(&'a str, &'a str)> {
        let [ok, err] = self.standard_arguments(ty, "rust:Result")? else {
            return None;
        };
        Some((ok.as_str(), err.as_str()))
    }
}

/// A call's dispatch evidence refreshed with its operands' and result's tokens.
fn call_tokens(node: &mut HirNode) {
    let complete = node.ty.as_ref().is_some_and(|ty| ty.normalized.is_some())
        && node
            .children
            .iter()
            .all(|child| child.ty.as_ref().is_some_and(|ty| ty.normalized.is_some()));
    if !complete && !original_free_call(node) {
        return;
    }
    let Some(dispatch) = &mut node.dispatch else {
        return;
    };
    let operands = dispatch
        .operand_tys
        .iter()
        .enumerate()
        .map(|(position, declared)| {
            node.children
                .get(position)
                .and_then(|child| child.ty.as_ref())
                .and_then(|ty| ty.normalized.as_ref())
                .unwrap_or(declared)
                .clone()
        })
        .collect();
    dispatch.operand_tys = operands;

    if let Some(token) = node.ty.as_ref().and_then(|ty| ty.normalized.as_deref()) {
        token.clone_into(&mut dispatch.result_ty);
    }
}

fn original_free_call(node: &HirNode) -> bool {
    node.kind == "expr_call"
        && node.expansion.is_none()
        && node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.kind == "resolved_direct")
        && node
            .res
            .as_ref()
            .is_some_and(|resolution| resolution.kind == "resolved_direct_callee")
}

/// A struct's package and declaration identity, as the frontend names it.
fn nominal(
    reader: &super::Reader<'_>,
    definition_path: &str,
    source: &crate::SourceText,
) -> Result<Nominal, Failure> {
    let package = reader
        .unit
        .extensions
        .as_ref()
        .and_then(|extensions| extensions.get("rust:cargo-package"));
    let text = |key: &str| {
        package
            .and_then(|package| package.get(key)?.as_str())
            .map(str::to_owned)
    };
    let data = reader.profile.is_data();
    let source_identity = match data {
        true => Some(
            text("package_id")
                .ok_or_else(|| unsupported("Missing explicit selected-unit context"))?,
        ),
        false => None,
    };
    let missing = || unsupported("Missing Cargo package identity");

    Ok(Nominal {
        source_identity,
        package_name: text("name").ok_or_else(missing)?,
        package_version: text("version").ok_or_else(missing)?,
        manifest_digest: text("manifest_digest").ok_or_else(missing)?,
        definition_path: definition_path.to_owned(),
        declaration_digest: source.checksum().to_owned(),
    })
}

impl Context<'_, '_> {
    /// The local struct a static method's implementation is for, as its nominal
    /// identity; none for any other owner, and an error past the source budget.
    pub(super) fn static_owner(&self, owner_type: &str) -> Result<Option<Nominal>, String> {
        let reader = self.reader;
        let Ok(ty) = reader.ty(owner_type) else {
            return Ok(None);
        };
        let definition = ty
            .definition
            .as_deref()
            .and_then(|definition| reader.tables.definition(definition).ok());
        let Some(definition) = definition.filter(|definition| {
            definition.scope == rstyle_core::cpg::model::Scope::Unit
                && super::identity(definition, "def_kind") == Some("Struct")
        }) else {
            return Ok(None);
        };
        let Some(source) = definition
            .span
            .as_ref()
            .and_then(|span| reader.source(span).ok())
        else {
            return Ok(None);
        };
        if source.len() > MAX_SOURCE_BYTES {
            return Err("Static owner declaration exceeds source budget".into());
        }

        match nominal(reader, &definition.name, &source) {
            Ok(owner) => Ok(Some(owner)),
            Err(Failure::Unsupported(_)) => Ok(None),
            Err(Failure::Budget(reason)) => Err(reason),
        }
    }
}
