//! Independent R0 fixture expectations for `compat.rust.hir-r0 / 0.1.0`.
//!
//! This crate parses the vendored hand-authored extraction expectations and
//! resolves their token selectors to UTF-8 byte spans. It is deliberately
//! independent of every producer analysis routine: expected facts come only
//! from the pinned fixture file plus this selector encoding, never from the
//! HIR frontend or the CFG/PDG derivation. Producer output is compared
//! against these expectations; the expectations are never derived from the
//! producer.

pub mod fixture;
pub mod selector;

pub use fixture::{Expected, FixtureCase, FixturePack, load_vendored_pack, parse_pack};
pub use selector::{Token, count_matches, lex, resolve};
