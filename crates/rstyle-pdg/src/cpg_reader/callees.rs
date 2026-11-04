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
    pub(super) fn direct_callee(&self, callee: &Node) -> Option<PathRes> {
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
            return self.library_callee(definition);
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
        self.library_callee(definition)
    }

    /// A definition as a library callee record.
    pub(super) fn library_callee(&self, definition: &Definition) -> Option<PathRes> {
        let callee = crate::data_library::Callee {
            path: definition.name.clone(),
            crate_name: identity(definition, "crate_name")?.to_owned(),
            crate_hash: identity(definition, "crate_hash")
                .unwrap_or("local-unencoded")
                .to_owned(),
            definition_hash: identity(definition, "def_path_hash")?.to_owned(),
            implementation: None,
        };
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
