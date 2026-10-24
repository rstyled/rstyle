//! Index exact compiler records once; expectation consumption cannot hide surplus.
use super::*;
use crate::nominal_boundary::census::expansions::FileSpan;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    kind: String,
    path: String,
    start: u32,
    end: u32,
    expansion: String,
    transparency: u8,
}

impl Key {
    fn new(kind: &str, path: &str, source: &FileSpan, mark: &Mark) -> Self {
        Self {
            kind: kind.into(),
            path: path.into(),
            start: source.span.start,
            end: source.span.end,
            expansion: mark.expansion.clone(),
            transparency: match mark.transparency {
                Transparency::Opaque => 0,
                Transparency::SemiOpaque => 1,
                Transparency::Transparent => 2,
            },
        }
    }
}

pub(super) struct Expected<'a> {
    pub kind: &'static str,
    pub path: String,
    pub source: &'a FileSpan,
    pub mark: &'a Mark,
    pub parent: &'a Definition,
}

pub(super) struct Inventory<'a> {
    records: BTreeMap<Key, Vec<&'a Emitted>>,
}

impl<'a> Inventory<'a> {
    pub(super) fn new(
        projection: &Projection,
        generated: &'a capture::UnitCapture,
        graph: &derives::Graph<'_>,
        module: &Definition,
    ) -> Result<Self, ProofError> {
        let mut records: BTreeMap<Key, Vec<&Emitted>> = BTreeMap::new();
        let mut identities = BTreeSet::new();
        for item in &generated.emitted {
            if !selected(projection, item, graph) {
                continue;
            }
            owner(&item.definition, module)?;
            owner(&item.parent, module)?;
            if !identities.insert(&item.definition.definition_identity) {
                return Err(ProofError::Contradictory(
                    "Generated roster repeats a compiler identity",
                ));
            }
            let [mark] = item.origin.marks.as_slice() else {
                return Err(ProofError::Contradictory(
                    "Generated roster hygiene is not exact",
                ));
            };
            if !graph.contains(&mark.expansion)
                || item.origin.raw.path != projection.full_definition().path
                || item.origin.raw.digest != projection.full_definition().digest
            {
                return Err(ProofError::Contradictory(
                    "Generated roster source/expansion ownership differs",
                ));
            }
            let key = Key::new(&item.kind, &item.definition.path, &item.origin.raw, mark);
            records.entry(key).or_default().push(item);
        }
        Ok(Self { records })
    }

    pub(super) fn take(&mut self, expected: Expected<'_>) -> Result<&'a Emitted, ProofError> {
        let key = Key::new(
            expected.kind,
            &expected.path,
            expected.source,
            expected.mark,
        );
        let item = self
            .records
            .get_mut(&key)
            .and_then(Vec::pop)
            .ok_or(ProofError::Missing(
                "Missing exact source-derived generated impl/member record",
            ))?;
        if item.parent != *expected.parent {
            return Err(ProofError::Contradictory(
                "Generated roster exact parent identity differs",
            ));
        }
        Ok(item)
    }

    pub(super) fn peek(
        &self,
        kind: &str,
        path: &str,
        source: &FileSpan,
    ) -> Result<&'a Emitted, ProofError> {
        let lower = Key {
            kind: kind.into(),
            path: path.into(),
            start: source.span.start,
            end: source.span.end,
            expansion: String::new(),
            transparency: 0,
        };
        let mut matching = self
            .records
            .range(lower..)
            .take_while(|(key, _)| {
                key.kind == kind
                    && key.path == path
                    && key.start == source.span.start
                    && key.end == source.span.end
            })
            .flat_map(|(_, records)| records.iter().copied());
        let record = matching
            .next()
            .ok_or(ProofError::Missing("Missing exact derived unit impl"))?;
        if matching.next().is_some() {
            return Err(ProofError::Contradictory(
                "Ambiguous derived unit implementation",
            ));
        }
        Ok(record)
    }

    pub(super) fn finish(self) -> Result<(), ProofError> {
        if self.records.values().any(|records| !records.is_empty()) {
            return Err(ProofError::Contradictory(
                "Surplus unchecked generated impl/member records",
            ));
        }
        Ok(())
    }
}

fn selected(projection: &Projection, item: &Emitted, graph: &derives::Graph<'_>) -> bool {
    let namespace = projection.namespace().join("::");
    let source = &item.origin.raw;
    item.origin
        .marks
        .iter()
        .any(|mark| graph.contains(&mark.expansion))
        || item.definition.path.starts_with(&format!("{namespace}::"))
        || item.definition.path.starts_with(&format!("<{namespace}::"))
        || source.path == projection.full_definition().path
            && source.span.start >= projection.full_definition().span.start
            && source.span.end <= projection.full_definition().span.end
}

fn owner(definition: &Definition, module: &Definition) -> Result<(), ProofError> {
    if !definition.local
        || definition.path.is_empty()
        || definition.definition_identity.is_empty()
        || definition.crate_name != module.crate_name
        || definition.stable_crate_id != module.stable_crate_id
    {
        return Err(ProofError::Contradictory(
            "Generated roster compiler owner differs",
        ));
    }
    Ok(())
}

/// A source-template record expectation before binding snapshot and hygiene.
pub(super) struct TemplateRecord<'a> {
    pub kind: &'static str,
    pub path: String,
    pub parent: &'a Definition,
    pub role: TemplateRole,
}

pub(super) struct Context<'p, 'r, 'g> {
    pub projection: &'p Projection,
    pub inventory: &'r mut Inventory<'g>,
    pub mark: Mark,
}

impl<'g> Context<'_, '_, 'g> {
    pub(super) fn take(&mut self, record: TemplateRecord<'_>) -> Result<&'g Emitted, ProofError> {
        self.inventory.take(Expected {
            kind: record.kind,
            path: record.path,
            parent: record.parent,
            mark: &self.mark,
            source: self.projection.template(record.role),
        })
    }
}
