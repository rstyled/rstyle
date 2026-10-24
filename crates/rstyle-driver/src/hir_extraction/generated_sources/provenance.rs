//! Read compiler-owned raw spans/hygiene without changing historical HIR spans.
pub(super) use super::super::instances::definition_record;
use super::*;
use rustc_span::hygiene::{ExpnKind, MacroKind, Transparency as CompilerTransparency};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Provenance<'a, 'tcx> {
    pub tcx: TyCtxt<'tcx>,
    sources: &'a super::super::sources::Sources,
    expansions: BTreeMap<String, Expansion>,
    visiting: BTreeSet<String>,
}

impl<'a, 'tcx> Provenance<'a, 'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>, sources: &'a super::super::sources::Sources) -> Self {
        Self {
            tcx,
            sources,
            expansions: BTreeMap::new(),
            visiting: BTreeSet::new(),
        }
    }

    pub fn finish(self) -> Vec<Expansion> {
        self.expansions.into_values().collect()
    }

    pub fn origin(&mut self, span: Span) -> Result<Origin, String> {
        if span.is_dummy() {
            return Err("Generated provenance contains a dummy raw span".into());
        }

        let raw = super::super::module_expansions::file_span(self.tcx, self.sources, span)?;
        let mut marks = Vec::new();
        for (expansion, transparency) in span.ctxt().marks() {
            self.expansion(expansion)?;
            marks.push(Mark {
                expansion: identity(expansion),
                transparency: match transparency {
                    CompilerTransparency::Opaque => Transparency::Opaque,
                    CompilerTransparency::SemiOpaque => Transparency::SemiOpaque,
                    CompilerTransparency::Transparent => Transparency::Transparent,
                },
            });
        }

        Ok(Origin { raw, marks })
    }

    pub fn expansion(&mut self, id: ExpnId) -> Result<(), String> {
        if id == ExpnId::root() {
            return Ok(());
        }

        let key = identity(id);
        if self.expansions.contains_key(&key) || self.visiting.contains(&key) {
            return Ok(());
        }

        if self.visiting.len() >= 128 || self.expansions.len() >= 8192 {
            return Err("Generated expansion ancestry exceeds diagnostic budget".into());
        }

        let record = self.visited_record(id, &key)?;
        self.expansions.insert(key, record);
        Ok(())
    }

    fn visited_record(&mut self, id: ExpnId, key: &str) -> Result<Expansion, String> {
        self.visiting.insert(key.into());
        let result = self.expansion_record(id);
        self.visiting.remove(key);
        result
    }

    fn expansion_record(&mut self, id: ExpnId) -> Result<Expansion, String> {
        let data = id.expn_data();
        self.expansion(data.parent)?;
        let invocation = self.origin(data.call_site)?;
        let definition = match data.macro_def_id {
            Some(id) if id.is_local() => Some(super::super::module_expansions::file_span(
                self.tcx,
                self.sources,
                self.tcx.def_span(id),
            )?),
            _ => None,
        };

        let (kind, name) = match data.kind {
            ExpnKind::Macro(kind, name) => (
                match kind {
                    MacroKind::Bang => ExpansionKind::Bang,
                    MacroKind::Attr => ExpansionKind::Attr,
                    MacroKind::Derive => ExpansionKind::Derive,
                },
                name.to_string(),
            ),
            ExpnKind::Desugaring(kind) => (ExpansionKind::Desugaring, format!("{kind:?}")),
            ExpnKind::AstPass(kind) => (ExpansionKind::AstPass, format!("{kind:?}")),
            ExpnKind::Root => return Err("Root expansion cannot supply generated ownership".into()),
        };

        Ok(Expansion {
            expansion: identity(id),
            parent: (data.parent != ExpnId::root()).then(|| identity(data.parent)),
            kind,
            name,
            macro_definition: data.macro_def_id.map(|id| definition_record(self.tcx, id)),
            definition,
            invocation,
            edition: data.edition.to_string(),
            local_inner_macros: data.local_inner_macros,
        })
    }
}

pub(super) fn identity(expansion: ExpnId) -> String {
    // Read rustc's existing stable identity; do not hash or serialize source.
    format!("{:?}", expansion.expn_hash())
}
