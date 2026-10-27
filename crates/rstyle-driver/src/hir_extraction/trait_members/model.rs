use super::instances::{Definition, types};
use rstyle_pdg::nominal_boundary::census::expansions::FileSpan;
use serde::Serialize;

#[derive(Serialize)]
pub(super) struct Observation {
    pub identity: &'static str,
    pub admission_authority: bool,
    pub method: Definition,
    pub implementation: Definition,
    pub trait_definition: Definition,
    pub trait_method: Definition,
    pub implementation_source: FileSpan,
    pub method_source: FileSpan,
    pub method_header: Header,
    pub members: Vec<Member>,
    pub trait_arguments: types::Arguments,
    pub selected: Definition,
    pub selected_arguments: types::Arguments,
    pub requested_signature: Signature,
    pub selected_signature: Signature,
    pub associated_error: Option<AssociatedError>,
}

#[derive(Serialize)]
pub(super) struct Member {
    pub definition: Definition,
    pub trait_member: Option<Definition>,
    pub source: FileSpan,
}

#[derive(Serialize)]
pub(super) struct Signature {
    pub inputs: Vec<types::Arguments>,
    pub output: types::Arguments,
    pub bound_variables: usize,
    pub abi: String,
    pub safety: String,
    pub variadic: bool,
}

#[derive(Serialize)]
pub(super) struct Header {
    pub is_const: bool,
    pub is_async: bool,
}

#[derive(Serialize)]
pub(super) struct AssociatedError {
    pub trait_member: Definition,
    pub implementation_member: Definition,
    pub source: FileSpan,
    pub declared_type: types::Arguments,
    pub normalized_projection: types::Arguments,
}
