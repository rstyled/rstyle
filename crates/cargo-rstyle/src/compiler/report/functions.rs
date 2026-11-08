//! Navigation records refer to the complete documents they came from.
use super::super::Function;
use serde::{Serialize, Serializer, ser::SerializeSeq};

pub(super) struct Functions<'a> {
    pub items: &'a [Function],
}

impl<'a> Functions<'a> {
    pub fn new(items: &'a [Function]) -> Self {
        Self { items }
    }
}

impl Serialize for Functions<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.items.len()))?;
        for function in self.items {
            sequence.serialize_element(&Navigation::new(function))?;
        }
        sequence.end()
    }
}

#[derive(Serialize)]
struct Navigation<'a> {
    key: &'a super::super::FunctionKey,
    compilation_unit: &'a str,
    unit_evidence: &'a super::super::UnitEvidence,
    name: &'a str,
    fn_span: crate::compiler::build::ByteSpan,
    source_digest: &'a str,
}

impl<'a> Navigation<'a> {
    fn new(function: &'a Function) -> Self {
        Self {
            key: &function.key,
            compilation_unit: &function.compilation_unit,
            unit_evidence: &function.unit_evidence,
            name: function.facts.view().name,
            fn_span: function.facts.view().fn_span,
            source_digest: function.facts.view().digest,
        }
    }
}
