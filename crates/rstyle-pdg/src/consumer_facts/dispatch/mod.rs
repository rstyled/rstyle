//! Bounded source-owned enum dispatch; no body/effect or rewrite authority.
pub mod capabilities;
mod index;
mod model;
pub mod review;
mod shapes;
pub use index::Index;
pub use model::{Arm, Assessment, Candidate, MethodOrigin, Signature};

#[cfg(test)]
mod tests;
