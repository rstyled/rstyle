//! Independently checked owner-local scalar projections.
//!
//! This proves source/substitution, resolved bounds, assertions and operations.
//! It grants no caller/request, artifact, helper-summary or admission authority.
pub(crate) mod activation;
mod comparisons;
mod owner;

/// Reuse exact original lexical ownership without scalar substitution authority.
pub(crate) fn verify_lexical_owner(original: &crate::HirFnFacts) -> Result<(), String> {
    owner::lexical(original)
}
mod provenance;
mod source;

use crate::{BindingFact, ByteSpan, HirFnFacts, HirNode, HirRef, SignatureFacts, SourceFacts};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CAPABILITY: &str = "rust.checked-scalar-instances";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Projection {
    definition: Definition,
    arguments: Vec<Argument>,
    source: SourceFacts,
    fn_span: ByteSpan,
    signature: SignatureFacts,
    bindings: Vec<BindingFact>,
    body: HirNode,
    comparisons: Vec<comparisons::Observation>,
    adjustments: Vec<comparisons::Adjustments>,
    provenance: provenance::Observation,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    path: String,
    definition_identity: String,
    #[serde(rename = "crate")]
    crate_name: String,
    stable_crate_id: String,
    local: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Argument {
    kind: String,
    #[serde(rename = "value")]
    _display_value: String,
    display_only: bool,
    identity_status: String,
    identity: ScalarType,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum ScalarType {
    Builtin { name: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScalarArgument {
    I32,
    U32,
}

/// A complete checked body, still unavailable to ordinary admission.
pub struct CheckedProjection {
    facts: HirFnFacts,
    comparisons: usize,
    scalar: ScalarArgument,
    comparison_nodes: BTreeMap<Site, Vec<usize>>,
}

impl CheckedProjection {
    pub fn facts(&self) -> &HirFnFacts {
        &self.facts
    }

    pub fn scalar_argument(&self) -> ScalarArgument {
        self.scalar
    }

    pub(crate) fn comparison_site(&self, node: &HirNode) -> bool {
        let Some(path) = self.comparison_nodes.get(&site(&node.hir)) else {
            return false;
        };
        let checked = path.iter().try_fold(&self.facts.body, |current, index| {
            current.children.get(*index)
        });
        checked == Some(node)
    }

    pub fn comparison_count(&self) -> usize {
        self.comparisons
    }
}

pub fn verify(original: &HirFnFacts, projection: &Projection) -> Result<CheckedProjection, String> {
    if crate::extraction_profile::ExtractionProfile::from_facts(original)?
        != crate::extraction_profile::ExtractionProfile::DataV3
        || !original.source_scan_complete
        || !original.source_diagnostics.is_empty()
    {
        return Err("Scalar projection requires complete DataV3 original source evidence".into());
    }
    definition(original, &projection.definition)?;
    let scalar = scalar(&projection.arguments)?;
    let projected = materialize(original, projection);

    let source = source::verify(original, &projected, scalar)?;
    provenance::verify(&projection.provenance, &source, scalar)?;
    operations(&source, projection, scalar)?;
    let comparison_nodes = comparison_nodes(&projected, projection)?;
    Ok(CheckedProjection {
        facts: projected,
        comparisons: projection.comparisons.len(),
        scalar: if scalar == "i32" {
            ScalarArgument::I32
        } else {
            ScalarArgument::U32
        },
        comparison_nodes,
    })
}

fn scalar(arguments: &[Argument]) -> Result<&str, String> {
    let [argument] = arguments else {
        return Err("Scalar projection requires one structural argument".into());
    };
    let ScalarType::Builtin { name } = &argument.identity;
    if argument.kind != "type"
        || !argument.display_only
        || argument.identity_status != "known"
        || !matches!(name.as_str(), "i32" | "u32")
    {
        return Err("Scalar projection argument lacks exact i32/u32 identity".into());
    }
    Ok(name)
}

fn definition(original: &HirFnFacts, definition: &Definition) -> Result<(), String> {
    let options = &original.build.frontend_options;
    let owner = options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Scalar projection lacks original owning-crate evidence")?;
    if !definition.local
        || definition.path != original.name
        || options
            .get("definition_identity")
            .and_then(serde_json::Value::as_str)
            != Some(definition.definition_identity.as_str())
        || owner.get("identity").and_then(serde_json::Value::as_str)
            != Some("rstyle.rust.compiled-crate/1")
        || owner.get("crate_name").and_then(serde_json::Value::as_str)
            != Some(definition.crate_name.as_str())
        || owner
            .get("stable_crate_id")
            .and_then(serde_json::Value::as_str)
            != Some(definition.stable_crate_id.as_str())
    {
        return Err("Scalar projection differs from the original compiler definition".into());
    }
    Ok(())
}

fn materialize(original: &HirFnFacts, projection: &Projection) -> HirFnFacts {
    HirFnFacts {
        name: original.name.clone(),
        source: projection.source.clone(),
        fn_span: projection.fn_span,
        signature: projection.signature.clone(),
        bindings: projection.bindings.clone(),
        body: projection.body.clone(),
        build: original.build.clone(),
        source_diagnostics: original.source_diagnostics.clone(),
        source_scan_complete: original.source_scan_complete,
    }
}

fn operations(
    source: &source::Checked<'_>,
    projection: &Projection,
    scalar: &str,
) -> Result<(), String> {
    let facts = source.facts();
    let mut nodes = BTreeMap::new();
    collect(&facts.body, &mut nodes)?;
    let mut comparisons = BTreeMap::new();

    for record in &projection.comparisons {
        let site = site(comparisons::site(record));
        let node = nodes
            .get(&site)
            .ok_or("Comparison names an absent body node")?;
        comparisons::verify(node, record, source)?;
        if comparisons.insert(site, ()).is_some() {
            return Err("Duplicate scalar comparison evidence".into());
        }
    }
    for node in nodes.values() {
        if node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.kind == "overloaded")
            && !comparisons.contains_key(&site(&node.hir))
        {
            return Err("Missing proof for an overloaded body operation".into());
        }
    }
    adjustments(projection, scalar, &nodes)
}

type Site = (u32, u32, u32);

fn site(hir: &HirRef) -> Site {
    (hir.krate, hir.owner, hir.local)
}

fn collect<'a>(node: &'a HirNode, nodes: &mut BTreeMap<Site, &'a HirNode>) -> Result<(), String> {
    if nodes.insert(site(&node.hir), node).is_some() {
        return Err("Duplicate HIR identity in scalar projection".into());
    }
    for child in &node.children {
        collect(child, nodes)?;
    }
    Ok(())
}

fn adjustments(
    projection: &Projection,
    scalar: &str,
    nodes: &BTreeMap<Site, &HirNode>,
) -> Result<(), String> {
    let mut operands = BTreeMap::new();
    for comparison in &projection.comparisons {
        for operand in comparisons::operand_sites(comparison) {
            if operands.insert(site(operand), comparison).is_some() {
                return Err("Duplicate comparison operand adjustment owner".into());
            }
        }
    }

    let mut seen = std::collections::BTreeSet::new();
    for record in &projection.adjustments {
        let site = site(comparisons::adjustment_site(record));
        let node = nodes
            .get(&site)
            .ok_or("Adjustment names an absent body node")?;
        if !seen.insert(site) {
            return Err("Duplicate scalar adjustment evidence".into());
        }
        comparisons::verify_adjustments(node, record, operands.get(&site).copied(), scalar)?;
    }
    if operands.keys().any(|site| !seen.contains(site)) {
        return Err("Missing ordered adjustment evidence for a comparison operand".into());
    }
    Ok(())
}

fn comparison_nodes(
    facts: &HirFnFacts,
    projection: &Projection,
) -> Result<BTreeMap<Site, Vec<usize>>, String> {
    let expected: std::collections::BTreeSet<_> = projection
        .comparisons
        .iter()
        .map(|record| site(comparisons::site(record)))
        .collect();
    let mut paths = BTreeMap::new();
    comparison_paths(&facts.body, &mut Vec::new(), &expected, &mut paths);

    if paths.len() != expected.len() {
        return Err("Missing verified comparison path".into());
    }
    Ok(paths)
}

fn comparison_paths(
    node: &HirNode,
    path: &mut Vec<usize>,
    expected: &std::collections::BTreeSet<Site>,
    paths: &mut BTreeMap<Site, Vec<usize>>,
) {
    if expected.contains(&site(&node.hir)) {
        paths.insert(site(&node.hir), path.to_vec());
    }
    for (index, child) in node.children.iter().enumerate() {
        path.push(index);
        comparison_paths(child, path, expected, paths);
        path.pop();
    }
}

pub(crate) fn verify_owner(
    original: &HirFnFacts,
    checked: &CheckedProjection,
) -> Result<(), String> {
    owner::verify(original, checked)
}
