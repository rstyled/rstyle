//! Reuse completed root observations within one function's fixed capture context.
use super::*;
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct Completed<'tcx> {
    observations: HashMap<Ty<'tcx>, Observation>,
    tokens: HashMap<Ty<'tcx>, Option<String>>,
}

impl<'tcx> Capture<'_, 'tcx> {
    pub(super) fn observe(&mut self, ty: Ty<'tcx>) -> Result<Observation, String> {
        // Recursive tree calls have depth/visiting-dependent permissions and must
        // never consult these root receipts. The exact Ty retains substitutions
        // and lifetimes; the enclosing Capture fixes owner, source and profile.
        debug_assert!(self.visiting.is_empty());
        if let Some(observation) = self.reuse.observations.get(&ty) {
            return Ok(observation.clone());
        }

        let observation = match self.tree(ty, 0) {
            Ok(ty) => Observation::Captured { ty },
            Err(Failure::Unsupported(reason)) => Observation::Unsupported { reason },
            Err(Failure::Budget(reason)) => return Err(reason),
        };
        // First-pass source origins remain in this Capture until final contextual
        // pruning. Only completed observations enter the table; no partial tree
        // or budget failure becomes unsupported or reusable.
        self.reuse.observations.insert(ty, observation.clone());
        Ok(observation)
    }

    pub(super) fn type_token(&mut self, ty: Ty<'tcx>) -> Result<Option<String>, String> {
        if let Some(token) = self.reuse.tokens.get(&ty) {
            return Ok(token.clone());
        }

        let token = match self.observe(ty)? {
            Observation::Captured { ty } => Some(rstyle_pdg::data_types::token(&ty)?),
            Observation::Unsupported { .. } => None,
        };
        self.reuse.tokens.insert(ty, token.clone());
        Ok(token)
    }
}
