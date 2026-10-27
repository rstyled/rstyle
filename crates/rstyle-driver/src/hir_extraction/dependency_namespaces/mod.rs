//! Unit-local compiler observations, never checked namespace authority.
mod exports;
mod sites;

use super::{instances::definition_record, sources::Sources};
use rstyle_pdg::{
    data_receivers::MAX_RECORDS,
    dependency_namespaces::capture::{self, UnitCapture, Unsupported},
    nominal_boundary::census::{MAX_BYTES, MAX_FILES},
};
use rustc_hir::{ItemKind, def_id::DefId};
use rustc_middle::ty::TyCtxt;

pub(super) fn capture(tcx: TyCtxt<'_>, sources: &Sources) -> Result<UnitCapture, String> {
    let mut capture = UnitCapture {
        identity: capture::IDENTITY.into(),
        imports: Vec::new(),
        declarations: Vec::new(),
        unsupported: Vec::new(),
    };
    let mut budget = Budget::default();
    for id in tcx.hir_free_items() {
        let item = tcx.hir_item(id);
        let ItemKind::Use(tree) = item.kind else {
            continue;
        };
        let context = Context {
            tcx,
            sources,
            output: &mut capture,
            budget: &mut budget,
        };
        sites::capture(context, item, tree)?;
    }
    Ok(capture)
}

struct Context<'a, 'tcx> {
    tcx: TyCtxt<'tcx>,
    sources: &'a Sources,
    output: &'a mut UnitCapture,
    budget: &'a mut Budget,
}

impl Context<'_, '_> {
    fn unsupported(&mut self, definition: DefId, reason: impl Into<String>) -> Result<(), String> {
        let request = UnsupportedRequest {
            definition,
            reason: reason.into(),
        };
        let record = self.budget.unsupported(self.tcx, request)?;

        self.output.unsupported.push(record);
        Ok(())
    }
}

#[derive(Default)]
struct Budget {
    records: usize,
    bytes: usize,
}

struct UnsupportedRequest {
    definition: DefId,
    reason: String,
}

impl Budget {
    fn unsupported(
        &mut self,
        tcx: TyCtxt<'_>,
        request: UnsupportedRequest,
    ) -> Result<Unsupported, String> {
        self.record()?;
        let UnsupportedRequest { definition, reason } = request;
        self.text(&reason)?;
        let definition = self.definition(tcx, definition)?;

        Ok(Unsupported { definition, reason })
    }

    fn definition(
        &mut self,
        tcx: TyCtxt<'_>,
        id: DefId,
    ) -> Result<rstyle_pdg::generated_sources::capture::Definition, String> {
        let definition = definition_record(tcx, id);
        for text in [
            &definition.path,
            &definition.definition_identity,
            &definition.crate_name,
            &definition.stable_crate_id,
        ] {
            self.text(text)?;
        }
        Ok(definition)
    }

    fn record(&mut self) -> Result<(), String> {
        self.records += 1;
        if self.records > MAX_RECORDS {
            return Err("Dependency namespace inventory exceeds capture record budget".into());
        }
        Ok(())
    }

    fn text(&mut self, value: &str) -> Result<(), String> {
        self.bytes = self
            .bytes
            .checked_add(value.len())
            .ok_or("Namespace byte overflow")?;
        if self.bytes > MAX_BYTES {
            return Err("Dependency namespace inventory exceeds capture byte budget".into());
        }
        Ok(())
    }
}
