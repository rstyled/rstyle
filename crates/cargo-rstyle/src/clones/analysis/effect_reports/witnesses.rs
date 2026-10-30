//! Export original write ownership separately from projected summary roots.
use rstyle_pdg::{
    ByteSpan, HirRef,
    consumer_facts::{effects, mutations},
};
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Site {
    function_index: usize,
    site_index: usize,
    call_path: Vec<Call>,
    mapped_root: mutations::Root,
    #[serde(flatten)]
    original: mutations::WriteSite,
}

#[derive(Serialize)]
struct Call {
    function_index: usize,
    call_index: usize,
    span: ByteSpan,
    native: HirRef,
}

pub(super) fn sites(
    summary: &effects::Summary,
    session: &effects::Session<'_>,
) -> Result<Vec<Site>, String> {
    summary
        .sites
        .iter()
        .map(|witness| site(witness, session))
        .collect()
}

fn site(witness: &effects::SiteWitness, session: &effects::Session<'_>) -> Result<Site, String> {
    let observed = session
        .observations(witness.function_index)
        .ok_or("Effect summary lost its original write owner")?;
    let original = observed
        .direct
        .sites
        .get(witness.site_index)
        .ok_or("Effect summary write is outside its original owner inventory")?;
    let call_path = witness
        .call_path
        .iter()
        .map(|reference| call(reference, session))
        .collect::<Result<_, _>>()?;
    Ok(Site {
        function_index: witness.function_index,
        site_index: witness.site_index,
        call_path,
        mapped_root: witness.root.clone(),
        original: super::copy_site(original),
    })
}

fn call(
    reference: &effects::CallReference,
    session: &effects::Session<'_>,
) -> Result<Call, String> {
    let observed = session
        .observations(reference.function_index)
        .ok_or("Effect summary path lost its original call owner")?;
    let call = observed
        .calls
        .get(reference.call_index)
        .ok_or("Effect summary path is outside its original call inventory")?;
    Ok(Call {
        function_index: reference.function_index,
        call_index: reference.call_index,
        span: call.span,
        native: call.native.clone(),
    })
}
