//! Bounded successful chain receipts belonging to one exact immutable reader owner.
use super::*;

const MAX_CHAINS: usize = 32;

#[derive(Default)]
pub(in crate::nominal_boundary::resolution) struct Memo {
    owner: Option<Owner>,
    records: Vec<Observation>,
}

#[derive(Clone)]
pub(super) struct Owner {
    definition: String,
    file: String,
    span: ByteSpan,
    source: crate::SourceText,
    build: crate::BuildFacts<()>,
    capture: Arc<UnitCapture>,
}

impl Owner {
    pub(super) fn capture(input: &dyn EvidenceFacts) -> Result<Self, String> {
        let facts = input.facts();
        let options = &facts.build.frontend_options;
        Ok(Self {
            definition: options
                .get("definition_identity")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
                .ok_or("ID SIZE memo lacks exact reader identity")?
                .into(),
            file: options
                .get("source_file")
                .and_then(serde_json::Value::as_str)
                .ok_or("ID SIZE memo lacks original reader file")?
                .into(),
            span: facts.fn_span,
            source: facts.source.bytes.clone(),
            build: facts.build.settings(),
            capture: input
                .generated_source_capture()?
                .ok_or("Missing ID SIZE unit capture")?,
        })
    }

    fn same(&self, other: &Self) -> bool {
        self.definition == other.definition
            && self.file == other.file
            && self.span == other.span
            && self.source == other.source
            && self.build == other.build
            && Arc::ptr_eq(&self.capture, &other.capture)
    }
}

impl Memo {
    pub(super) fn contains(&self, owner: &Owner, record: &Observation) -> Result<bool, String> {
        if self
            .owner
            .as_ref()
            .is_some_and(|checked| !checked.same(owner))
        {
            return Err("ID SIZE chain receipt belongs to different reader evidence".into());
        }
        Ok(self.records.contains(record))
    }

    /// Called only after the entire independently checked chain succeeds.
    pub(super) fn retain(&mut self, owner: Owner, record: Observation) {
        if self.records.len() < MAX_CHAINS && !self.records.contains(&record) {
            self.owner = Some(owner);
            self.records.push(record);
        }
    }
}

#[cfg(test)]
mod tests;
