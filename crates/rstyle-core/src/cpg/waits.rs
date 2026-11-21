//! Library calls that wait a fixed time, for the `test_sleep` rule. Which calls those are is
//! data, one file per frontend namespace under `contracts/waits`, so a language needs a file
//! here and no code. A namespace without a file names none.

use super::model::Definition;
use crate::versioned::{ByNamespace, Namespaced};
use serde::Deserialize;

const SCHEMA: &str = "rstyle.fixed-waits/1";
const EMBEDDED: &[&str] = &[
    include_str!("contracts/waits/go.toml"),
    include_str!("contracts/waits/py.toml"),
];

/// The operations of one namespace that wait a fixed time, keyed without the namespace.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedWaits {
    schema: String,
    namespace: String,
    operations: Vec<String>,
}

impl FixedWaits {
    pub fn of(namespace: &str) -> &'static FixedWaits {
        static WAITS: ByNamespace<FixedWaits> = ByNamespace::empty();
        WAITS.named(namespace, (EMBEDDED, (SCHEMA, "fixed waits")))
    }

    /// Whether a call that selects `callee` waits a fixed time: an operation this namespace's
    /// file names exactly. Only a library's functions have an operation key.
    pub fn holds(&self, callee: &Definition) -> bool {
        callee
            .operation
            .as_deref()
            .and_then(|key| key.strip_prefix(self.namespace.as_str()))
            .and_then(|key| key.strip_prefix(':'))
            .is_some_and(|operation| self.operations.iter().any(|listed| listed == operation))
    }
}

impl Namespaced for FixedWaits {
    fn schema(&self) -> &str {
        &self.schema
    }
    fn namespace(&self) -> &str {
        &self.namespace
    }
}

#[cfg(test)]
mod tests;
