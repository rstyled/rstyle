//! Compact navigation records refer to complete, independently verifiable bundles.
use super::{super::Function, Detail};
use serde::{Serialize, Serializer, ser::SerializeSeq};

pub(super) struct Functions<'a> {
    pub items: &'a [Function],
    detail: Detail,
}

impl<'a> Functions<'a> {
    pub fn new(items: &'a [Function], detail: Detail) -> Self {
        Self { items, detail }
    }
}

impl Serialize for Functions<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.items.len()))?;
        for function in self.items {
            match self.detail {
                Detail::Full => sequence.serialize_element(function)?,
                Detail::Compact => sequence.serialize_element(&Compact::new(function))?,
            }
        }
        sequence.end()
    }
}

#[derive(Serialize)]
struct Compact<'a> {
    key: &'a super::super::FunctionKey,
    compilation_unit: &'a str,
    unit_evidence: &'a super::super::UnitEvidence,
    outcome: &'a str,
    reasons: &'a [String],
    name: &'a str,
    fn_span: rstyle_pdg::ByteSpan,
    source_digest: &'a str,
    nominal_unit_binding: &'a Option<rstyle_pdg::nominal_binding::Binding>,
}

impl<'a> Compact<'a> {
    fn new(function: &'a Function) -> Self {
        Self {
            key: &function.key,
            compilation_unit: &function.compilation_unit,
            unit_evidence: &function.unit_evidence,
            outcome: &function.outcome,
            reasons: &function.reasons,
            name: function.facts.view().name,
            fn_span: function.facts.view().fn_span,
            source_digest: &function.facts.view().source.digest,
            nominal_unit_binding: &function.nominal_unit_binding,
        }
    }
}
