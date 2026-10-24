//! Two bounded owned Range uses; no general move, storage or call authority.
mod fields;
mod formal;
mod source;
#[cfg(test)]
mod tests;
mod transfer;

pub(crate) use fields::verify_fields;
pub(crate) use transfer::verify_transfer;
