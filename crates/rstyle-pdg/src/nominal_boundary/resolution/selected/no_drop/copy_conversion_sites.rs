//! Original tagged Copy conversion source and target cleanup only.
//! Core trait selection, namespace permission, values and origins are separate.
mod checked;
mod owner;
mod source;
mod targets;
#[cfg(test)]
mod tests;

pub(crate) use checked::CheckedConversion;
pub(in crate::nominal_boundary::resolution) use checked::capture_checked;

use super::*;
use crate::nominal_boundary::{
    census::expansions::FileSpan,
    resolution::{Namespace, Resolution},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::nominal_boundary::resolution) enum Direction {
    Wrap,
    Unwrap,
}

pub(in crate::nominal_boundary::resolution) struct Endpoints {
    direction: Direction,
    input: TypeTree,
    output: TypeTree,
}

impl Endpoints {
    pub(in crate::nominal_boundary::resolution) fn direction(&self) -> Direction {
        self.direction
    }
    pub(in crate::nominal_boundary::resolution) fn input(&self) -> &TypeTree {
        &self.input
    }
    pub(in crate::nominal_boundary::resolution) fn output(&self) -> &TypeTree {
        &self.output
    }
    pub(in crate::nominal_boundary::resolution) fn product(&self) -> &TypeTree {
        match &self.output {
            TypeTree::Result { ok, .. } => ok,
            _ => &self.input,
        }
    }
    pub(in crate::nominal_boundary::resolution) fn tagged(&self) -> &TypeTree {
        match self.direction {
            Direction::Wrap => &self.output,
            Direction::Unwrap => &self.input,
        }
    }
    pub(in crate::nominal_boundary::resolution) fn error(&self) -> Option<&TypeTree> {
        match &self.output {
            TypeTree::Result { err, .. } => Some(err),
            _ => None,
        }
    }
}

/// Nonserializable exact source membership, never a checked core trait member or
/// namespace certificate. It retains only the exact conversion source anchors.
pub(in crate::nominal_boundary::resolution) struct CheckedSource {
    endpoints: Endpoints,
    namespace: Namespace,
    targets: Vec<targets::Target>,
    implementation: FileSpan,
    method: FileSpan,
    associated_error: Option<FileSpan>,
}

impl CheckedSource {
    pub(in crate::nominal_boundary::resolution) fn endpoints(&self) -> &Endpoints {
        &self.endpoints
    }
    pub(in crate::nominal_boundary::resolution) fn implementation(&self) -> &FileSpan {
        &self.implementation
    }
    pub(in crate::nominal_boundary::resolution) fn method(&self) -> &FileSpan {
        &self.method
    }
    pub(in crate::nominal_boundary::resolution) fn associated_error(&self) -> Option<&FileSpan> {
        self.associated_error.as_ref()
    }
}

pub(in crate::nominal_boundary::resolution) fn capture(
    facts: &HirFnFacts,
    resolution: &Resolution,
    origins: &Origins,
) -> Result<CheckedSource, String> {
    let interface = facts
        .build
        .frontend_options
        .data::<InterfaceObservations>()?;
    interface.validate()?;
    let endpoints = targets::endpoints(&interface)?;
    resolution.verify_transport_origins(&interface, origins, facts)?;
    let targets = targets::verify(facts, resolution, origins, &endpoints)?;

    let function = crate::data_frontend::source_function(facts, Some(resolution))?;
    source::function(&function, &endpoints)?;
    let site = owner::find(facts, resolution, &endpoints)?;
    crate::source_evidence::verify(facts)?;

    Ok(CheckedSource {
        endpoints,
        namespace: site.namespace,
        targets,
        implementation: site.implementation,
        method: site.method,
        associated_error: site.associated_error,
    })
}
