//! Helper call resolutions: the callee a call, method call or index names, as the
//! frontend records its direct, inherent and library callees.

use super::{Context, identity};
use crate::frontend::PathRes;
use rstyle_core::cpg::model::{Definition, Node, Scope, TargetKind};
use serde::Serialize;

/// A direct or inherent helper callee, as the frontend serializes it.
#[derive(Serialize)]
struct DirectCallee {
    path: String,
    #[serde(rename = "crate")]
    crate_name: String,
    local: bool,
    definition_identity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    safe: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inherent: Option<crate::inherent_calls::CallOwner>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compiled_dependency: Option<Dependency>,
}

/// The compiled crate a library helper comes from, in the frontend's field order.
#[derive(Serialize)]
pub(super) struct Dependency {
    identity: &'static str,
    crate_name: String,
    stable_crate_id: String,
    crate_hash: String,
    rmeta: String,
    instance: Instance,
}

/// A nongeneric dependency's instance: no type or const arguments.
#[derive(Serialize, Default)]
pub(super) struct Instance {
    type_arguments: [(); 0],
    const_arguments: [(); 0],
}

impl<'a> Context<'_, 'a> {
    /// The callee a call's path names, when the profile verifies helper calls.
    pub(super) fn direct_callee(&self, call: &Node) -> Option<PathRes> {
        let callee = self.body.child(call, 0).ok()?;
        let profile = self.reader.profile;
        if !profile.supports("rust.verified-helper-calls") || callee.original_kind != "expr_path" {
            return None;
        }
        let target = self.body.reference(&callee.id)?.target.as_ref()?;
        if !matches!(
            target.kind,
            TargetKind::Definition | TargetKind::Constructor
        ) {
            return None;
        }
        let definition = self.reader.tables.definition(target.id.as_deref()?).ok()?;
        let kind = identity(definition, "def_kind")?;
        let local = definition.scope == Scope::Unit;
        if profile.supports("rust.scalar-enum-values") && local && kind.starts_with("Ctor(Variant,")
        {
            return self.library_callee(definition);
        }
        if !local && kind == "AssocFn" && profile.is_data() {
            if let Some(resolution) = self.range_from_helper(call, definition) {
                return Some(resolution);
            }
            let capture = self.associated_capture(call, definition);
            return self.instantiated_callee(call, definition, capture);
        }
        if kind == "Fn"
            && let Some(resolution) = self.size_callee((call, callee), definition)
        {
            return Some(resolution);
        }
        let inherent = self.inherent_owner(definition);
        if generics_count(definition) != Some(0) && inherent.is_none() {
            return None;
        }
        if kind != "Fn" && inherent.is_none() {
            return None;
        }
        let dependency = (kind == "Fn" && self.compiled_dependencies())
            .then(|| self.dependency(definition))
            .flatten();

        self.helper(definition, ("resolved_direct_callee", inherent), dependency)
    }

    /// The method a method call or overloaded index dispatches to.
    pub(super) fn method_callee(&self, node: &Node) -> Option<PathRes> {
        let reference = self.body.reference(&node.id)?;
        let declared = match &reference.target {
            Some(target) => target.id.as_deref()?,
            None => reference.via.as_deref()?,
        };
        let definition = self.reader.tables.definition(declared).ok()?;
        if let Some(owner) = self.inherent_owner(definition) {
            return self.helper(definition, ("resolved_inherent_callee", Some(owner)), None);
        }
        let profile = self.reader.profile;
        let path = definition.name.as_str();
        let capture = profile.supports("rust.byte-iterator-position")
            && matches!(
                path,
                "core::iter::Iterator::position" | "std::iter::Iterator::position"
            )
            || profile.supports("rust.slice-array-byte-calls")
                && path.ends_with("::convert::TryInto::try_into");

        self.instantiated_callee(node, definition, capture)
    }

    /// A library callee, with the implementation static selection finds for a call the
    /// profile captures.
    fn instantiated_callee(
        &self,
        call: &Node,
        definition: &Definition,
        capture: bool,
    ) -> Option<PathRes> {
        if !capture {
            return self.library_callee(definition);
        }
        let mut record = self.library_record(definition)?;
        record.implementation = self
            .selected(call)
            .and_then(|selected| self.library_record(selected))
            .map(Box::new);
        Some(PathRes {
            kind: "resolved_callee".into(),
            target: None,
            description: serde_json::to_string(&record).ok()?,
        })
    }

    /// The implementation static selection finds for a call: a trait member's selected
    /// implementation, or any other callee itself.
    fn selected(&self, call: &Node) -> Option<&'a Definition> {
        let reference = self.body.reference(&call.id)?;
        if let Some(selected) = &reference.selected {
            return self.reader.tables.definition(selected.id.as_deref()?).ok();
        }
        let target = reference.target.as_ref()?;
        let callee = self.reader.tables.definition(target.id.as_deref()?).ok()?;
        let member = callee
            .owner
            .as_deref()
            .and_then(|owner| self.reader.tables.definition(owner).ok())
            .is_some_and(|owner| owner.kind == rstyle_core::cpg::model::DefinitionKind::Trait);

        (!member).then_some(callee)
    }

    /// A local nongeneric `From` conversion producing a `Range<usize>`, as the local
    /// helper it selects, with the trait call's selection.
    fn range_from_helper(&self, call: &Node, definition: &Definition) -> Option<PathRes> {
        if !self.reader.profile.supports("rust.fresh-range-results")
            || !self.local_from(call, definition)
        {
            return None;
        }
        let output = self.reader.ty(call.ty.as_deref()?).ok()?;
        let range = self
            .reader
            .tables
            .definition(output.definition.as_deref()?)
            .ok()?;
        let usize = match output.arguments.as_deref()? {
            [argument] => self.reader.ty(argument).ok()?.spelling == "usize",
            _ => false,
        };
        if !matches!(range.name.as_str(), "core::ops::Range" | "std::ops::Range")
            || range.scope == Scope::Unit
            || !usize
        {
            return None;
        }
        let selected = self.selected(call)?;
        let mut resolution = self.helper(selected, ("resolved_direct_callee", None), None)?;
        let mut record: serde_json::Value = serde_json::from_str(&resolution.description).ok()?;
        let selection = self.instantiated_callee(call, definition, true)?;
        record["trait_selection"] = serde_json::from_str(&selection.description).ok()?;
        resolution.description = record.to_string();

        Some(resolution)
    }

    /// Whether the profile captures a library associated function's conversion.
    fn associated_capture(&self, call: &Node, definition: &Definition) -> bool {
        let profile = self.reader.profile;
        let path = definition.name.as_str();
        let spelling = |id: Option<&str>| {
            id.and_then(|id| self.reader.ty(id).ok())
                .map(|ty| ty.spelling.as_str())
        };
        let arguments: Vec<Option<&str>> = self
            .body
            .children(call)
            .unwrap_or_default()
            .iter()
            .skip(1)
            .map(|argument| spelling(argument.ty.as_deref()))
            .collect();
        let result = call.ty.as_deref().and_then(|id| self.reader.ty(id).ok());
        let output = result.map(|ty| ty.spelling.as_str());
        let input = |scalar: &str| arguments == [Some(scalar)];
        let scalar_from = profile.supports("rust.scalar-from-conversions")
            && path.ends_with("::convert::From::from")
            && (input("u16") && output == Some("u32")
                || profile.supports("rust.usize-from-u8")
                    && input("u8")
                    && output == Some("usize")
                || profile.supports("rust.tagged-unit-classifiers")
                    && input("u16")
                    && output == Some("usize")
                || profile.supports("rust.unsigned-index-conversion-family")
                    && input("u16")
                    && output == Some("u64")
                || profile.supports("rust.u64-from-u32") && input("u32") && output == Some("u64"));
        let ok = result.and_then(|ty| self.result_ok(ty));
        let int_conversion = profile.supports("rust.bitset-cardinality")
            && path.ends_with("::convert::TryFrom::try_from")
            && (input("u32") && ok == Some("usize")
                || profile.supports("rust.end-of-input") && input("usize") && ok == Some("u16"));

        scalar_from || int_conversion || self.local_from(call, definition)
    }

    /// A `From::from` call whose selected implementation is local and nongeneric.
    fn local_from(&self, call: &Node, definition: &Definition) -> bool {
        let trait_from = definition
            .owner
            .as_deref()
            .and_then(|owner| self.reader.tables.definition(owner).ok())
            .is_some_and(|owner| identity(owner, "diagnostic") == Some("From"));
        let named = definition.name.rsplit("::").next() == Some("from");
        let selected = self.selected(call).is_some_and(|selected| {
            selected.scope == Scope::Unit && generics_count(selected) == Some(0)
        });
        trait_from && named && selected
    }

    /// A standard `Result`'s success type spelling.
    fn result_ok(&self, ty: &rstyle_core::cpg::model::Type) -> Option<&'a str> {
        let ok = self.reader.diagnostic_arguments(ty, "Result")?.first()?;
        Some(self.reader.ty(ok).ok()?.spelling.as_str())
    }

    pub(super) fn library_record(
        &self,
        definition: &Definition,
    ) -> Option<crate::data_library::Callee> {
        Some(crate::data_library::Callee {
            path: definition.name.clone(),
            crate_name: identity(definition, "crate_name")?.to_owned(),
            crate_hash: identity(definition, "crate_hash")
                .unwrap_or("local-unencoded")
                .to_owned(),
            definition_hash: identity(definition, "def_path_hash")?.to_owned(),
            implementation: None,
        })
    }

    /// A definition as a library callee record.
    /// `size_of` of an unsigned scalar, with the argument the call instantiates.
    fn size_callee(
        &self,
        (call, path): (&Node, &Node),
        definition: &Definition,
    ) -> Option<PathRes> {
        if !self.reader.profile.supports(crate::size_calls::CAPABILITY)
            || definition.scope == Scope::Unit
            || identity(definition, "crate_name") != Some("core")
            || !matches!(
                definition.name.as_str(),
                "core::mem::size_of" | "std::mem::size_of"
            )
        {
            return None;
        }
        let arguments = [call, path]
            .into_iter()
            .find_map(|node| self.body.reference(&node.id)?.instance_arguments.as_deref())?;
        let [argument] = arguments else {
            return None;
        };
        let argument = self.reader.ty(argument).ok()?;
        let callable = definition.callable.as_ref()?;
        let result = self.reader.ty(&callable.result).ok()?;
        if argument.kind != rstyle_core::cpg::model::TypeKind::Integer
            || !matches!(argument.spelling.as_str(), "u16" | "u32" | "u128")
            || !callable.parameters.is_empty()
            || result.spelling != "usize"
        {
            return None;
        }
        let capture = crate::size_calls::Capture {
            callee: self.library_record(definition)?,
            argument: argument.spelling.clone(),
        };

        Some(PathRes {
            kind: crate::size_calls::RESOLUTION.into(),
            target: None,
            description: serde_json::to_string(&capture).ok()?,
        })
    }

    pub(super) fn library_callee(&self, definition: &Definition) -> Option<PathRes> {
        let callee = self.library_record(definition)?;
        Some(PathRes {
            kind: "resolved_callee".into(),
            target: None,
            description: serde_json::to_value(callee).ok()?.to_string(),
        })
    }

    pub(super) fn helper(
        &self,
        definition: &Definition,
        (kind, inherent): (&str, Option<crate::inherent_calls::CallOwner>),
        compiled_dependency: Option<Dependency>,
    ) -> Option<PathRes> {
        let callable = matches!(identity(definition, "def_kind")?, "Fn" | "AssocFn")
            || identity(definition, "def_kind")?.starts_with("Ctor(");
        let safe = callable.then(|| {
            let modifiers = definition
                .callable
                .as_ref()
                .map(|callable| &callable.modifiers);
            !modifiers.is_some_and(|modifiers| modifiers.iter().any(|m| m == "unsafe"))
        });
        let callee = DirectCallee {
            path: definition.name.clone(),
            crate_name: identity(definition, "crate_name")?.to_owned(),
            local: definition.scope == Scope::Unit,
            definition_identity: identity(definition, "def_path_hash")?.to_owned(),
            safe,
            inherent,
            compiled_dependency,
        };
        Some(PathRes {
            kind: kind.into(),
            target: None,
            description: serde_json::to_string(&callee).ok()?,
        })
    }

    /// The inherent implementation a local method belongs to, as a call owner.
    fn inherent_owner(&self, definition: &Definition) -> Option<crate::inherent_calls::CallOwner> {
        let profile = self.reader.profile;
        if !profile.supports("rust.verified-inherent-helper-calls")
            || definition.scope != Scope::Unit
            || identity(definition, "def_kind")? != "AssocFn"
        {
            return None;
        }
        let compiler = definition.extensions.get("rust:compiler-identity")?;
        let implementation = self
            .reader
            .tables
            .definition(compiler.get("inherent_impl")?.as_str()?)
            .ok()?;
        let nongeneric = generics_count(definition) == Some(0)
            && generics_count(implementation) == Some(0)
            && !self.source_lifetime_arguments(implementation);
        if !nongeneric
            && !(profile.supports("rust.borrowed-mixed-owners")
                && self.lifetime_owner(definition, implementation))
        {
            return None;
        }

        Some(crate::inherent_calls::CallOwner {
            identity: crate::inherent_calls::IDENTITY.into(),
            implementation_identity: identity(implementation, "def_path_hash")?.to_owned(),
        })
    }

    /// A method of an implementation generic only over lifetimes, written as its own
    /// self type's lifetime arguments, that reads its receiver through a shared borrow;
    /// or an associated function, where the profile follows product results.
    fn lifetime_owner(&self, method: &Definition, implementation: &Definition) -> bool {
        let generics = |definition: &Definition| {
            definition
                .extensions
                .get("rust:compiler-identity")
                .and_then(|identity| identity.get("generics"))
                .cloned()
        };
        let (Some(method_generics), Some(owner)) = (generics(method), generics(implementation))
        else {
            return false;
        };
        let own = |generics: &serde_json::Value| {
            generics
                .get("own")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default()
        };

        let parameters = own(&owner);
        let lifetimes = !parameters.is_empty()
            && parameters.len() <= crate::concrete_types::MAX_FIELDS
            && parameters.iter().all(|kind| kind == "lifetime");
        if !own(&method_generics).is_empty()
            || owner.get("parented") != Some(&serde_json::Value::Bool(false))
            || !lifetimes
            || !self.source_lifetime_implementation(implementation)
        {
            return false;
        }

        match method
            .callable
            .as_ref()
            .and_then(|callable| callable.receiver.as_deref())
        {
            Some(receiver) => self.reader.ty(receiver).is_ok_and(|receiver| {
                receiver.kind == rstyle_core::cpg::model::TypeKind::Reference
                    && receiver.mode.as_deref() == Some("shared")
            }),
            None => self
                .reader
                .profile
                .supports("rust.linear-product-result-flow"),
        }
    }

    /// Whether an implementation declares only plain lifetimes, without bounds or a
    /// where clause, and names exactly them, in order, as its self type's arguments.
    fn source_lifetime_implementation(&self, implementation: &Definition) -> bool {
        let Some(item) = self.implementation_item(implementation) else {
            return false;
        };
        let generics = &item.generics;
        let parameters: Option<Vec<&syn::Lifetime>> = generics
            .params
            .iter()
            .map(|parameter| match parameter {
                syn::GenericParam::Lifetime(parameter)
                    if parameter.attrs.is_empty()
                        && parameter.bounds.is_empty()
                        && parameter.colon_token.is_none()
                        && parameter.lifetime.ident != "_" =>
                {
                    Some(&parameter.lifetime)
                }
                _ => None,
            })
            .collect();
        let Some(parameters) = parameters else {
            return false;
        };
        let syn::Type::Path(path) = &*item.self_ty else {
            return false;
        };
        let segments: Vec<&syn::PathSegment> = path.path.segments.iter().collect();
        let Some((last, parents)) = segments.split_last() else {
            return false;
        };
        let syn::PathArguments::AngleBracketed(arguments) = &last.arguments else {
            return false;
        };

        generics.where_clause.is_none()
            && path.qself.is_none()
            && parents
                .iter()
                .all(|segment| segment.arguments.is_none())
            && arguments.args.len() == parameters.len()
            && arguments.args.iter().zip(&parameters).all(|(argument, parameter)| {
                matches!(argument, syn::GenericArgument::Lifetime(lifetime) if lifetime == *parameter)
            })
    }

    fn implementation_item(&self, implementation: &Definition) -> Option<syn::ItemImpl> {
        let source = self.reader.source(implementation.span.as_ref()?).ok()?;
        syn::parse_str::<syn::ItemImpl>(source.as_str()).ok()
    }

    /// Whether an implementation's self type is written with lifetime arguments.
    fn source_lifetime_arguments(&self, implementation: &Definition) -> bool {
        let Some(source) = implementation
            .span
            .as_ref()
            .and_then(|span| self.reader.source(span).ok())
        else {
            return false;
        };
        let Ok(item) = syn::parse_str::<syn::ItemImpl>(source.as_str()) else {
            return false;
        };
        let syn::Type::Path(path) = &*item.self_ty else {
            return false;
        };
        path.qself.is_none()
            && path
                .path
                .segments
                .iter()
                .any(|segment| match &segment.arguments {
                    syn::PathArguments::AngleBracketed(arguments) => arguments
                        .args
                        .iter()
                        .any(|argument| matches!(argument, syn::GenericArgument::Lifetime(_))),
                    _ => false,
                })
    }

    fn compiled_dependencies(&self) -> bool {
        let profile = self.reader.profile;
        profile == crate::extraction_profile::ExtractionProfile::DataV3
            && profile.supports("rust.verified-helper-calls")
    }

    /// The compiled crate a nongeneric library function comes from.
    fn dependency(&self, definition: &Definition) -> Option<Dependency> {
        if definition.scope == Scope::Unit || generics_count(definition) != Some(0) {
            return None;
        }
        Some(Dependency {
            identity: "rstyle.rust.compiled-crate/1",
            crate_name: identity(definition, "crate_name")?.to_owned(),
            stable_crate_id: identity(definition, "stable_crate_id")?.to_owned(),
            crate_hash: identity(definition, "crate_hash")?.to_owned(),
            rmeta: identity(definition, "rmeta")?.to_owned(),
            instance: Instance::default(),
        })
    }
}

/// All of a definition's generic parameters, its parents' included.
fn generics_count(definition: &Definition) -> Option<u64> {
    definition
        .extensions
        .get("rust:compiler-identity")?
        .get("generics")?
        .get("count")?
        .as_u64()
}
