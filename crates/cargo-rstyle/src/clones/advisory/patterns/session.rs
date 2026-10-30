//! Keep compact status observations only when the root review command requests them.
use rstyle_core::{
    facts::FnFacts,
    ledger::status::{self, Current},
};
use std::collections::BTreeMap;
type Census = BTreeMap<String, Current>;

pub(super) fn join(first: Option<Census>, second: Option<Census>) -> Option<Census> {
    first.zip(second).map(|(mut first, second)| {
        first.extend(second);
        first
    })
}

pub(super) fn current(original: Option<Census>, prepared: &[FnFacts]) -> Option<Census> {
    let mut original = original?;
    original.extend(status::current(prepared, true));
    Some(original)
}

impl super::Report {
    pub(in crate::clones::advisory) fn into_session(
        self,
    ) -> (Vec<rstyle_core::candidates::Candidate>, Option<Census>) {
        (self.evaluation.candidates, self.current)
    }
}
