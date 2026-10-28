//! Bounded source-owned enum dispatch; no body/effect or rewrite authority.
mod index;
mod model;
mod shapes;
pub use index::Index;
pub use model::{Arm, Assessment, Candidate, MethodOrigin, Signature};

#[cfg(test)]
mod tests;
