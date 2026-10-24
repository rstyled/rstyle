//! Independent compiler-selected uses; these observations grant no semantic authority.
use crate::{ByteSpan, HirRef, data_library::Callee};
use serde::{Deserialize, Serialize};

pub const KEY: &str = "product_constant_selections";
pub const IDENTITY: &str = "rstyle.rust.product-constant-selections/1";
pub const MAX_USES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    pub definition: Callee,
    pub source_file: String,
    pub source_span: ByteSpan,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedUse {
    pub hir: HirRef,
    pub span: ByteSpan,
    pub constant: Definition,
    pub implementation: Definition,
    pub tuple_constructor: Definition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selections {
    pub identity: String,
    pub uses: Vec<SelectedUse>,
}

impl Selections {
    pub fn validate(&self) -> Result<(), String> {
        if self.identity != IDENTITY || self.uses.len() > MAX_USES {
            return Err("Unregistered or over-budget product constant selections".into());
        }

        let mut hirs = std::collections::BTreeSet::new();
        let mut spans = std::collections::BTreeSet::new();
        for selected in &self.uses {
            if !hirs.insert((selected.hir.krate, selected.hir.owner, selected.hir.local))
                || !spans.insert((selected.span.start, selected.span.end))
            {
                return Err("Ambiguous product constant compiler selection".into());
            }
        }

        Ok(())
    }
}
