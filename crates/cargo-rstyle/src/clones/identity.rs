//! Report-local function labels, separate from evidence and matching authority.
use super::capture;
use rstyle_pdg::HirFnFacts;
use serde::Serialize;
use std::{collections::BTreeSet, fmt, path::Path};

pub(super) const SCHEMA: &str = "rstyle.report-function/2";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub(super) struct UnitId(u32);

impl UnitId {
    pub(super) fn ordinal(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct FunctionKey {
    unit: UnitId,
    function: u32,
}

impl FunctionKey {
    fn new(unit: UnitId, function: usize) -> Result<Self, String> {
        Ok(Self {
            unit,
            function: u32::try_from(function).map_err(|_| "Oversized report function table")?,
        })
    }

    pub(super) fn unit(self) -> UnitId {
        self.unit
    }
}

impl fmt::Display for FunctionKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "function:u{}:f{}", self.unit.0, self.function)
    }
}

impl Serialize for FunctionKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// `observed_unit` resolves only in this report's observed compiler-unit table.
#[derive(Serialize)]
pub(super) struct UnitContext {
    pub id: UnitId,
    pub observed_unit: usize,
}

pub(super) struct Assignment<'a> {
    pub context: UnitContext,
    pub unit: &'a capture::Unit,
}

pub(super) fn assign(units: &[capture::Unit]) -> Result<Vec<Assignment<'_>>, String> {
    let mut ordered: Vec<_> = units.iter().enumerate().collect();
    ordered.sort_by(|(_, a), (_, b)| order(a).cmp(&order(b)));
    if ordered
        .windows(2)
        .any(|pair| order(pair[0].1) == order(pair[1].1))
    {
        return Err("Duplicate or contradictory report compilation-unit descriptor".into());
    }

    ordered
        .into_iter()
        .enumerate()
        .map(|(index, (observed_unit, unit))| {
            Ok(Assignment {
                context: UnitContext {
                    id: UnitId(u32::try_from(index).map_err(|_| "Oversized report unit table")?),
                    observed_unit,
                },
                unit,
            })
        })
        .collect()
}

fn order(unit: &capture::Unit) -> (&str, &Path, &str, &[String], bool, &[String]) {
    (
        &unit.target.package,
        &unit.target.source,
        &unit.target.name,
        &unit.target.kind,
        unit.test,
        &unit.invocation,
    )
}

/// HIR owners distinguish definitions within this unit, including expansions.
/// Display paths and callsite spans are navigation only, not definition identity.
pub(super) trait BodyOwner {
    fn owner(&self) -> &rstyle_pdg::HirRef;
}

impl BodyOwner for rstyle_pdg::HirNode {
    fn owner(&self) -> &rstyle_pdg::HirRef {
        &self.hir
    }
}

impl BodyOwner for rstyle_pdg::HirRef {
    fn owner(&self) -> &rstyle_pdg::HirRef {
        self
    }
}

pub(super) fn functions<B: BodyOwner>(
    unit: UnitId,
    facts: &[HirFnFacts<B>],
) -> Result<Vec<FunctionKey>, String> {
    let mut seen = BTreeSet::new();
    facts
        .iter()
        .enumerate()
        .map(|(index, function)| {
            function
                .build
                .frontend_options
                .get("source_file")
                .and_then(serde_json::Value::as_str)
                .ok_or("missing captured source-file navigation")?;
            let owner = function.body.owner();
            if !seen.insert((owner.krate, owner.owner)) {
                return Err("Duplicate compiler function owner in one compilation unit".into());
            }
            FunctionKey::new(unit, index)
        })
        .collect()
}

#[cfg(test)]
mod tests;
