//! What a run takes from each document's records as soon as its worker derives them, so the
//! records held for the whole-run analyses keep only what those read. A function's flow goes to
//! a temporary file and comes back one row at a time when the report is written; its defects
//! fold into the run's tally; and what only a merge-risk comparison reads is dropped.

use super::FlowRow;
use crate::config::Config;
use crate::cpg::flow::Standing;
use crate::cpg::records::{CanonicalKey, Record};
use crate::defects_report::DefectsFold;
use crate::filter::{Analysis, Filter};
use serde::ser::{Error as _, SerializeSeq};
use std::fs::File;
use std::io::{BufWriter, Read, Seek, SeekFrom, Write};
use std::sync::Mutex;

/// A run's analyses, taking from each document's records as they are derived. The pool's
/// workers share it by reference; `unified::analyze_taking` then consumes it.
pub struct Early {
    flow: Option<Mutex<Spill>>,
    defects: Option<Mutex<DefectsFold>>,
}

impl Early {
    /// Spills flow when the filter selects flow, and folds defects when it selects defects.
    pub fn new(filter: &Filter, config: &Config) -> Result<Self, String> {
        let flow = match filter.core(Analysis::Flow) {
            true => Some(Mutex::new(Spill::new()?)),
            false => None,
        };
        let defects = match filter.core(Analysis::Defects) {
            true => Some(Mutex::new(DefectsFold::new(config)?)),
            false => None,
        };
        Ok(Self { flow, defects })
    }

    /// Takes a document's flow and defects out of its records, and drops what only a
    /// merge-risk comparison reads.
    pub fn take(&self, mut records: Vec<Record>) -> Result<Vec<Record>, String> {
        if let Some(spill) = &self.flow {
            let rows = records
                .iter_mut()
                .filter_map(spilled_row)
                .collect::<Result<Vec<_>, String>>()?;
            spill.lock().map_err(|_| POISONED)?.write(rows)?;
        }
        if let Some(defects) = &self.defects {
            defects.lock().map_err(|_| POISONED)?.add(&mut records);
        }
        Ok(records
            .into_iter()
            .map(Record::without_review_fields)
            .collect())
    }

    /// What was taken: the flow rows in canonical order, and the defects tally.
    pub(super) fn finish(self) -> Result<(Option<SpilledFlow>, Option<DefectsFold>), String> {
        let flow = match self.flow {
            Some(spill) => Some(spill.into_inner().map_err(|_| POISONED)?.finish()?),
            None => None,
        };
        let defects = match self.defects {
            Some(fold) => Some(fold.into_inner().map_err(|_| POISONED)?),
            None => None,
        };
        Ok((flow, defects))
    }
}

const POISONED: &str = "A worker panicked while handing over its records";

/// One function's flow row, serialized and ready to spill.
struct Taken {
    key: CanonicalKey,
    bytes: Vec<u8>,
    standing: Standing,
}

/// A record's flow as the report writes it, ready to spill.
fn spilled_row(record: &mut Record) -> Option<Result<Taken, String>> {
    let flow = record.flow.take()?;
    let standing = flow.standing;
    let row = FlowRow {
        function: record.function.clone(),
        name: record.name.clone(),
        path: record.path.clone(),
        flow,
    };
    let key = record.owned_canonical_key();
    Some(
        serde_json::to_vec(&row)
            .map(|bytes| Taken {
                key,
                bytes,
                standing,
            })
            .map_err(|error| error.to_string()),
    )
}

/// Flow rows written so far, with where each one is.
struct Spill {
    file: BufWriter<File>,
    written: u64,
    rows: Vec<Row>,
}

struct Row {
    key: CanonicalKey,
    at: u64,
    len: usize,
    standing: Standing,
}

impl Spill {
    // rstyle-allow(side_effect_density): IO boundary: opens the spill file
    fn new() -> Result<Self, String> {
        let file = tempfile::tempfile().map_err(|error| format!("Cannot spill flow: {error}"))?;
        Ok(Self {
            file: BufWriter::new(file),
            written: 0,
            rows: Vec::new(),
        })
    }

    // rstyle-allow(side_effect_density): IO boundary: appends rows to the spill file
    fn write(&mut self, rows: Vec<Taken>) -> Result<(), String> {
        for Taken {
            key,
            bytes,
            standing,
        } in rows
        {
            self.file
                .write_all(&bytes)
                .map_err(|error| format!("Cannot spill flow: {error}"))?;
            self.rows.push(Row {
                key,
                at: self.written,
                len: bytes.len(),
                standing,
            });
            self.written += bytes.len() as u64;
        }
        Ok(())
    }

    /// The rows in the order `take_flow` takes them from records in canonical order.
    // rstyle-allow(side_effect_density): IO boundary: flushes the spill file
    fn finish(mut self) -> Result<SpilledFlow, String> {
        let file = self
            .file
            .into_inner()
            .map_err(|error| format!("Cannot spill flow: {}", error.error()))?;
        self.rows.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(SpilledFlow {
            standings: self.rows.iter().map(|row| row.standing).collect(),
            rows: self.rows.into_iter().map(|row| (row.at, row.len)).collect(),
            file: Mutex::new(file),
        })
    }
}

/// A run's flow rows, held on disk. Serializing reads each row back as a `FlowRow`, so the
/// report reads exactly as it would from rows held in memory, compact or pretty.
pub struct SpilledFlow {
    file: Mutex<File>,
    rows: Vec<(u64, usize)>,
    standings: Vec<Standing>,
}

impl SpilledFlow {
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Each row's standing, in report order.
    pub fn standings(&self) -> &[Standing] {
        &self.standings
    }

    // rstyle-allow(side_effect_density): IO boundary: reads one row back from the spill file
    fn row(&self, (at, len): (u64, usize)) -> Result<FlowRow, String> {
        let mut file = self.file.lock().map_err(|_| POISONED)?;
        let mut bytes = vec![0; len];
        file.seek(SeekFrom::Start(at))
            .and_then(|_| file.read_exact(&mut bytes))
            .map_err(|error| format!("Cannot read spilled flow: {error}"))?;
        serde_json::from_slice(&bytes).map_err(|error| format!("Cannot read spilled flow: {error}"))
    }
}

impl serde::Serialize for SpilledFlow {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.rows.len()))?;
        for &place in &self.rows {
            sequence.serialize_element(&self.row(place).map_err(S::Error::custom)?)?;
        }
        sequence.end()
    }
}
