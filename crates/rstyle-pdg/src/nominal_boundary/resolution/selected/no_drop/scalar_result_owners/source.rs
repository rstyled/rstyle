//! Exact original static owner binding, independent of nominal permission selection.
use super::*;
use crate::nominal_boundary::resolution::tryfrom_owners;

pub(super) enum Binding {
    Inherent {
        observation: Box<crate::inherent_calls::Definition>,
        namespace: Namespace,
    },
    TryFrom {
        member: Box<crate::tryfrom_calls::SourceMember>,
        namespace: Namespace,
    },
}

impl Binding {
    pub(super) fn namespace(&self) -> &[String] {
        match self {
            Self::Inherent { namespace, .. } | Self::TryFrom { namespace, .. } => namespace,
        }
    }

    pub(super) fn lexical(
        &self,
        facts: &HirFnFacts,
        resolution: &Resolution,
        closure: &super::CheckedNoDrop,
    ) -> Result<(), String> {
        if let Self::TryFrom { member, namespace } = self {
            tryfrom_owners::lexical(
                facts,
                &crate::nominal_boundary::resolution::Scoped {
                    resolution,
                    namespace,
                },
                member,
                closure,
            )?;
        }
        Ok(())
    }

    pub(super) fn verify(&self, facts: &HirFnFacts) -> Result<(), String> {
        match self {
            Self::Inherent { observation, .. } => {
                if super::super::copy_owners::source::observation(facts)?.as_ref()
                    != Some(observation)
                {
                    return Err("Scalar Result receipt has another original inherent owner".into());
                }
                Ok(())
            }
            Self::TryFrom { member, .. } => crate::tryfrom_calls::verify_owner(
                facts,
                member,
                &crate::tryfrom_calls::RegistryAuthority,
            ),
        }
    }
}

pub(super) fn interface<'a>(
    facts: &HirFnFacts,
    interface: &'a InterfaceObservations,
) -> Result<Option<(&'a TypeTree, &'a TypeTree)>, String> {
    interface.validate()?;
    let Observation::Captured {
        ty: TypeTree::Result { ok, err },
    } = &interface.result
    else {
        return Ok(None);
    };
    if !copy_products::supported(ok) || !super::super::scalar_errors::supported(err) {
        return Ok(None);
    }
    if facts.signature.has_self || facts.signature.params.len() != interface.parameters.len() {
        return Ok(None);
    }
    if interface.parameters.iter().any(|parameter| {
        !matches!(parameter, Observation::Captured { ty }
            if crate::checked_index_bytes::parameter(facts, ty))
    }) {
        return Ok(None);
    }

    Ok(Some((ok, err)))
}

pub(super) fn bind(
    facts: &HirFnFacts,
    resolution: &Resolution,
    success: &TypeTree,
    error: &TypeTree,
) -> Result<Option<Binding>, String> {
    if facts
        .build
        .frontend_options
        .contains_key(crate::tryfrom_calls::KEY)
    {
        let (namespace, member) = tryfrom_owners::find(facts, resolution)?;
        if &member.success != success || &member.error != error {
            return Err("Scalar Result owner disagrees with its selected trait interface".into());
        }
        return Ok(Some(Binding::TryFrom {
            member: Box::new(member),
            namespace,
        }));
    }
    let Some(observation) = super::super::copy_owners::source::observation(facts)? else {
        return Ok(None);
    };
    let Some(owner) = super::super::copy_owners::source::resolve(facts, resolution, &observation)?
    else {
        return Ok(None);
    };
    if &owner.root != success {
        return Err("Scalar Result success is not the exact original inherent owner".into());
    }

    Ok(Some(Binding::Inherent {
        observation: Box::new(observation),
        namespace: owner.namespace,
    }))
}
