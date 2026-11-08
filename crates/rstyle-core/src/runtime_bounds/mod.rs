//! Upper bounds on loop-body entries, independently of whole-function runtime.
mod analysis;
pub mod compose;
pub mod contracts;
mod ir;
mod symbolic;
pub use ir::*;
pub use symbolic::{Bound, Opaque, OpaqueKind, Order};

pub fn analyze(program: &Program) -> Outcome {
    analyze_with_limits(program, Limits::default())
}

pub fn analyze_with_limits(program: &Program, limits: Limits) -> Outcome {
    analysis::analyze(program, limits)
}

#[cfg(test)]
mod compose_tests;
#[cfg(test)]
mod tests;
