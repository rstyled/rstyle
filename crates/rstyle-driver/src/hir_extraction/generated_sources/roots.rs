//! Cache definition-to-original-local-bang attribution once per selected unit.
use super::*;
use rustc_hir::def_id::CRATE_DEF_ID;
use rustc_span::hygiene::{ExpnKind, MacroKind};
use std::collections::{BTreeMap, HashMap, HashSet};

pub(super) struct Roots<'tcx> {
    tcx: TyCtxt<'tcx>,
    definitions: HashMap<LocalDefId, Result<Option<ExpnId>, String>>,
    expansions: HashMap<ExpnId, Result<Option<ExpnId>, String>>,
}

impl<'tcx> Roots<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self {
            tcx,
            definitions: HashMap::new(),
            expansions: HashMap::new(),
        }
    }

    pub fn definition(&mut self, id: LocalDefId) -> Result<Option<ExpnId>, String> {
        if id == CRATE_DEF_ID {
            return Ok(None);
        }

        if let Some(result) = self.definitions.get(&id) {
            return result.clone();
        }

        let mut result = self.expansion(self.tcx.expn_that_defined(id.to_def_id()));
        if matches!(result, Ok(None)) {
            result = self
                .tcx
                .parent(id.to_def_id())
                .as_local()
                .map(|parent| self.definition(parent))
                .unwrap_or(Ok(None));
        }

        self.definitions.insert(id, result.clone());
        result
    }

    fn expansion(&mut self, id: ExpnId) -> Result<Option<ExpnId>, String> {
        if id == ExpnId::root() {
            return Ok(None);
        }

        if let Some(result) = self.expansions.get(&id) {
            return result.clone();
        }

        let mut seen = HashSet::new();
        let mut roots = BTreeMap::new();
        let result = self.collect(id, &mut seen, &mut roots).and_then(|()| {
            if roots.len() > 1 {
                return Err("Generated origin has multiple original local bang roots".into());
            }
            Ok(roots.into_values().next())
        });

        self.expansions.insert(id, result.clone());
        result
    }

    fn collect(
        &self,
        id: ExpnId,
        seen: &mut HashSet<ExpnId>,
        roots: &mut BTreeMap<String, ExpnId>,
    ) -> Result<(), String> {
        if id == ExpnId::root() || !seen.insert(id) {
            return Ok(());
        }
        if seen.len() > 128 {
            return Err("Generated origin exceeds expansion-root budget".into());
        }

        let data = id.expn_data();
        if matches!(data.kind, ExpnKind::Macro(MacroKind::Bang, _))
            && data
                .macro_def_id
                .is_some_and(|definition| definition.is_local())
            && !data.call_site.from_expansion()
        {
            roots.insert(identity(id), id);
        }

        self.collect(data.parent, seen, roots)?;
        for (invocation, _) in data.call_site.ctxt().marks() {
            self.collect(invocation, seen, roots)?;
        }
        Ok(())
    }
}
