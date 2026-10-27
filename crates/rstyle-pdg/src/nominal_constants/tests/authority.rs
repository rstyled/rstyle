use super::super::*;
use super::{fixture, fixture::Fixture};
use crate::{HirNode, data_library::Callee};

impl Authority for Fixture {
    fn actual(&self, node: &HirNode, record: &ProductConstant) -> Result<(), String> {
        if node.snippet != "SmallIndex::MAX" || record.definition != self.record.definition {
            return Err("Fixture actual source/definition association refused".into());
        }
        Ok(())
    }

    fn build(&self) -> Result<BuildContext<'_>, String> {
        if self.deny_authority {
            return Err("Fixture independent build authority unavailable".into());
        }
        Ok(BuildContext {
            pointer_width: 64,
            cfgs: &self.cfgs,
        })
    }

    fn sources(&self, _: &ProductConstant) -> Result<SourceSelection<'_>, String> {
        Ok(self.selection())
    }

    fn local(&self, definition: &Callee, source: &SourceSite) -> Result<(), String> {
        let selected = [
            (&self.record.definition, &self.record.declaration),
            (
                &self.record.implementation,
                &self.record.implementation_source,
            ),
            (
                &self.record.constructor.definition,
                &self.record.constructor.declaration,
            ),
            (
                &self.record.constructor.implementation,
                &self.record.constructor.implementation_source,
            ),
        ];
        if !selected.iter().any(|(identity, site)| {
            *identity == definition && site.file == source.file && site.span == source.span
        }) {
            return Err("Fixture original local source/identity association refused".into());
        }
        Ok(())
    }

    fn bindings(&self, record: &ProductConstant, _: &SourceSelection<'_>) -> Result<(), String> {
        if record.constructor.tuple_constructor != fixture::local("SmallIndex::{constructor#0}") {
            return Err("Fixture selected tuple constructor refused".into());
        }
        Ok(())
    }

    fn owned_copy(&self, record: &ProductConstant) -> Result<(), String> {
        if record.ty != fixture::owner() {
            return Err("Fixture original complete nominal Copy owner refused".into());
        }
        Ok(())
    }

    fn core_constant(&self, record: &CoreConstant) -> Result<(), String> {
        if record.definition != fixture::core().definition {
            return Err("Fixture independent reviewed core authority refused".into());
        }
        Ok(())
    }
}
