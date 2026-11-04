//! Helper call resolutions: the callee a call, method call or index names, as the
//! frontend records its direct, inherent and library callees.

use super::Context;
use crate::frontend::PathRes;
use rstyle_core::cpg::model::{Definition, Node, Scope, TargetKind};
use serde::Serialize;
use serde_json::Value;

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
struct Dependency {
    identity: &'static str,
    crate_name: String,
    stable_crate_id: String,
    crate_hash: String,
    rmeta: String,
    instance: Value,
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
            let capture = self.associated_capture(call, definition);
            return self.instantiated_callee(call, definition, capture);
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
        let definition = self
            .reader
            .tables
            .definition(ty.definition.as_deref()?)
            .ok()?;
        if identity(definition, "diagnostic") != Some("Result") {
            return None;
        }
        let ok = ty.arguments.as_deref()?.first()?;
        Some(self.reader.ty(ok).ok()?.spelling.as_str())
    }

    fn library_record(&self, definition: &Definition) -> Option<crate::data_library::Callee> {
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
    pub(super) fn library_callee(&self, definition: &Definition) -> Option<PathRes> {
        let callee = self.library_record(definition)?;
        Some(PathRes {
            kind: "resolved_callee".into(),
            target: None,
            description: serde_json::to_value(callee).ok()?.to_string(),
        })
    }

    fn helper(
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
        if !nongeneric {
            return None;
        }
        Some(crate::inherent_calls::CallOwner {
            identity: crate::inherent_calls::IDENTITY.into(),
            implementation_identity: identity(implementation, "def_path_hash")?.to_owned(),
        })
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
            instance: serde_json::json!({"type_arguments": [], "const_arguments": []}),
        })
    }
}

fn identity<'d>(definition: &'d Definition, key: &str) -> Option<&'d str> {
    definition
        .extensions
        .get("rust:compiler-identity")?
        .get(key)?
        .as_str()
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
