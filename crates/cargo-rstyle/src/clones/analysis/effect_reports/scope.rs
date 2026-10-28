use crate::clones::analysis::local_scope;

pub(super) struct Policy;

impl local_scope::Policy for Policy {
    const POPULATION: &'static str =
        "captured original function owners, independent of strict graph admission";
    const UNAVAILABLE_REASON: &'static str = "Some requested scalar mutation or density observations are incomplete; inspect their boundaries and reasons";
}

pub(super) type Summary = local_scope::Summary<Policy>;
