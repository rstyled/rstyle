use crate::clones::analysis::local_scope;

pub(super) struct Policy;

impl local_scope::Policy for Policy {
    const POPULATION: &'static str =
        "captured original function owners, independent of strict graph admission";
    const UNAVAILABLE_REASON: &'static str =
        "Some owners have incomplete scalar mutation effects; inspect their boundaries and reasons";
}

pub(super) type Summary = local_scope::Summary<Policy>;
