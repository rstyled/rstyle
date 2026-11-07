//! The defects a run's records hold, as the report and SARIF show them.

use crate::config::Config;
use crate::cpg::defects::{Defect, Examined};
use crate::cpg::records::{Measured, Record};
use crate::{exclusions, suppress};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::Path;

/// One thing a rule found, and where.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct DefectRow {
    pub path: String,
    pub line: usize,
    pub rule: &'static str,
    pub message: String,
    /// The function it's in, by identity.
    pub function: String,
}

/// A function the rules couldn't read, and why.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct UnknownRow {
    pub function: String,
    pub path: String,
    pub reason: String,
}

/// What the rules found in a run. No defect in a function doesn't make it correct, and
/// `unknown` lists the functions they couldn't read at all.
#[derive(Debug, Serialize)]
pub struct DefectsReport {
    pub defects: Vec<DefectRow>,
    /// Functions read, each once however many compilations hold it.
    pub functions: usize,
    pub suppressed: usize,
    pub unknown: Vec<UnknownRow>,
}

impl DefectsReport {
    pub fn text(&self) -> String {
        let rows = self
            .defects
            .iter()
            .map(|row| format!("{}:{}: {}: {}", row.path, row.line, row.rule, row.message));
        let plural = if self.defects.len() == 1 { "" } else { "s" };
        let summary = format!(
            "Defects: {} defect{plural} in {} functions, {} suppressed, {} unknown",
            self.defects.len(),
            self.functions,
            self.suppressed,
            self.unknown.len(),
        );
        rows.chain(std::iter::once(summary))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Moves each record's defects out, for the functions a person wrote and a run selected. A unit
/// compiled twice, as a library and its test harness are, holds the same function twice, so what
/// repeats is reported once.
pub fn from_records(records: &mut [Record], config: &Config) -> Result<DefectsReport, String> {
    let excluded = exclusions::Matcher::compile(&config.exclude.0)?;
    let mut read = Gathered::default();
    for record in records {
        let Some(examined) = record.defects.take() else {
            continue;
        };
        let written = matches!(record.measured, Measured::Known(_));
        if !written || excluded.is_excluded(Path::new(&record.path)) {
            continue;
        }
        read.functions.insert(record.identity.clone());
        match examined {
            Examined::Read(found) => read.add_found(record, found),
            Examined::Unknown(reason) => read.add_unknown(record, reason),
        }
    }
    Ok(read.into_report())
}

/// What's been read so far, in the sets that make a function count once.
#[derive(Default)]
struct Gathered {
    defects: BTreeSet<DefectRow>,
    functions: BTreeSet<String>,
    suppressed: BTreeSet<(String, usize, &'static str)>,
    unknown: BTreeSet<UnknownRow>,
}

impl Gathered {
    fn add_unknown(&mut self, record: &Record, reason: String) {
        self.unknown.insert(UnknownRow {
            function: record.identity.clone(),
            path: record.path.clone(),
            reason,
        });
    }

    fn add_found(&mut self, record: &Record, found: Vec<Defect>) {
        let preceding = match &record.measured {
            Measured::Known(facts) => facts.preceding_line.as_str(),
            _ => "",
        };
        for defect in found {
            if suppress::allows(preceding, defect.rule) {
                self.suppressed
                    .insert((record.path.clone(), defect.line, defect.rule));
                continue;
            }
            self.defects.insert(DefectRow {
                path: record.path.clone(),
                line: defect.line,
                rule: defect.rule,
                message: defect.message,
                function: record.identity.clone(),
            });
        }
    }

    fn into_report(self) -> DefectsReport {
        DefectsReport {
            defects: self.defects.into_iter().collect(),
            functions: self.functions.len(),
            suppressed: self.suppressed.len(),
            unknown: self.unknown.into_iter().collect(),
        }
    }
}

#[cfg(test)]
mod tests;
