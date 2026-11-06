//! Bounded review rosters share original owner resolution and ambiguity checks.

pub(super) struct Observations<R> {
    pub rows: Vec<R>,
    pub truncated: bool,
    pub unavailable_scope: bool,
    pub requested: bool,
}
