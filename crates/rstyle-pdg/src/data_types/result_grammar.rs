//! Structural success/error grammar; this grants no ownership or source authority.
use super::{Nominal, TypeToken};

pub(crate) struct ResultGrammar {
    pub success: fn(&TypeToken) -> bool,
    pub error: fn(&TypeToken) -> Option<&Nominal>,
}

impl ResultGrammar {
    pub fn permits(&self, ok: &TypeToken, err: &TypeToken) -> bool {
        (self.success)(ok) && (self.error)(err).is_some()
    }
}
