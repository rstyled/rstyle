//! Shared observed Result boundary; payload ownership is proved by each caller.
use super::*;
use crate::concrete_types::{InterfaceObservations, Observation};
use crate::data_types::ResultGrammar;

pub(super) struct ObservedResult<'a> {
    pub result: TypeToken,
    pub ok: TypeToken,
    pub error: TypeToken,
    interface: std::borrow::Cow<'a, InterfaceObservations>,
}

impl ObservedResult<'_> {
    pub fn success(&self) -> &TypeTree {
        let Observation::Captured {
            ty: TypeTree::Result { ok, .. },
        } = &self.interface.result
        else {
            unreachable!("ObservedResult retains its validated captured interface");
        };
        ok
    }
}

pub(super) fn observe<'a>(
    facts: &'a dyn EvidenceFacts,
    capability: &str,
    grammar: &ResultGrammar,
) -> Result<ObservedResult<'a>, String> {
    if !crate::extraction_profile::facts_support(facts, capability)
        || !crate::extraction_profile::facts_support(facts, CAPABILITY)
    {
        return Err("Fresh Result lacks its registered boundary capabilities".into());
    }

    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    let Observation::Captured {
        ty: TypeTree::Result { ok, err },
    } = &interface.result
    else {
        return Err("Fresh Result requires its complete captured return observation".into());
    };

    let ok = TypeToken::from_tree(ok);
    let error = TypeToken::from_tree(err);
    if !grammar.permits(&ok, &error) {
        return Err("Fresh Result has an unsupported complete payload shape".into());
    }

    constructors::verify_core(facts)?;
    let result = TypeToken::Result {
        ok: Box::new(ok.clone()),
        err: Box::new(error.clone()),
    };
    if facts.signature.return_ty.normalized.as_deref() != Some(result.text()?.as_str()) {
        return Err("Fresh Result signature disagrees with its captured interface".into());
    }
    Ok(ObservedResult {
        result,
        ok,
        error,
        interface,
    })
}

/// Proofs are issued independently for each payload family and retained privately.
pub(crate) struct CheckedResult<Proof> {
    pub result: TypeToken,
    pub ok: TypeToken,
    pub error: TypeToken,
    _proof: Proof,
}

impl<Proof> CheckedResult<Proof> {
    pub fn verify_payload(&self, ok: &TypeToken, error: &TypeToken) -> Result<(), String> {
        if *ok != self.ok || *error != self.error {
            return Err("Result constructor differs from its exact whole return boundary".into());
        }
        Ok(())
    }
}

pub(super) struct Contract<Proof> {
    pub capability: &'static str,
    pub grammar: &'static ResultGrammar,
    pub check_error: fn(&TypeToken, &dyn EvidenceFacts) -> Result<(), String>,
    pub check_success: fn(&ObservedResult<'_>, &dyn EvidenceFacts) -> Result<Option<Proof>, String>,
}

pub(super) fn verify<Proof>(
    facts: &dyn EvidenceFacts,
    contract: Contract<Proof>,
) -> Result<CheckedResult<Proof>, String> {
    let observed = observe(facts, contract.capability, contract.grammar)?;
    (contract.check_error)(&observed.error, facts)?;
    let proof = (contract.check_success)(&observed, facts)?
        .ok_or("Fresh Result success lacks its exact checked payload authority")?;
    Ok(CheckedResult {
        result: observed.result,
        ok: observed.ok,
        error: observed.error,
        _proof: proof,
    })
}
