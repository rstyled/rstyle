//! Structural substitution is checked against independent source-selected roles.
use super::{Result, body::Role, receipts, transport::*};
use crate::{
    HirFnFacts, HirNode, ResolvedTy,
    concrete_types::{InterfaceObservations, Observation, TypeTree},
    unit_store::EvidenceFacts,
};

pub(super) struct Types {
    pub result: TypeTree,
    pub error: TypeTree,
    result_identity: Type,
    error_identity: Type,
}

impl Types {
    pub fn new(
        owning: &dyn EvidenceFacts,
        projection: &Projection,
        failure: &HirNode,
    ) -> Result<Self> {
        let original = owning.facts();
        let interface = original
            .build
            .frontend_options
            .data::<InterfaceObservations>()?;
        interface.validate()?;

        let Observation::Captured {
            ty: TypeTree::Result { ok, err },
        } = &interface.result
        else {
            return Err("Slice helper requires its complete original Result interface".into());
        };
        if **ok != TypeTree::Scalar("unit".into()) || interface.parameters.len() != 3 {
            return Err("Slice helper original success/interface roster differs".into());
        }

        let TypeTree::Struct {
            nominal, fields, ..
        } = err.as_ref()
        else {
            return Err("Slice helper error is not nominal".into());
        };
        if !matches!(fields.as_slice(), [field] if field.name == "0" && matches!(field.ty, TypeTree::MixedEnum { .. }))
            || !owning.nominal_resolution()?.checked_no_drop(nominal)
        {
            return Err("Slice helper error lacks its independent complete no-drop proof".into());
        }

        let (result_identity, error_identity) =
            projected_result(original, projection, &nominal.definition_path, failure)?;

        Ok(Self {
            result: TypeTree::Result {
                ok: ok.clone(),
                err: err.clone(),
            },
            error: err.as_ref().clone(),
            error_identity,
            result_identity,
        })
    }

    pub fn expected(&self, role: Role) -> Type {
        match role {
            Role::Bytes => shared(Type::Slice {
                element: Box::new(builtin("u8")),
            }),
            Role::Usize => builtin("usize"),
            Role::Text => shared(Type::Str),
            Role::Bool => builtin("bool"),
            Role::Unit => unit(),
            Role::Never | Role::Returned => builtin("!"),
            Role::Result => self.result_identity.clone(),
            Role::Error => self.error_identity.clone(),
        }
    }

    pub fn endpoint(&self, endpoint: &Endpoint, role: Role) -> Result<()> {
        if *identity(endpoint)? != self.expected(role) {
            return Err("Projected type differs from complete independent source flow".into());
        }
        Ok(())
    }

    pub fn resolved(&self, original: &ResolvedTy, role: Role) -> Result<ResolvedTy> {
        let tree = self.tree(role);
        let normalized = tree.as_ref().map(crate::data_types::token).transpose()?;
        Ok(ResolvedTy {
            normalized,
            observed: match role {
                Role::Bytes => "&[u8]".into(),
                Role::Never => "!".into(),
                _ => original.observed.clone(),
            },
            spelling: original.spelling.clone(),
        })
    }

    pub fn tree(&self, role: Role) -> Option<TypeTree> {
        Some(match role {
            Role::Bytes => TypeTree::Shared(Box::new(TypeTree::Slice(Box::new(TypeTree::Scalar(
                "u8".into(),
            ))))),
            Role::Text => TypeTree::Shared(Box::new(TypeTree::Str)),
            Role::Usize => TypeTree::Scalar("usize".into()),
            Role::Bool => TypeTree::Scalar("bool".into()),
            Role::Unit => TypeTree::Scalar("unit".into()),
            Role::Result | Role::Returned => self.result.clone(),
            Role::Error => self.error.clone(),
            Role::Never => return None,
        })
    }
}

pub(super) fn identity(endpoint: &Endpoint) -> Result<&Type> {
    match endpoint {
        Endpoint::Structural { arguments } => {
            let [argument] = arguments.as_slice() else {
                return Err("Projected endpoint requires one type".into());
            };
            argument_identity(argument)
        }
        Endpoint::FunctionItem {
            definition,
            arguments,
        } => Err(format!(
            "Unexpected function-item value {} with {} arguments",
            definition.path,
            arguments.len()
        )),
    }
}

fn projected_result(
    original: &HirFnFacts,
    projection: &Projection,
    error_path: &str,
    failure: &HirNode,
) -> Result<(Type, Type)> {
    let result_identity = identity(&projection.result)?.clone();
    let Type::Adt {
        definition,
        arguments,
    } = &result_identity
    else {
        return Err("Projected Result is not structural ADT evidence".into());
    };

    result_definition(original, definition)?;
    let [success, error_identity] = arguments.as_slice() else {
        return Err("Projected Result arguments differ".into());
    };
    if *success != unit() {
        return Err("Projected Result success is not unit".into());
    }

    local_error(original, error_identity, error_path)?;

    let arguments: Vec<Argument> = serde_json::from_value(
        receipts::inventory_call(original, &failure.hir)?["requested_arguments"].clone(),
    )
    .map_err(|e| e.to_string())?;
    arguments_match(&arguments, &[unit(), error_identity.clone()])?;
    let original_call = receipts::inventory_call(original, &failure.hir)?;
    if original_call["requested"]["stable_crate_id"] != definition.stable_crate_id {
        return Err("Projected Result differs from original core crate identity".into());
    }

    Ok((result_identity.clone(), error_identity.clone()))
}

fn argument_identity(argument: &Argument) -> Result<&Type> {
    if argument.kind != "type"
        || !argument.display_only
        || argument.identity_status != "known"
        || argument.display.len() > 4096
    {
        return Err("Projected type requires complete structural identity".into());
    }
    Ok(&argument.identity)
}

pub(super) fn arguments_match(arguments: &[Argument], expected: &[Type]) -> Result<()> {
    if arguments.len() != expected.len() {
        return Err("Concrete generic argument roster differs".into());
    }
    for (argument, expected) in arguments.iter().zip(expected) {
        if argument_identity(argument)? != expected {
            return Err("Concrete generic substitution differs".into());
        }
    }
    Ok(())
}

pub(super) fn builtin(name: &str) -> Type {
    Type::Builtin { name: name.into() }
}
pub(super) fn unit() -> Type {
    Type::Tuple {
        elements: Vec::new(),
    }
}
pub(super) fn shared(referent: Type) -> Type {
    Type::Reference {
        region: "erased".into(),
        mutable: false,
        referent: Box::new(referent),
    }
}

fn result_definition(
    original: &HirFnFacts,
    definition: &crate::generated_sources::capture::Definition,
) -> Result<()> {
    let registry = crate::data_library::verify_64bit_sysroot(original, "Slice Result instance")?;
    if definition.local
        || definition.crate_name != "core"
        || definition.stable_crate_id.is_empty()
        || !matches!(
            definition.path.as_str(),
            "core::result::Result" | "std::result::Result"
        )
        || registry.definitions.get("std::result::Result") != Some(&definition.definition_identity)
    {
        return Err("Projected Result changes the pinned core definition".into());
    }
    Ok(())
}

fn local_error(original: &HirFnFacts, ty: &Type, path: &str) -> Result<()> {
    let Type::Adt {
        definition,
        arguments,
    } = ty
    else {
        return Err("Projected error lacks nominal identity".into());
    };
    let compiled = original
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Missing compiled instance owner")?;
    if !definition.local
        || !arguments.is_empty()
        || definition.path != path
        || definition.definition_identity.is_empty()
        || compiled["crate_name"] != definition.crate_name
        || compiled["stable_crate_id"] != definition.stable_crate_id
    {
        return Err("Projected error differs from its original local nominal owner".into());
    }
    Ok(())
}
