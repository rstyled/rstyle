//! Depth-two original scalar write summaries, independent of strict admission.
//! This consumer counts syntactic witnesses, never runtime writes or purity.
mod engine;
mod model;
#[cfg(test)]
mod tests;

pub use engine::Session;
pub use model::*;
