//! Move raw compiler proposals into the ordinary repository review session.
use rstyle_core::{candidates::Candidate, ledger::Ledger};

impl super::Report {
    pub(in crate::clones) fn into_repository_session(
        self,
        ledger: &Ledger,
        filter: Option<&str>,
        status: bool,
    ) -> Result<(Vec<Candidate>, String), String> {
        let mut candidates = Vec::new();
        let mut reports = Vec::new();
        for unit in self.units {
            let Some(patterns) = unit.patterns else {
                continue;
            };
            let (raw, current) = patterns.into_session();
            if status {
                let current = current.ok_or("Compiler review session census was not requested")?;
                let statuses = rstyle_core::ledger::status::status_observed(ledger, &raw, &current);
                reports.push(format!(
                    "Compiler population anchored at unit {}\n{}",
                    unit.unit.ordinal(),
                    crate::status_text(statuses, filter)
                ));
            }
            candidates.extend(raw);
        }
        Ok((candidates, reports.join("\n\n")))
    }
}
