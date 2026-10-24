//! Exact coordinates in an immutable original census buffer; no proof authority.
use super::{UnitFile, expansions::FileSpan};

pub(crate) fn site(file: &UnitFile, span: proc_macro2::Span) -> FileSpan {
    FileSpan {
        path: file.path.clone(),
        digest: file.digest.clone(),
        span: crate::source_evidence::original_span(&file.bytes, span),
    }
}
