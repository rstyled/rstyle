//! Derive each bundled unit's records before reading the following document.
use super::{
    layout::{self, Parsed},
    model::Document,
    records::{self, Demand, Record},
};
use serde_json::Value;

/// Bundled units are released after their records are derived. Split documents are
/// retained until EOF to preserve arbitrary input ordering.
pub fn read_records(
    documents: impl IntoIterator<Item = Result<Parsed, String>>,
    demand: Demand,
) -> Result<Vec<Record>, String> {
    let mut records = Vec::new();

    visit_records(documents, demand, |batch| {
        records.extend(batch);
        Ok(())
    })?;
    Ok(records)
}

/// Visits derived records without retaining all of them. Earlier batches are
/// provisional: a later malformed document or duplicate can still reject the run.
pub fn visit_records(
    documents: impl IntoIterator<Item = Result<Parsed, String>>,
    demand: Demand,
    mut visit: impl FnMut(Vec<Record>) -> Result<(), String>,
) -> Result<(), String> {
    let mut reader = RunReader {
        demand,
        split: Vec::new(),
        functions: layout::FunctionIds::default(),
    };

    for document in documents {
        visit(reader.read(document?)?)?;
    }
    reader.finish(visit)
}

struct RunReader {
    demand: Demand,
    split: Vec<Value>,
    functions: layout::FunctionIds,
}

impl RunReader {
    fn read(&mut self, document: Parsed) -> Result<Vec<Record>, String> {
        match document {
            Parsed::Bundled(document) => self.bundled(*document),
            Parsed::Split(document) => {
                self.split.push(document);
                Ok(Vec::new())
            }
        }
    }

    fn bundled(&mut self, document: Document) -> Result<Vec<Record>, String> {
        self.functions.include(&document)?;
        records::records(&document, self.demand)
    }

    fn finish(
        mut self,
        mut visit: impl FnMut(Vec<Record>) -> Result<(), String>,
    ) -> Result<(), String> {
        let groups = layout::split_runs(std::mem::take(&mut self.split));

        for group in groups {
            visit(self.bundled(layout::bundle(group)?)?)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
