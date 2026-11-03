//! The frontend's findings on a function's own source before expansion, from the
//! scan of its document's root file.

use super::{Context, Reader, scan};
use crate::frontend::{ByteSpan, HirDiagnostic};
use rstyle_core::cpg::model::{File, Function, Origin};
use std::rc::Rc;

impl<'a> Reader<'a> {
    /// The functions the frontend's pre-expansion scan finds in a file.
    fn scanned(&self, file: &'a File) -> Rc<Vec<scan::Function>> {
        let mut scans = self.scans.borrow_mut();
        let scanned = scans.entry(file.id.as_str()).or_insert_with(|| {
            let source = file.content.as_deref().unwrap_or_default();
            Rc::new(scan::scan(source, self.profile))
        });
        Rc::clone(scanned)
    }
}

impl<'a> Context<'_, 'a> {
    /// The findings on the function's source, and whether all of it was scanned.
    pub(super) fn source_scan(
        &self,
        function: &'a Function,
    ) -> Result<(Vec<HirDiagnostic>, bool), String> {
        let root = self.body.root()?;
        if root.provenance.origin == Origin::MacroExpansion {
            let span = root
                .provenance
                .call_site
                .as_ref()
                .map_or(ByteSpan { start: 0, end: 0 }, |site| self.span(site));
            let reason = "Macro-generated functions are deferred to rust.macros.";
            let generated =
                self.diagnostic(("rust.macros", "macro_generated_function"), span, reason);
            return Ok((vec![generated], false));
        }
        if self.reader.unit.root_file.as_deref() != Some(function.span.file.as_str()) {
            return Ok((Vec::new(), false));
        }

        let scanned = self.reader.scanned(self.reader.file(&function.span.file)?);
        let signature = (function.span.start, function.span.end);
        let body = root.span.as_ref().map(|span| (span.start, span.end));
        let conditional = self
            .reader
            .profile
            .supports("rust.conditional-source-correspondence");
        let found = scanned
            .iter()
            .find(|scan| scan.span == signature || (conditional && Some(scan.body) == body));
        let Some(found) = found else {
            return Ok((Vec::new(), false));
        };
        let diagnostics = found
            .findings
            .iter()
            .map(|finding| {
                let span = ByteSpan {
                    start: finding.span.0.saturating_sub(self.origin) as u32,
                    end: finding.span.1.saturating_sub(self.origin) as u32,
                };
                let reason = format!(
                    "Source construct is deferred to {}; remove it or assess it under a later policy.",
                    finding.capability
                );
                self.diagnostic((finding.capability, finding.construct), span, &reason)
            })
            .collect();
        Ok((diagnostics, !found.conditional))
    }

    fn diagnostic(
        &self,
        (capability, construct): (&str, &str),
        span: ByteSpan,
        reason: &str,
    ) -> HirDiagnostic {
        HirDiagnostic {
            capability: capability.into(),
            construct: construct.into(),
            span,
            snippet: self.snippet(span),
            severity: "error".into(),
            reason: reason.into(),
        }
    }
}
