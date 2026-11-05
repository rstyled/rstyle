//! A compilation's `compat.cpg` documents, as its frontend published them, read through
//! the core. Nothing here needs a compiler, so a saved run reads them the same way.
use super::capture;
use rstyle_core::cpg::{
    self,
    model::Document,
    records::{Demand, Measured, Record, records},
};
use rstyle_pdg::{
    BuildFacts, cpg_reader,
    unit_store::{DOCUMENTS_MANIFEST, DocumentManifest, transport_checksum},
};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

/// Documents of one compilation read at once: each is bounded by its source file.
const READERS: usize = 8;

/// One function of a compilation, as the routes over documents report it.
pub(super) struct DocumentFunction {
    pub record: Record,
    pub digest: String,
    pub build: Arc<BuildFacts>,
    pub source_file: Option<String>,
    /// A macro writes its signature, as the frontend's source scan records.
    pub generated: bool,
}

impl DocumentFunction {
    /// The identity of the function's measured facts, or of its generated events.
    pub fn compiled_id(&self) -> Option<&str> {
        match &self.record.measured {
            Measured::Known(facts) => Some(&facts.id),
            Measured::Generated(observation) => observation
                .connections
                .as_ref()
                .map(|connections| connections.id.as_str()),
            Measured::Unavailable => None,
        }
    }
}

// rstyle-allow(side_effect_density): IO boundary: reads one sealed file of a unit
/// The published manifest of a unit, checked against the seal its ledger recorded.
pub(super) fn manifest(unit: &capture::Unit) -> Result<DocumentManifest, String> {
    let bytes = captured(&unit.directory, DOCUMENTS_MANIFEST)?;
    let seal = unit
        .artifacts
        .iter()
        .find(|artifact| artifact.file == DOCUMENTS_MANIFEST)
        .ok_or("Missing document manifest inventory entry")?;
    if seal.checksum != transport_checksum(&bytes) {
        return Err(format!(
            "captured artifact checksum mismatch: {DOCUMENTS_MANIFEST}"
        ));
    }

    DocumentManifest::read(&bytes)
}

// rstyle-allow(side_effect_density): IO boundary: reads one captured file
fn captured(directory: &Path, file: &str) -> Result<Vec<u8>, String> {
    std::fs::read(directory.join(file))
        .map_err(|error| format!("missing captured artifact {file}: {error}"))
}

/// Every function of a unit, in document order.
pub(super) fn read(unit: &capture::Unit, demand: Demand) -> Result<Vec<DocumentFunction>, String> {
    let manifest = manifest(unit)?;
    let read = parallel(&manifest.documents, |sealed| {
        let bytes = captured(&unit.directory, &sealed.file)?;
        sealed.verify(&bytes)?;
        let document = cpg::layout::read(&bytes)?;
        // A document's bytes are its size again; only its typed form is read from here on.
        drop(bytes);

        functions(&document, demand)
    })?;
    Ok(read.into_iter().flatten().collect())
}

fn functions(document: &Document, demand: Demand) -> Result<Vec<DocumentFunction>, String> {
    let build = Arc::new(cpg_reader::unit_build(document)?);
    let unit = document.unit.as_ref().ok_or("The document has no unit")?;
    let bodies = unit.functions.as_deref().unwrap_or_default();

    Ok(records(document, demand)?
        .into_iter()
        .zip(bodies)
        .map(|(record, body)| DocumentFunction {
            digest: transport_checksum(record.source.as_bytes()),
            build: Arc::clone(&build),
            source_file: cpg_reader::session_path(unit, &record.located.file).map(str::to_owned),
            generated: cpg_reader::generated_signature(body),
            record,
        })
        .collect())
}

// rstyle-allow(side_effect_density): worker boundary: claims items and retains each result in its place
/// Applies `read` to every item at once, keeping their order; the first failure wins.
fn parallel<T: Sync, R: Send>(
    items: &[T],
    read: impl Fn(&T) -> Result<R, String> + Sync,
) -> Result<Vec<R>, String> {
    let next = AtomicUsize::new(0);
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .clamp(1, READERS)
        .min(items.len().max(1));
    let worker = || {
        let mut read_here = Vec::new();
        loop {
            let index = next.fetch_add(1, Ordering::Relaxed);
            let Some(item) = items.get(index) else {
                return read_here;
            };
            read_here.push((index, read(item)));
        }
    };
    let mut read: Vec<(usize, Result<R, String>)> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..workers).map(|_| scope.spawn(worker)).collect();
        workers
            .into_iter()
            .flat_map(|worker| worker.join().expect("A document reader panicked"))
            .collect()
    });
    read.sort_by_key(|(index, _)| *index);

    read.into_iter().map(|(_, result)| result).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parallel_keeps_item_order() {
        let items: Vec<usize> = (0..200).collect();
        let read = parallel(&items, |item| Ok::<_, String>(item * 2)).unwrap();
        assert_eq!(read, items.iter().map(|item| item * 2).collect::<Vec<_>>());
    }

    #[test]
    fn parallel_reports_the_first_failure_in_item_order() {
        let items: Vec<usize> = (0..50).collect();
        let error = parallel(&items, |item| match item % 7 {
            3 => Err(format!("failed at {item}")),
            _ => Ok(*item),
        })
        .unwrap_err();
        assert_eq!(error, "failed at 3");
    }

    #[test]
    fn parallel_reads_nothing_from_no_items() {
        let read = parallel::<usize, usize>(&[], |item| Ok(*item)).unwrap();
        assert!(read.is_empty());
    }
}
