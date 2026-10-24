//! Source-only binding: checked roles and raw substitutions confer no effects.
use super::request::{self, TransportRequest};
mod callbacks;
use crate::{
    HirFnFacts, concrete_types::InterfaceObservations, declaration_origins::Origins,
    generated_sources::checked::CheckedMember, nominal_boundary::resolution::Resolution,
};

pub(crate) struct CheckedTransportSource {
    request: TransportRequest,
    member: CheckedMember,
    interface: InterfaceObservations,
    origins: Origins,
}

pub(crate) fn capture(
    facts: &HirFnFacts,
    resolution: &Resolution,
    origins: &Origins,
    member: CheckedMember,
) -> Result<Option<CheckedTransportSource>, String> {
    let Some(request) = request::verify(facts, &member)? else {
        return Ok(None);
    };
    callbacks::verify(&member, &request)?;
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    resolution.verify_transport_origins(&interface, origins)?;

    Ok(Some(CheckedTransportSource {
        request,
        member,
        interface: interface.into_owned(),
        origins: origins.clone(),
    }))
}

impl CheckedTransportSource {
    pub(crate) fn request(&self) -> &TransportRequest {
        &self.request
    }
    pub(crate) fn member(&self) -> &CheckedMember {
        &self.member
    }
    pub(crate) fn matches_request(&self, request: &TransportRequest) -> bool {
        &self.request == request
    }
    pub(crate) fn verify(&self, facts: &HirFnFacts) -> Result<(), String> {
        let interface = facts
            .build
            .frontend_options
            .data::<InterfaceObservations>()?;
        let origins = facts.build.frontend_options.data::<Origins>()?;
        if interface.as_ref() != &self.interface || origins.as_ref() != &self.origins {
            return Err(
                "Transport source receipt belongs to another interface or origin roster".into(),
            );
        }
        let request = request::verify(facts, &self.member)?
            .ok_or("Transport source member no longer has its exact constructor chain")?;
        callbacks::verify(&self.member, &request)?;
        if !self.matches_request(&request) {
            return Err("Transport source roles or raw call substitutions changed".into());
        }

        Ok(())
    }
}
