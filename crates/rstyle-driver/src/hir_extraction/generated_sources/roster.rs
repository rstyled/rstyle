//! Enumerate every compiler local definition in the selected expansion closure.
use super::*;
use std::collections::BTreeSet;

pub(super) fn capture<'tcx>(
    provenance: &mut Provenance<'_, 'tcx>,
    roots: &mut Roots<'tcx>,
    selected: &BTreeSet<String>,
) -> (Vec<Emitted>, Vec<Outcome>) {
    if selected.is_empty() {
        return (Vec::new(), Vec::new());
    }

    Roster {
        provenance,
        roots,
        selected,
        records: Vec::new(),
        errors: Vec::new(),
    }
    .finish()
}

struct Roster<'a, 'source, 'tcx> {
    provenance: &'a mut Provenance<'source, 'tcx>,
    roots: &'a mut Roots<'tcx>,
    selected: &'a BTreeSet<String>,
    records: Vec<Emitted>,
    errors: Vec<Outcome>,
}

impl Roster<'_, '_, '_> {
    fn finish(mut self) -> (Vec<Emitted>, Vec<Outcome>) {
        for id in self.provenance.tcx.iter_local_def_id() {
            self.definition(id);
            if self.stop_at_budget(id) {
                break;
            }
        }

        self.records.sort_by(|left, right| {
            left.definition
                .path
                .cmp(&right.definition.path)
                .then_with(|| {
                    left.definition
                        .definition_identity
                        .cmp(&right.definition.definition_identity)
                })
        });
        (self.records, self.errors)
    }

    fn definition(&mut self, id: LocalDefId) {
        let root = match self.roots.definition(id) {
            Ok(Some(root)) if self.selected.contains(&identity(root)) => root,
            Ok(_) => return,
            Err(reason) => {
                self.error(id, reason);
                return;
            }
        };

        self.emit(id, root);
    }

    fn emit(&mut self, id: LocalDefId, root: ExpnId) {
        match record(self.provenance, id, root) {
            Ok(record) => self.records.push(record),
            Err(reason) => self.error(id, reason),
        }
    }

    fn error(&mut self, id: LocalDefId, reason: String) {
        self.errors.push(Outcome::Unsupported {
            owner: definition_record(self.provenance.tcx, id.to_def_id()),
            reason,
        });
    }

    fn stop_at_budget(&mut self, id: LocalDefId) -> bool {
        if self.records.len() + self.errors.len() < 65536 {
            return false;
        }

        self.error(
            id,
            "Generated emitted-definition roster exceeds diagnostic budget".into(),
        );
        true
    }
}

fn record(
    provenance: &mut Provenance<'_, '_>,
    id: LocalDefId,
    root: ExpnId,
) -> Result<Emitted, String> {
    let tcx = provenance.tcx;
    provenance.expansion(root)?;
    // Keep the defining expansion even when a descendant's span has root context.
    provenance.expansion(tcx.expn_that_defined(id.to_def_id()))?;

    Ok(Emitted {
        definition: definition_record(tcx, id.to_def_id()),
        parent: definition_record(tcx, tcx.parent(id.to_def_id())),
        kind: format!("{:?}", tcx.def_kind(id)),
        origin: provenance.origin(tcx.def_span(id))?,
    })
}
